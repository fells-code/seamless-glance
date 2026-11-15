use chrono::{DateTime, Duration, Utc};

use crate::models::service_status::ServiceStatus;
use crate::models::tags::Tags;
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
    pub tags: Tags,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn secret(
        name: &str,
        rotation_enabled: bool,
        last_rotated: Option<String>,
        tags: Tags,
    ) -> SecretInfo {
        SecretInfo {
            name: name.into(),
            rotation_enabled,
            last_rotated,
            tags,
        }
    }

    #[test]

    fn rotation_disabled_check() {
        let now: DateTime<Utc> = Utc::now();
        let s: String = now.to_rfc3339();

        assert!(secret("prod_secret", false, Some(s.clone()), Tags::empty()).rotation_disabled());

        assert!((!secret("prod_secret", true, Some(s.clone()), Tags::empty()).rotation_disabled()));
    }

    #[test]
    fn has_production_like_name_test() {
        let now: DateTime<Utc> = Utc::now();
        let s: String = now.to_rfc3339();

        assert!(
            !secret("bad_name", false, Some(s.clone()), Tags::empty()).has_production_like_name()
        );

        assert!(
            secret("prod_secret", false, Some(s.clone()), Tags::empty()).has_production_like_name()
        );
    }

    #[test]
    fn needs_rotation_review_tests() {
        let now: DateTime<Utc> = Utc::now();
        let s: String = now.to_rfc3339();

        // returns true
        assert!(
            secret("prod_secret", false, Some(s.clone()), Tags::empty()).needs_rotation_review()
        );

        // returns false
        assert!(!secret("bad_name", false, Some(s.clone()), Tags::empty()).needs_rotation_review());
        // also false
        assert!(
            !secret("prod_secret", true, Some(s.clone()), Tags::empty()).needs_rotation_review()
        );
    }
    #[test]
    fn has_stale_rotation_tests() {
        let now: DateTime<Utc> = Utc::now();
        let s: String = now.to_rfc3339();

        // returns false because rotation_enabled is false
        assert!(!secret("prod_secret", false, Some(s), Tags::empty()).has_stale_rotation());

        // should return true for the rest
        let minus_200 = now - Duration::days(200);
        let s2: String = minus_200.to_rfc3339();
        assert!(secret("prod_secret", true, Some(s2), Tags::empty()).has_stale_rotation());
    }
}
