use crate::aws::clients::AwsClients;
use crate::models::describable::{shell_quote, DescribableResource};
use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct SecurityGroupInfo {
    pub id: String,
    pub name: String,
    pub vpc_id: String,
    pub inbound_rules: usize,
    pub outbound_rules: usize,
    pub open_to_world: bool,
}

#[async_trait]
impl DescribableResource for SecurityGroupInfo {
    fn resource_name(&self) -> String {
        format!("{} ({})", self.name, self.id)
    }

    async fn describe(&self, clients: &AwsClients) -> Result<String> {
        let resp = clients
            .ec2
            .describe_security_groups()
            .group_ids(&self.id)
            .send()
            .await?;

        Ok(format!("{:#?}", resp))
    }

    fn console_url(&self, region: &str) -> Option<String> {
        Some(format!(
            "https://console.aws.amazon.com/ec2/v2/home?region={region}#SecurityGroup:groupId={}",
            self.id
        ))
    }

    fn cli_command(&self, region: &str) -> Option<String> {
        Some(format!(
            "aws ec2 describe-security-groups --group-ids {} --region {}",
            shell_quote(&self.id),
            shell_quote(region)
        ))
    }
}
