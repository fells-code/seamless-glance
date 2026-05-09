use async_trait::async_trait;

use crate::{
    aws::clients::AwsClients,
    models::describable::{shell_quote, DescribableResource},
};

#[derive(Debug, Clone)]
pub struct Ec2InstanceInfo {
    pub id: String,
    pub name: Option<String>,
    pub instance_type: String,
    pub state: String,
    pub region: String,
    pub az: String,
    pub private_ip: Option<String>,
    pub public_ip: Option<String>,
    pub key_name: Option<String>,
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
