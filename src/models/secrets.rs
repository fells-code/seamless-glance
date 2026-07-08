use chrono::{DateTime, Duration, Utc};

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

impl SecretInfo {
    pub const STALE_ROTATION_DAYS: i64 = 180;
    pub const PRODUCTION_NAME_HINTS: [&str; 7] = [
        "prod",
        "production",
        "live",
        "critical",
        "primary",
        "main",
        "customer",
    ];

    pub fn rotation_disabled(&self) -> bool {
        !self.rotation_enabled
    }

    pub fn has_production_like_name(&self) -> bool {
        let normalized = self.name.to_ascii_lowercase();
        Self::PRODUCTION_NAME_HINTS
            .iter()
            .any(|hint| normalized.contains(hint))
    }

    pub fn needs_rotation_review(&self) -> bool {
        self.rotation_disabled() && self.has_production_like_name()
    }

    pub fn has_stale_rotation(&self) -> bool {
        if !self.rotation_enabled {
            return false;
        }

        let Some(last_rotated) = self.parsed_last_rotated() else {
            return false;
        };

        last_rotated <= Utc::now() - Duration::days(Self::STALE_ROTATION_DAYS)
    }

    pub fn review_signals(&self) -> Vec<&'static str> {
        let mut signals = Vec::new();

        if self.rotation_disabled() {
            signals.push("no-rotation");
        }

        if self.has_production_like_name() {
            signals.push("prod-name");
        }

        if self.has_stale_rotation() {
            signals.push("stale-rotation");
        }

        signals
    }

    fn parsed_last_rotated(&self) -> Option<DateTime<Utc>> {
        self.last_rotated
            .as_deref()
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|dt| dt.with_timezone(&Utc))
    }
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
