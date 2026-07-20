use std::collections::HashSet;

use async_trait::async_trait;

use crate::{
    aws::clients::AwsClients,
    models::{
        describable::{shell_quote, DescribableResource},
        service_status::ServiceStatus,
        tags::Tags,
    },
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
    pub queue_url: String,
    pub is_fifo: bool,
    pub messages_available: i64,
    pub messages_in_flight: i64,
    pub has_dlq: bool,
    /// ARN of the queue this one redrives to, from its RedrivePolicy. Collecting
    /// these across a region gives the exact set of queues acting as DLQs.
    pub dead_letter_target_arn: Option<String>,
    pub tags: Tags,
}

/// Names of every queue that another queue redrives to.
pub fn dead_letter_queue_names(queues: &[SqsQueueInfo]) -> HashSet<&str> {
    queues
        .iter()
        .filter_map(|queue| queue.dead_letter_target_arn.as_deref())
        .filter_map(|arn| arn.rsplit(':').next())
        .filter(|name| !name.is_empty())
        .collect()
}

impl SqsQueueInfo {
    pub const HIGH_VISIBLE_THRESHOLD: i64 = 100;
    pub const HIGH_IN_FLIGHT_THRESHOLD: i64 = 50;

    pub fn has_high_visible_messages(&self) -> bool {
        self.messages_available >= Self::HIGH_VISIBLE_THRESHOLD
    }

    pub fn has_high_in_flight_messages(&self) -> bool {
        self.messages_in_flight >= Self::HIGH_IN_FLIGHT_THRESHOLD
    }

    pub fn has_backlog_incident(&self) -> bool {
        self.has_high_visible_messages() || self.has_high_in_flight_messages()
    }

    pub fn backlog_signals(&self) -> Vec<&'static str> {
        let mut signals = Vec::new();

        if self.has_high_visible_messages() {
            signals.push("visible");
        }

        if self.has_high_in_flight_messages() {
            signals.push("in-flight");
        }

        signals
    }
}

#[async_trait]
impl DescribableResource for SqsQueueInfo {
    fn resource_name(&self) -> String {
        self.name.clone()
    }

    async fn describe(&self, clients: &AwsClients) -> anyhow::Result<String> {
        let resp = clients
            .sqs
            .get_queue_attributes()
            .queue_url(&self.queue_url)
            .attribute_names(aws_sdk_sqs::types::QueueAttributeName::All)
            .send()
            .await?;

        Ok(format!("{:#?}", resp))
    }

    fn console_url(&self, region: &str) -> Option<String> {
        Some(format!(
            "https://{}.console.aws.amazon.com/sqs/v3/home?region={}#/queues/{}",
            region, region, self.name
        ))
    }

    fn cli_command(&self, region: &str) -> Option<String> {
        Some(format!(
            "aws sqs get-queue-attributes --queue-url {} --attribute-names All --region {}",
            shell_quote(&self.queue_url),
            shell_quote(region)
        ))
    }
}
