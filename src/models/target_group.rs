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

#[cfg(test)]
mod tests {
    use super::*;

    fn group(total: usize, unhealthy: usize, attached: bool) -> TargetGroupInfo {
        TargetGroupInfo {
            arn: "arn:aws:elasticloadbalancing:::targetgroup/tg".into(),
            name: "tg".into(),
            protocol: "HTTP".into(),
            port: 80,
            target_type: "instance".into(),
            attached_load_balancer_arns: if attached {
                vec!["arn:aws:elasticloadbalancing:::loadbalancer/app".into()]
            } else {
                Vec::new()
            },
            total_targets: total,
            unhealthy_targets: unhealthy,
            tags: Tags::empty(),
        }
    }

    #[test]
    fn healthy_targets_are_the_remainder() {
        assert_eq!(group(4, 1, true).healthy_targets(), 3);
        assert_eq!(group(4, 4, true).healthy_targets(), 0);
    }

    /// More unhealthy than registered should not underflow into a huge count.
    #[test]
    fn more_unhealthy_than_registered_cannot_underflow() {
        assert_eq!(group(2, 5, true).healthy_targets(), 0);
        assert!(group(2, 5, true).has_zero_healthy_targets());
    }

    /// An empty group is not an outage. Nothing is failing, there is just
    /// nothing registered, which the orphan rule covers instead.
    #[test]
    fn an_empty_group_is_not_zero_healthy() {
        assert!(!group(0, 0, true).has_zero_healthy_targets());
        assert!(group(1, 1, true).has_zero_healthy_targets());
    }

    #[test]
    fn an_orphan_has_neither_a_balancer_nor_targets() {
        assert!(group(0, 0, false).is_orphan_candidate());

        assert!(
            !group(0, 0, true).is_orphan_candidate(),
            "attached but empty is a deployment in progress, not an orphan"
        );
        assert!(
            !group(2, 0, false).is_orphan_candidate(),
            "unattached but serving targets is not an orphan"
        );
    }

    /// Zero-healthy and partially-unhealthy are mutually exclusive, so a group
    /// is never reported as both.
    #[test]
    fn health_signals_do_not_double_report() {
        assert_eq!(group(2, 2, true).review_signals(), vec!["zero-healthy"]);
        assert_eq!(group(4, 1, true).review_signals(), vec!["unhealthy"]);
        assert!(group(4, 0, true).review_signals().is_empty());
    }
}
