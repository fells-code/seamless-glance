use crate::{
    aws::clients::AwsClients,
    models::{
        describable::{shell_quote, DescribableResource},
        service_status::ServiceStatus,
        tags::Tags,
    },
};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct CloudWatchSummary {
    pub status: ServiceStatus,
    pub total_alarms: usize,
    pub alarms_in_alarm: usize,
}

#[derive(Debug, Clone)]
pub struct CloudWatchAlarm {
    pub name: String,
    pub state: String,
    pub namespace: String,
    pub metric: String,
    pub tags: Tags,
}

#[async_trait]
impl DescribableResource for CloudWatchAlarm {
    fn resource_name(&self) -> String {
        self.name.clone()
    }

    async fn describe(&self, clients: &AwsClients) -> anyhow::Result<String> {
        let resp = clients
            .cw
            .describe_alarms()
            .alarm_names(&self.name)
            .send()
            .await?;

        Ok(format!("{:#?}", resp))
    }

    fn console_url(&self, region: &str) -> Option<String> {
        Some(format!(
            "https://console.aws.amazon.com/cloudwatch/home?region={region}#alarmsV2:alarm/{}",
            self.name
        ))
    }

    fn cli_command(&self, region: &str) -> Option<String> {
        Some(format!(
            "aws cloudwatch describe-alarms --alarm-names {} --region {}",
            shell_quote(&self.name),
            shell_quote(region)
        ))
    }
}
