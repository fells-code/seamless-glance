use chrono::{DateTime, Duration, Utc};

use anyhow::Result;
use async_trait::async_trait;

use crate::aws::clients::AwsClients;
use crate::models::describable::{shell_quote, DescribableResource};
use crate::models::service_status::ServiceStatus;

#[derive(Debug, Clone)]
pub struct LambdaSummary {
    pub function_count: u32,
    pub status: ServiceStatus,
}

#[derive(Debug, Clone)]
pub struct LambdaFunctionInfo {
    pub name: String,
    pub region: String,
    pub runtime: String,
    pub memory_mb: i32,
    pub timeout_sec: i32,
    pub last_modified: String,
}

impl LambdaFunctionInfo {
    pub const HIGH_MEMORY_THRESHOLD_MB: i32 = 2048;
    pub const STALE_DEPLOY_DAYS: i64 = 180;

    pub fn has_high_memory(&self) -> bool {
        self.memory_mb >= Self::HIGH_MEMORY_THRESHOLD_MB
    }

    pub fn is_stale(&self) -> bool {
        let Some(last_modified) = self.parsed_last_modified() else {
            return false;
        };

        last_modified <= Utc::now() - Duration::days(Self::STALE_DEPLOY_DAYS)
    }

    fn parsed_last_modified(&self) -> Option<DateTime<Utc>> {
        [
            "%Y-%m-%dT%H:%M:%S%.3f%z",
            "%Y-%m-%dT%H:%M:%S%.f%z",
            "%Y-%m-%dT%H:%M:%S%z",
        ]
        .iter()
        .find_map(|fmt| DateTime::parse_from_str(&self.last_modified, fmt).ok())
        .map(|dt| dt.with_timezone(&Utc))
    }
}

#[async_trait]
impl DescribableResource for LambdaFunctionInfo {
    fn resource_name(&self) -> String {
        self.name.clone()
    }

    fn action_region(&self) -> Option<&str> {
        Some(&self.region)
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

    fn cli_command(&self, region: &str) -> Option<String> {
        Some(format!(
            "aws lambda get-function --function-name {} --region {}",
            shell_quote(&self.name),
            shell_quote(region)
        ))
    }
}
