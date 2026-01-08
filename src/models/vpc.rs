use async_trait::async_trait;

use crate::{
    aws::clients::AwsClients,
    models::{describable::DescribableResource, service_status::ServiceStatus},
};

#[derive(Debug, Clone)]
pub struct VpcSummary {
    pub vpc_count: u32,
    pub subnet_count: u32,
    pub status: ServiceStatus,
}

#[derive(Debug, Clone)]
pub struct VpcInfo {
    pub vpc_id: String,
    pub cidr: String,
    pub state: String,
    pub is_default: bool,
    pub subnet_count: u32,
}

#[async_trait]
impl DescribableResource for VpcInfo {
    fn resource_name(&self) -> String {
        self.vpc_id.clone()
    }

    async fn describe(&self, clients: &AwsClients) -> anyhow::Result<String> {
        let resp = clients.sm.list_secrets().send().await?;

        Ok(format!("{:#?}", resp))
    }

    fn console_url(&self, region: &str) -> Option<String> {
        Some(format!(
            "https://console.aws.amazon.com/vpcconsole/home?region={}#VpcDetails:VpcId={}",
            region, self.vpc_id
        ))
    }
}
