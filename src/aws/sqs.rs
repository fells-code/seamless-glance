use crate::{
    app::App,
    models::{
        service_status::ServiceStatus,
        sqs::{SqsQueueInfo, SqsSummary},
    },
};
use aws_sdk_sqs::types::QueueAttributeName;

pub async fn fetch_sqs_summary(app: &App) -> SqsSummary {
    let mut pages = app.aws.sqs.list_queues().into_paginator().items().send();

    let mut queue_count = 0u32;
    let mut dlq_count = 0;

    while let Some(item) = pages.next().await {
        let url = match item {
            Ok(url) => url,
            Err(err) => {
                return SqsSummary {
                    queue_count: 0,
                    dlq_count: 0,
                    status: ServiceStatus::from_sdk_error(&err),
                };
            }
        };

        queue_count += 1;

        let attrs = match app
            .aws
            .sqs
            .get_queue_attributes()
            .queue_url(&url)
            .attribute_names(QueueAttributeName::RedrivePolicy)
            .send()
            .await
        {
            Ok(a) => a,
            Err(_) => continue,
        };

        if attrs
            .attributes()
            .map(|m| m.contains_key(&QueueAttributeName::RedrivePolicy))
            .unwrap_or(false)
        {
            dlq_count += 1;
        }
    }

    SqsSummary {
        queue_count,
        dlq_count,
        status: ServiceStatus::Ok,
    }
}

pub async fn fetch_sqs_queues(app: &App) -> (Vec<SqsQueueInfo>, ServiceStatus) {
    let mut pages = app.aws.sqs.list_queues().into_paginator().items().send();

    let mut queues = vec![];

    while let Some(item) = pages.next().await {
        let url = match item {
            Ok(url) => url,
            Err(err) => return (vec![], ServiceStatus::from_sdk_error(&err)),
        };

        let attrs = match app
            .aws
            .sqs
            .get_queue_attributes()
            .queue_url(&url)
            .attribute_names(QueueAttributeName::ApproximateNumberOfMessages)
            .attribute_names(QueueAttributeName::ApproximateNumberOfMessagesNotVisible)
            .attribute_names(QueueAttributeName::RedrivePolicy)
            .send()
            .await
        {
            Ok(a) => a,
            Err(_) => continue,
        };

        let name = url.rsplit('/').next().unwrap_or("unknown").to_string();

        let is_fifo = name.ends_with(".fifo");

        let map = match attrs.attributes() {
            Some(m) => m,
            None => continue,
        };

        let messages_available = map
            .get(&QueueAttributeName::ApproximateNumberOfMessages)
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0);

        let messages_in_flight = map
            .get(&QueueAttributeName::ApproximateNumberOfMessagesNotVisible)
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0);

        let has_dlq = map.contains_key(&QueueAttributeName::RedrivePolicy);

        queues.push(SqsQueueInfo {
            name,
            queue_url: url.to_string(),
            is_fifo,
            messages_available,
            messages_in_flight,
            has_dlq,
        });
    }

    (queues, ServiceStatus::Ok)
}
