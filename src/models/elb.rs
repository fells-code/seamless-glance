use crate::aws::clients::AwsClients;
use crate::models::describable::{shell_quote, DescribableResource};
use crate::models::tags::Tags;
use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct LoadBalancerInfo {
    pub arn: String,
    pub name: String,
    pub lb_type: String, // application | network
    pub scheme: String,  // internet-facing | internal
    pub state: String,
    pub az_count: usize,
    pub attached_target_groups: usize,
    pub total_targets: usize,
    pub healthy_targets: usize,
    pub tags: Tags,
}

impl LoadBalancerInfo {
    pub fn has_no_active_targets(&self) -> bool {
        self.attached_target_groups == 0 || self.total_targets == 0
    }

    pub fn has_zero_healthy_targets(&self) -> bool {
        self.total_targets > 0 && self.healthy_targets == 0
    }

    pub fn review_signals(&self) -> Vec<&'static str> {
        let mut signals = Vec::new();

        if self.attached_target_groups == 0 {
            signals.push("no-target-groups");
        } else if self.total_targets == 0 {
            signals.push("no-targets");
        }

        if self.has_zero_healthy_targets() {
            signals.push("zero-healthy");
        }

        signals
    }
}

#[async_trait]
impl DescribableResource for LoadBalancerInfo {
    fn resource_name(&self) -> String {
        self.name.clone()
    }

    async fn describe(&self, clients: &AwsClients) -> Result<String> {
        let lbs = clients
            .elb
            .describe_load_balancers()
            .load_balancer_arns(&self.arn)
            .send()
            .await?;

        let listeners = clients
            .elb
            .describe_listeners()
            .load_balancer_arn(&self.arn)
            .send()
            .await?;

        Ok(format!(
            "Load Balancer:\n{:#?}\n\nListeners:\n{:#?}",
            lbs, listeners
        ))
    }

    fn console_url(&self, region: &str) -> Option<String> {
        Some(format!(
            "https://console.aws.amazon.com/ec2/v2/home?region={region}#LoadBalancer:loadBalancerArn={}",
            self.arn
        ))
    }

    fn cli_command(&self, region: &str) -> Option<String> {
        Some(format!(
            "aws elbv2 describe-load-balancers --load-balancer-arns {} --region {}",
            shell_quote(&self.arn),
            shell_quote(region)
        ))
    }
}
