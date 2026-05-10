use async_trait::async_trait;

use crate::{
    aws::clients::AwsClients,
    models::describable::{shell_quote, DescribableResource},
};

#[derive(Debug, Clone)]
pub struct Ec2InstanceInfo {
    pub id: String,
    pub name: Option<String>,
    pub owner: Option<String>,
    pub environment: Option<String>,
    pub instance_type: String,
    pub state: String,
    pub region: String,
    pub az: String,
    pub private_ip: Option<String>,
    pub public_ip: Option<String>,
    pub key_name: Option<String>,
}

impl Ec2InstanceInfo {
    pub const PRODUCTION_NAME_HINTS: [&str; 7] = [
        "prod",
        "production",
        "live",
        "critical",
        "primary",
        "main",
        "customer",
    ];

    pub fn is_stopped(&self) -> bool {
        self.state == "stopped"
    }

    pub fn has_public_ip(&self) -> bool {
        self.public_ip.is_some()
    }

    pub fn has_production_like_name(&self) -> bool {
        let Some(name) = &self.name else {
            return false;
        };

        let normalized = name.to_ascii_lowercase();
        Self::PRODUCTION_NAME_HINTS
            .iter()
            .any(|hint| normalized.contains(hint))
    }

    pub fn needs_stopped_review(&self) -> bool {
        self.is_stopped() && (self.has_public_ip() || self.has_production_like_name())
    }

    pub fn missing_required_tags(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();

        if self
            .name
            .as_deref()
            .map(|value| value.trim().is_empty())
            .unwrap_or(true)
        {
            missing.push("Name");
        }

        if self
            .owner
            .as_deref()
            .map(|value| value.trim().is_empty())
            .unwrap_or(true)
        {
            missing.push("Owner");
        }

        if self
            .environment
            .as_deref()
            .map(|value| value.trim().is_empty())
            .unwrap_or(true)
        {
            missing.push("Environment");
        }

        missing
    }

    pub fn has_tag_coverage_gap(&self) -> bool {
        !self.missing_required_tags().is_empty()
    }

    pub fn review_signals(&self) -> Vec<&'static str> {
        let mut signals = Vec::new();

        if self.has_public_ip() {
            signals.push("public-ip");
        }

        if self.has_production_like_name() {
            signals.push("prod-name");
        }

        if self.has_tag_coverage_gap() {
            signals.push("missing-tags");
        }

        signals
    }
}

#[async_trait]
impl DescribableResource for Ec2InstanceInfo {
    fn resource_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| self.id.clone())
    }

    fn action_region(&self) -> Option<&str> {
        Some(&self.region)
    }

    async fn describe(&self, clients: &AwsClients) -> anyhow::Result<String> {
        let resp = clients
            .ec2
            .describe_instances()
            .instance_ids(&self.id)
            .send()
            .await?;

        Ok(format!("{:#?}", resp))
    }

    fn console_url(&self, region: &str) -> Option<String> {
        Some(format!(
            "https://console.aws.amazon.com/ec2/v2/home?region={region}#InstanceDetails:instanceId={}",
            self.id
        ))
    }

    fn cli_command(&self, region: &str) -> Option<String> {
        Some(format!(
            "aws ec2 describe-instances --instance-ids {} --region {}",
            shell_quote(&self.id),
            shell_quote(region)
        ))
    }
}
