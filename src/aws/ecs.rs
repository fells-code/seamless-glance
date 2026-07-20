use crate::{app::App, aws::tags, models::EcsClusterInfo};
use aws_sdk_ecs::types::ClusterField;

pub async fn fetch_ecs_clusters(app: &App) -> Vec<EcsClusterInfo> {
    // TODO(#16): surface throttle/denied errors in the UI instead of degrading
    // to an empty cluster list once the in-UI error surface exists.
    let mut arn_pages = app.aws.ecs.list_clusters().into_paginator().items().send();
    let mut arns = Vec::new();
    while let Some(item) = arn_pages.next().await {
        match item {
            Ok(arn) => arns.push(arn),
            Err(_) => return vec![],
        }
    }

    if arns.is_empty() {
        return vec![];
    }

    // describe_clusters accepts at most 100 cluster identifiers per call.
    let mut clusters = Vec::new();
    for chunk in arns.chunks(100) {
        // DescribeClusters omits tags unless they are explicitly included, at no
        // extra request cost.
        let mut builder = app.aws.ecs.describe_clusters().include(ClusterField::Tags);
        for arn in chunk {
            builder = builder.clusters(arn);
        }

        let resp = match builder.send().await {
            Ok(resp) => resp,
            Err(_) => return vec![],
        };

        for c in resp.clusters() {
            clusters.push(EcsClusterInfo {
                tags: tags::from_pairs(c.tags().iter().map(|t| (t.key(), t.value()))),
                arn: c.cluster_arn().unwrap_or("").into(),
                name: c.cluster_name().unwrap_or("").into(),
                running_tasks: c.running_tasks_count(),
                pending_tasks: c.pending_tasks_count(),
                active_services: c.active_services_count(),
                registered_container_instances: c.registered_container_instances_count(),
                cpu: c.registered_container_instances_count(), // placeholder
                memory: c.registered_container_instances_count(), // placeholder
            });
        }
    }

    clusters
}
