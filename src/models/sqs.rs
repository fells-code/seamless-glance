use async_trait::async_trait;

use crate::{
    aws::clients::AwsClients,
    models::{describable::DescribableResource, service_status::ServiceStatus},
};

#[derive(Debug, Clone)]
pub struct SqsSummary {
    pub queue_count: u32,
    pub dlq_count: u32,
    pub status: ServiceStatus,
}

#[derive(Debug, Clone)]
pub struct SqsQueueInfo {
    pub name: String,
    pub is_fifo: bool,
    pub messages_available: i64,
    pub messages_in_flight: i64,
    pub has_dlq: bool,
}

#[async_trait]
impl DescribableResource for SqsQueueInfo {
    fn resource_name(&self) -> String {
        self.name.clone()
    }

    async fn describe(&self, clients: &AwsClients) -> anyhow::Result<String> {
        let resp = clients.sqs.list_queues().send().await?;

        Ok(format!("{:#?}", resp))
    }

    fn console_url(&self, region: &str) -> Option<String> {
        Some(format!(
            "https://{}.console.aws.amazon.com/sqs/v3/home?region={}#/queues/{}",
            region, region, self.name
        ))
    }
}
