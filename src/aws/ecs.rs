use crate::{
    app::App,
    aws::tags,
    models::{ecs::ClusterCapacity, EcsClusterInfo},
};
use aws_sdk_ecs::types::ClusterField;
use aws_sdk_ecs::Client as EcsClient;

/// Resource names ECS reports on a container instance.
const CPU_RESOURCE: &str = "CPU";
const MEMORY_RESOURCE: &str = "MEMORY";

/// Sum one named resource across a set of container-instance resource lists.
fn total(resources: &[aws_sdk_ecs::types::Resource], name: &str) -> i32 {
    resources
        .iter()
        .filter(|resource| resource.name() == Some(name))
        .map(|resource| resource.integer_value())
        .sum()
}

/// Capacity registered by the instances backing a cluster.
///
/// Only called when the cluster reports container instances, so a Fargate-only
/// cluster costs no extra requests. Returns `None` if the lookup fails, leaving
/// the columns blank rather than reporting a capacity that was never read.
async fn fetch_cluster_capacity(ecs: &EcsClient, cluster_arn: &str) -> Option<ClusterCapacity> {
    let mut capacity = ClusterCapacity {
        registered_cpu_units: 0,
        available_cpu_units: 0,
        registered_memory_mib: 0,
        available_memory_mib: 0,
    };

    let mut pages = ecs
        .list_container_instances()
        .cluster(cluster_arn)
        .into_paginator()
        .items()
        .send();

    let mut instance_arns = Vec::new();
    while let Some(item) = pages.next().await {
        instance_arns.push(item.ok()?);
    }

    // DescribeContainerInstances accepts at most 100 identifiers per call.
    for chunk in instance_arns.chunks(100) {
        let mut request = ecs.describe_container_instances().cluster(cluster_arn);
        for arn in chunk {
            request = request.container_instances(arn);
        }

        let response = request.send().await.ok()?;

        for instance in response.container_instances() {
            let registered = instance.registered_resources();
            let remaining = instance.remaining_resources();

            capacity.registered_cpu_units += total(registered, CPU_RESOURCE);
            capacity.available_cpu_units += total(remaining, CPU_RESOURCE);
            capacity.registered_memory_mib += total(registered, MEMORY_RESOURCE);
            capacity.available_memory_mib += total(remaining, MEMORY_RESOURCE);
        }
    }

    Some(capacity)
}

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
            // Fargate clusters register no instances and have no capacity pool,
            // so this stays None and costs no extra requests.
            let capacity = if c.registered_container_instances_count() > 0 {
                fetch_cluster_capacity(&app.aws.ecs, c.cluster_arn().unwrap_or_default()).await
            } else {
                None
            };

            clusters.push(EcsClusterInfo {
                tags: tags::from_pairs(c.tags().iter().map(|t| (t.key(), t.value()))),
                arn: c.cluster_arn().unwrap_or("").into(),
                name: c.cluster_name().unwrap_or("").into(),
                running_tasks: c.running_tasks_count(),
                pending_tasks: c.pending_tasks_count(),
                active_services: c.active_services_count(),
                registered_container_instances: c.registered_container_instances_count(),
                status: c.status().unwrap_or_default().to_string(),
                capacity,
            });
        }
    }

    clusters
}
