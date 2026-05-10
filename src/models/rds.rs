use async_trait::async_trait;

use crate::{
    aws::clients::AwsClients,
    models::{
        describable::{shell_quote, DescribableResource},
        service_status::ServiceStatus,
    },
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

impl RdsInstanceInfo {
    pub const PRODUCTION_NAME_HINTS: [&str; 7] = [
        "prod",
        "production",
        "live",
        "critical",
        "primary",
        "main",
        "customer",
    ];

    pub fn is_available(&self) -> bool {
        self.status == "available"
    }

    pub fn has_production_like_identifier(&self) -> bool {
        let normalized = self.identifier.to_ascii_lowercase();
        Self::PRODUCTION_NAME_HINTS
            .iter()
            .any(|hint| normalized.contains(hint))
    }

    pub fn needs_single_az_review(&self) -> bool {
        self.is_available() && !self.multi_az && self.has_production_like_identifier()
    }

    pub fn review_signals(&self) -> Vec<&'static str> {
        let mut signals = Vec::new();

        if !self.multi_az {
            signals.push("single-az");
        }

        if self.has_production_like_identifier() {
            signals.push("prod-name");
        }

        signals
    }
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

    fn cli_command(&self, region: &str) -> Option<String> {
        Some(format!(
            "aws rds describe-db-instances --db-instance-identifier {} --region {}",
            shell_quote(&self.identifier),
            shell_quote(region)
        ))
    }
}
