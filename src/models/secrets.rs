use crate::models::service_status::ServiceStatus;
use crate::{
    aws::clients::AwsClients,
    models::describable::{shell_quote, DescribableResource},
};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct SecretsSummary {
    pub status: ServiceStatus,
    pub total: usize,
    pub rotation_disabled: usize,
}

#[derive(Debug, Clone)]
pub struct SecretInfo {
    pub name: String,
    pub rotation_enabled: bool,
    pub last_rotated: Option<String>,
}

#[async_trait]
impl DescribableResource for SecretInfo {
    fn resource_name(&self) -> String {
        self.name.clone()
    }

    async fn describe(&self, clients: &AwsClients) -> anyhow::Result<String> {
        let resp = clients
            .sm
            .describe_secret()
            .secret_id(&self.name)
            .send()
            .await?;

        Ok(format!("{:#?}", resp))
    }

    fn console_url(&self, region: &str) -> Option<String> {
        Some(format!(
            "https://console.aws.amazon.com/secretsmanager/secret?name={}&region={}",
            self.name, region
        ))
    }

    fn cli_command(&self, region: &str) -> Option<String> {
        Some(format!(
            "aws secretsmanager describe-secret --secret-id {} --region {}",
            shell_quote(&self.name),
            shell_quote(region)
        ))
    }
}
