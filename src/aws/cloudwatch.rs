use crate::app::App;
use crate::aws::tags;
use crate::models::cloudwatch::{CloudWatchAlarm, CloudWatchSummary};
use crate::models::service_status::ServiceStatus;
use crate::models::tags::Tags;

pub async fn fetch_cloudwatch(app: &App) -> (CloudWatchSummary, Vec<CloudWatchAlarm>) {
    let mut pages = app.aws.cw.describe_alarms().into_paginator().send();

    let mut alarms: Vec<CloudWatchAlarm> = Vec::new();
    let mut arns: Vec<Option<String>> = Vec::new();

    while let Some(page) = pages.next().await {
        let page = match page {
            Ok(page) => page,
            Err(err) => {
                return (
                    CloudWatchSummary {
                        status: ServiceStatus::from_sdk_error(&err),
                        total_alarms: 0,
                        alarms_in_alarm: 0,
                    },
                    vec![],
                );
            }
        };

        for a in page.metric_alarms() {
            arns.push(a.alarm_arn().map(|arn| arn.to_string()));

            alarms.push(CloudWatchAlarm {
                name: a.alarm_name().unwrap_or("").to_string(),
                state: a
                    .state_value()
                    .map(|s| s.as_str())
                    .unwrap_or("UNKNOWN")
                    .to_string(),
                namespace: a.namespace().unwrap_or("").to_string(),
                metric: a.metric_name().unwrap_or("").to_string(),
                tags: Tags::Unavailable,
            });
        }
    }

    // DescribeAlarms does not return tags, so they need one ListTagsForResource
    // per alarm. Done before the sort so the lookups stay aligned by index.
    let looked_up = tags::lookup_each(arns, |arn| async move {
        let resp = app
            .aws
            .cw
            .list_tags_for_resource()
            .resource_arn(arn?)
            .send()
            .await
            .ok()?;

        Some(tags::from_pairs(
            resp.tags().iter().map(|t| (t.key(), t.value())),
        ))
    })
    .await;

    for (alarm, tags) in alarms.iter_mut().zip(looked_up) {
        alarm.tags = tags;
    }

    alarms.sort_by(|a, b| {
        let a_rank = if a.state == "ALARM" { 0 } else { 1 };
        let b_rank = if b.state == "ALARM" { 0 } else { 1 };

        a_rank
            .cmp(&b_rank)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.namespace.cmp(&b.namespace))
    });

    let total = alarms.len();
    let alarming = alarms.iter().filter(|a| a.state == "ALARM").count();

    (
        CloudWatchSummary {
            status: ServiceStatus::Ok,
            total_alarms: total,
            alarms_in_alarm: alarming,
        },
        alarms,
    )
}
