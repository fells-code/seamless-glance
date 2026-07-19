use crate::{
    app::App,
    aws::DESCRIBE_CONCURRENCY,
    models::{
        service_status::ServiceStatus,
        sqs::{SqsQueueInfo, SqsSummary},
    },
};
use aws_sdk_sqs::types::QueueAttributeName;
use futures::StreamExt;

/// Page through every queue URL. Returns the list, or the classified status when
/// the list call itself fails.
async fn list_queue_urls(app: &App) -> Result<Vec<String>, ServiceStatus> {
    let mut pages = app.aws.sqs.list_queues().into_paginator().items().send();
    let mut urls = Vec::new();

    while let Some(item) = pages.next().await {
        match item {
            Ok(url) => urls.push(url),
            Err(err) => return Err(ServiceStatus::from_sdk_error(&err)),
        }
    }

    Ok(urls)
}

pub async fn fetch_sqs_summary(app: &App) -> SqsSummary {
    let urls = match list_queue_urls(app).await {
        Ok(urls) => urls,
        Err(status) => {
            return SqsSummary {
                queue_count: 0,
                dlq_count: 0,
                status,
            };
        }
    };

    let queue_count = urls.len() as u32;

    // Attributes are a separate call per queue, so run them with bounded
    // concurrency instead of one round trip at a time.
    let dlq_count = futures::stream::iter(urls)
        .map(|url| async move {
            let Ok(attrs) = app
                .aws
                .sqs
                .get_queue_attributes()
                .queue_url(&url)
                .attribute_names(QueueAttributeName::RedrivePolicy)
                .send()
                .await
            else {
                return false;
            };

            attrs
                .attributes()
                .map(|m| m.contains_key(&QueueAttributeName::RedrivePolicy))
                .unwrap_or(false)
        })
        .buffered(DESCRIBE_CONCURRENCY)
        .filter(|has_dlq| {
            let has_dlq = *has_dlq;
            async move { has_dlq }
        })
        .count()
        .await as u32;

    SqsSummary {
        queue_count,
        dlq_count,
        status: ServiceStatus::Ok,
    }
}

pub async fn fetch_sqs_queues(app: &App) -> (Vec<SqsQueueInfo>, ServiceStatus) {
    let urls = match list_queue_urls(app).await {
        Ok(urls) => urls,
        Err(status) => return (vec![], status),
    };

    // Attributes are a separate call per queue, so run them with bounded
    // concurrency instead of one round trip at a time.
    let queues = futures::stream::iter(urls)
        .map(|url| async move {
            let attrs = app
                .aws
                .sqs
                .get_queue_attributes()
                .queue_url(&url)
                .attribute_names(QueueAttributeName::ApproximateNumberOfMessages)
                .attribute_names(QueueAttributeName::ApproximateNumberOfMessagesNotVisible)
                .attribute_names(QueueAttributeName::RedrivePolicy)
                .send()
                .await
                .ok()?;

            let map = attrs.attributes()?;

            let name = url.rsplit('/').next().unwrap_or("unknown").to_string();
            let is_fifo = name.ends_with(".fifo");

            let messages_available = map
                .get(&QueueAttributeName::ApproximateNumberOfMessages)
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(0);

            let messages_in_flight = map
                .get(&QueueAttributeName::ApproximateNumberOfMessagesNotVisible)
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(0);

            let has_dlq = map.contains_key(&QueueAttributeName::RedrivePolicy);

            Some(SqsQueueInfo {
                name,
                queue_url: url.clone(),
                is_fifo,
                messages_available,
                messages_in_flight,
                has_dlq,
            })
        })
        .buffered(DESCRIBE_CONCURRENCY)
        .filter_map(|queue| async move { queue })
        .collect()
        .await;

    (queues, ServiceStatus::Ok)
}
