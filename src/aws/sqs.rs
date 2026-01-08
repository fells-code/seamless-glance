use crate::{
    app::App,
    models::{
        service_status::ServiceStatus,
        sqs::{SqsQueueInfo, SqsSummary},
    },
};
use aws_sdk_sqs::types::QueueAttributeName;

pub async fn fetch_sqs_summary(app: &App) -> SqsSummary {
    let resp = match app.aws.sqs.list_queues().send().await {
        Ok(r) => r,
        Err(err) => {
            let msg = err.to_string();
            return if msg.contains("AccessDenied") {
                SqsSummary {
                    queue_count: 0,
                    dlq_count: 0,
                    status: ServiceStatus::AccessDenied,
                }
            } else {
                SqsSummary {
                    queue_count: 0,
                    dlq_count: 0,
                    status: ServiceStatus::Unavailable(msg),
                }
            };
        }
    };

    let urls = resp.queue_urls();
    let queue_count = urls.len() as u32;

    let mut dlq_count = 0;

    for url in urls {
        let attrs = match app
            .aws
            .sqs
            .get_queue_attributes()
            .queue_url(url)
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

pub async fn fetch_sqs_queues(app: &App) -> Vec<SqsQueueInfo> {
    let resp = match app.aws.sqs.list_queues().send().await {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let mut queues = vec![];

    for url in resp.queue_urls() {
        let attrs = match app
            .aws
            .sqs
            .get_queue_attributes()
            .queue_url(url)
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
            is_fifo,
            messages_available,
            messages_in_flight,
            has_dlq,
        });
    }

    queues
}
