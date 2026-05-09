use crate::aws::clients::AwsClients;
use crate::models::describable::DescribableResource;
use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct TargetGroupInfo {
    pub arn: String,
    pub name: String,
    pub protocol: String,
    pub port: i32,
    pub target_type: String,
    pub total_targets: usize,
    pub unhealthy_targets: usize,
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
}
