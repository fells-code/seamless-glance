use async_trait::async_trait;

use crate::{
    aws::clients::AwsClients,
    models::{describable::DescribableResource, service_status::ServiceStatus},
};

#[derive(Debug, Clone)]
pub struct RdsSummary {
    pub status: ServiceStatus,
    pub total: usize,
    pub available: usize,
}

#[derive(Debug, Clone)]
pub struct RdsInstanceInfo {
    pub identifier: String,
    pub region: String,
    pub engine: String,
    pub instance_class: String,
    pub status: String,
    pub az: String,
    pub multi_az: bool,
}

#[async_trait]
impl DescribableResource for RdsInstanceInfo {
    fn resource_name(&self) -> String {
        self.identifier.clone()
    }

    fn action_region(&self) -> Option<&str> {
        Some(&self.region)
    }

    async fn describe(&self, clients: &AwsClients) -> anyhow::Result<String> {
        let resp = clients
            .rds
            .describe_db_instances()
            .db_instance_identifier(&self.identifier)
            .send()
            .await?;

        Ok(format!("{:#?}", resp))
    }

    fn console_url(&self, region: &str) -> Option<String> {
        Some(format!(
            "https://{}.console.aws.amazon.com/rds/home?region={}#database:id={}",
            region, region, self.identifier
        ))
    }
}
