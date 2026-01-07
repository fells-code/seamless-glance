use anyhow::Result;
use async_trait::async_trait;

use crate::aws::clients::AwsClients;
use crate::models::describable::DescribableResource;
use crate::models::service_status::ServiceStatus;

#[derive(Debug, Clone)]
pub struct LambdaSummary {
    pub function_count: u32,
    pub status: ServiceStatus,
}

#[derive(Debug, Clone)]
pub struct LambdaFunctionInfo {
    pub name: String,
    pub runtime: String,
    pub memory_mb: i32,
    pub timeout_sec: i32,
    pub last_modified: String,
}

#[async_trait]
impl DescribableResource for LambdaFunctionInfo {
    fn resource_name(&self) -> String {
        self.name.clone()
    }

    async fn describe(&self, clients: &AwsClients) -> Result<String> {
        let resp = clients
            .lambda
            .get_function()
            .function_name(&self.name)
            .send()
            .await?;

        Ok(format!("{:#?}", resp))
    }

    fn console_url(&self, region: &str) -> Option<String> {
        Some(format!(
            "https://console.aws.amazon.com/lambda/home?region={region}#/functions/{}",
            self.name
        ))
    }
}
