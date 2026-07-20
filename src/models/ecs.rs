use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    aws::clients::AwsClients,
    models::describable::{shell_quote, DescribableResource},
    models::tags::Tags,
};

/// Compute capacity registered with a cluster by the container instances
/// backing it.
///
/// Only EC2-backed clusters have one. Fargate provisions capacity per task with
/// no cluster-level pool, so there is no total to report and those clusters
/// carry `None` rather than a zero that would read as an empty cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterCapacity {
    pub registered_cpu_units: i32,
    pub available_cpu_units: i32,
    pub registered_memory_mib: i32,
    pub available_memory_mib: i32,
}

impl ClusterCapacity {
    /// Share of registered capacity currently claimed by tasks, as a percent.
    ///
    /// `None` when nothing is registered, which would otherwise divide by zero.
    fn used_percent(registered: i32, available: i32) -> Option<u32> {
        if registered <= 0 {
            return None;
        }

        let used = registered.saturating_sub(available).max(0);

        Some((used as i64 * 100 / registered as i64) as u32)
    }

    pub fn cpu_used_percent(&self) -> Option<u32> {
        Self::used_percent(self.registered_cpu_units, self.available_cpu_units)
    }

    pub fn memory_used_percent(&self) -> Option<u32> {
        Self::used_percent(self.registered_memory_mib, self.available_memory_mib)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcsClusterInfo {
    pub arn: String,
    pub name: String,
    pub running_tasks: i32,
    pub pending_tasks: i32,
    pub active_services: i32,
    pub registered_container_instances: i32,
    /// Cluster lifecycle status as ECS reports it: ACTIVE, PROVISIONING,
    /// DEPROVISIONING, FAILED, or INACTIVE.
    #[serde(default)]
    pub status: String,
    /// `None` for Fargate-only clusters, which have no capacity pool.
    #[serde(default)]
    pub capacity: Option<ClusterCapacity>,
    pub tags: Tags,
}

impl EcsClusterInfo {
    /// How a capacity share renders in a column, or `-` when the cluster has no
    /// capacity pool to measure against.
    fn capacity_label(percent: Option<u32>) -> String {
        percent.map_or_else(|| "-".to_string(), |value| format!("{value}%"))
    }

    pub fn cpu_label(&self) -> String {
        Self::capacity_label(self.capacity.and_then(|c| c.cpu_used_percent()))
    }

    pub fn memory_label(&self) -> String {
        Self::capacity_label(self.capacity.and_then(|c| c.memory_used_percent()))
    }

    pub fn status_label(&self) -> String {
        if self.status.is_empty() {
            "-".to_string()
        } else {
            self.status.clone()
        }
    }

    /// Whether the cluster is in its normal serving state.
    pub fn is_active(&self) -> bool {
        self.status.eq_ignore_ascii_case("ACTIVE")
    }
}

#[async_trait]
impl DescribableResource for EcsClusterInfo {
    fn resource_name(&self) -> String {
        self.name.clone()
    }

    async fn describe(&self, clients: &AwsClients) -> anyhow::Result<String> {
        let resp = clients
            .ecs
            .describe_clusters()
            .clusters(&self.arn)
            .include(aws_sdk_ecs::types::ClusterField::Tags)
            .send()
            .await?;

        Ok(format!("{:#?}", resp))
    }

    fn console_url(&self, region: &str) -> Option<String> {
        Some(format!(
            "https://console.aws.amazon.com/ecs/v2/clusters/{}/services?region={}",
            self.name, region
        ))
    }

    fn cli_command(&self, region: &str) -> Option<String> {
        Some(format!(
            "aws ecs describe-clusters --clusters {} --include TAGS --region {}",
            shell_quote(&self.arn),
            shell_quote(region)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cluster(capacity: Option<ClusterCapacity>) -> EcsClusterInfo {
        EcsClusterInfo {
            arn: "arn:aws:ecs:us-east-1:1:cluster/core".into(),
            name: "core".into(),
            running_tasks: 4,
            pending_tasks: 0,
            active_services: 4,
            registered_container_instances: 0,
            status: "ACTIVE".into(),
            capacity,
            tags: Tags::empty(),
        }
    }

    fn capacity(reg_cpu: i32, avail_cpu: i32, reg_mem: i32, avail_mem: i32) -> ClusterCapacity {
        ClusterCapacity {
            registered_cpu_units: reg_cpu,
            available_cpu_units: avail_cpu,
            registered_memory_mib: reg_mem,
            available_memory_mib: avail_mem,
        }
    }

    #[test]
    fn utilization_is_the_share_of_registered_capacity_in_use() {
        let half_cpu = capacity(4096, 2048, 8192, 6144);

        assert_eq!(half_cpu.cpu_used_percent(), Some(50));
        assert_eq!(half_cpu.memory_used_percent(), Some(25));
    }

    #[test]
    fn a_fully_free_cluster_reads_as_zero_not_absent() {
        let idle = capacity(4096, 4096, 8192, 8192);

        assert_eq!(idle.cpu_used_percent(), Some(0));
        assert_eq!(cluster(Some(idle)).cpu_label(), "0%");
    }

    #[test]
    fn a_fully_claimed_cluster_reads_as_one_hundred() {
        let full = capacity(4096, 0, 8192, 0);

        assert_eq!(full.cpu_used_percent(), Some(100));
        assert_eq!(full.memory_used_percent(), Some(100));
    }

    /// Fargate registers no instances, so there is no pool to measure against.
    /// Reporting 0% would read as an idle cluster rather than an inapplicable
    /// measurement, which is what the placeholder used to do.
    #[test]
    fn a_fargate_cluster_reports_no_utilization() {
        let fargate = cluster(None);

        assert_eq!(fargate.cpu_label(), "-");
        assert_eq!(fargate.memory_label(), "-");
    }

    #[test]
    fn nothing_registered_cannot_divide_by_zero() {
        let empty = capacity(0, 0, 0, 0);

        assert_eq!(empty.cpu_used_percent(), None);
        assert_eq!(empty.memory_used_percent(), None);
        assert_eq!(cluster(Some(empty)).cpu_label(), "-");
    }

    /// ECS can report more available than registered while an instance drains.
    /// That is not negative usage.
    #[test]
    fn more_available_than_registered_is_clamped_to_zero_used() {
        let draining = capacity(4096, 5000, 8192, 9000);

        assert_eq!(draining.cpu_used_percent(), Some(0));
        assert_eq!(draining.memory_used_percent(), Some(0));
    }

    /// Clusters cached before this field existed still load.
    #[test]
    fn a_cached_cluster_without_capacity_still_deserializes() {
        let older = r#"{
            "arn": "arn:aws:ecs:us-east-1:1:cluster/core",
            "name": "core",
            "running_tasks": 4,
            "pending_tasks": 0,
            "active_services": 4,
            "registered_container_instances": 0,
            "tags": "Unavailable"
        }"#;

        let restored: EcsClusterInfo = serde_json::from_str(older).expect("older shape loads");

        assert_eq!(restored.capacity, None);
        assert_eq!(restored.cpu_label(), "-");
        // Status was not stored either, so it reads as unknown rather than OK.
        assert_eq!(restored.status_label(), "-");
    }

    #[test]
    fn a_cluster_reports_the_status_ecs_gave_it() {
        let mut provisioning = cluster(None);
        provisioning.status = "PROVISIONING".into();

        assert_eq!(provisioning.status_label(), "PROVISIONING");
        assert!(!provisioning.is_active());
        assert!(cluster(None).is_active());
    }

    /// Never invent a status. An empty one means it was not reported.
    #[test]
    fn an_unreported_status_is_not_claimed_to_be_healthy() {
        let mut unknown = cluster(None);
        unknown.status = String::new();

        assert_eq!(unknown.status_label(), "-");
        assert!(!unknown.is_active());
    }
}
