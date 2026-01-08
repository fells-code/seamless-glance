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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcsServiceInfo {
    pub arn: String,
    pub name: String,
    pub desired_count: i32,
    pub running_count: i32,
    pub pending_count: i32,
    pub deployments: Vec<EcsDeploymentInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcsDeploymentInfo {
    pub id: String,
    pub status: String,
    pub rollout_state: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcsTaskInfo {
    pub arn: String,
    pub task_definition: String,
    pub last_status: String,
    pub desired_status: String,
    pub cpu: Option<String>,
    pub memory: Option<String>,
    pub container_instance_arn: Option<String>,
}

#[async_trait]
impl DescribableResource for EcsClusterInfo {
    fn resource_name(&self) -> String {
        self.name.clone()
    }

    async fn describe(&self, clients: &AwsClients) -> anyhow::Result<String> {
        let resp = clients.ecs.list_clusters().send().await?;

        Ok(format!("{:#?}", resp))
    }

    fn console_url(&self, region: &str) -> Option<String> {
        Some(format!(
            "https://console.aws.amazon.com/ecs/v2/clusters/{}/services?region={}",
            self.name, region
        ))
    }
}
