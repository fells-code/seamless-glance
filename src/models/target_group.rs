use crate::aws::clients::AwsClients;
use crate::models::describable::{shell_quote, DescribableResource};
use crate::models::tags::Tags;
use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct TargetGroupInfo {
    pub arn: String,
    pub name: String,
    pub protocol: String,
    pub port: i32,
    pub target_type: String,
    pub attached_load_balancer_arns: Vec<String>,
    pub total_targets: usize,
    pub unhealthy_targets: usize,
    pub tags: Tags,
}

impl TargetGroupInfo {
    pub fn healthy_targets(&self) -> usize {
        self.total_targets.saturating_sub(self.unhealthy_targets)
    }

    pub fn has_zero_healthy_targets(&self) -> bool {
        self.total_targets > 0 && self.healthy_targets() == 0
    }

    pub fn attached_load_balancer_count(&self) -> usize {
        self.attached_load_balancer_arns.len()
    }

    pub fn has_load_balancer_attachment(&self) -> bool {
        !self.attached_load_balancer_arns.is_empty()
    }

    pub fn is_orphan_candidate(&self) -> bool {
        !self.has_load_balancer_attachment() && self.total_targets == 0
    }

    pub fn review_signals(&self) -> Vec<&'static str> {
        let mut signals = Vec::new();

        if self.is_orphan_candidate() {
            signals.push("orphan");
        }

        if self.has_zero_healthy_targets() {
            signals.push("zero-healthy");
        } else if self.unhealthy_targets > 0 {
            signals.push("unhealthy");
        }

        signals
    }
}

#[async_trait]
impl DescribableResource for TargetGroupInfo {
    fn resource_name(&self) -> String {
        self.name.clone()
    }

    async fn describe(&self, clients: &AwsClients) -> Result<String> {
        let tg = clients
            .elb
            .describe_target_groups()
            .target_group_arns(&self.arn)
            .send()
            .await?;

        let health = clients
            .elb
            .describe_target_health()
            .target_group_arn(&self.arn)
            .send()
            .await?;

        Ok(format!(
            "Target Group:\n{:#?}\n\nTarget Health:\n{:#?}",
            tg, health
        ))
    }

    fn console_url(&self, region: &str) -> Option<String> {
        Some(format!(
            "https://console.aws.amazon.com/ec2/v2/home?region={region}#TargetGroup:targetGroupArn={}",
            self.arn
        ))
    }

    fn cli_command(&self, region: &str) -> Option<String> {
        Some(format!(
            "aws elbv2 describe-target-health --target-group-arn {} --region {}",
            shell_quote(&self.arn),
            shell_quote(region)
        ))
    }
}
