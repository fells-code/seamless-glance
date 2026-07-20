use crate::{
    app::App,
    aws::tags,
    aws::DESCRIBE_CONCURRENCY,
    models::{service_status::ServiceStatus, tags::Tags, target_group::TargetGroupInfo},
};
use futures::StreamExt;

pub async fn fetch_target_groups(app: &App) -> (Vec<TargetGroupInfo>, ServiceStatus) {
    let mut pages = app
        .aws
        .elb
        .describe_target_groups()
        .into_paginator()
        .items()
        .send();

    let mut target_groups = Vec::new();

    while let Some(item) = pages.next().await {
        match item {
            Ok(tg) => target_groups.push(tg),
            Err(err) => return (vec![], ServiceStatus::from_sdk_error(&err)),
        }
    }

    // Health is a separate describe per target group, so run them with bounded
    // concurrency instead of one round trip at a time.
    let groups = futures::stream::iter(target_groups)
        .map(|tg| async move {
            let arn = tg.target_group_arn()?.to_string();

            let health = app
                .aws
                .elb
                .describe_target_health()
                .target_group_arn(&arn)
                .send()
                .await
                .ok()?;

            let total = health.target_health_descriptions().len();
            let unhealthy = health
                .target_health_descriptions()
                .iter()
                .filter(|d| {
                    d.target_health()
                        .and_then(|s| s.state())
                        .map(|s| s.as_str() != "healthy")
                        .unwrap_or(true)
                })
                .count();

            Some(TargetGroupInfo {
                arn,
                name: tg.target_group_name().unwrap_or("unknown").to_string(),
                protocol: tg
                    .protocol()
                    .map(|p| format!("{:?}", p))
                    .unwrap_or_else(|| "unknown".into()),
                port: tg.port().unwrap_or(0),
                target_type: tg
                    .target_type()
                    .map(|t| format!("{:?}", t))
                    .unwrap_or_else(|| "unknown".into()),
                attached_load_balancer_arns: tg
                    .load_balancer_arns()
                    .iter()
                    .map(|arn| arn.to_string())
                    .collect(),
                total_targets: total,
                unhealthy_targets: unhealthy,
                tags: Tags::Unavailable,
            })
        })
        .buffered(DESCRIBE_CONCURRENCY)
        .filter_map(|group| async move { group })
        .collect()
        .await;

    let mut groups: Vec<TargetGroupInfo> = groups;
    let arns = groups.iter().map(|tg| tg.arn.clone()).collect::<Vec<_>>();
    let mut tags_by_arn = tags::for_elb_arns(&app.aws.elb, &arns).await;

    for group in &mut groups {
        group.tags = tags_by_arn.remove(&group.arn).unwrap_or(Tags::Unavailable);
    }

    (groups, ServiceStatus::Ok)
}
