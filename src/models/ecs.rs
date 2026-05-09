use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{aws::clients::AwsClients, models::describable::DescribableResource};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcsClusterInfo {
    pub arn: String,
    pub name: String,
    pub running_tasks: i32,
    pub pending_tasks: i32,
    pub active_services: i32,
    pub registered_container_instances: i32,
    pub cpu: i32,
    pub memory: i32,
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
}
