use crate::aws::clients::AwsClients;
use anyhow::Result;

#[async_trait::async_trait]
pub trait DescribableResource {
    fn resource_name(&self) -> String;

    fn action_region(&self) -> Option<&str> {
        None
    }

    async fn describe(&self, clients: &AwsClients) -> Result<String>;

    fn console_url(&self, region: &str) -> Option<String>;
}
