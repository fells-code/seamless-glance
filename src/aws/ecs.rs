use crate::{app::App, models::EcsClusterInfo};

pub async fn fetch_ecs_clusters(app: &App) -> Vec<EcsClusterInfo> {
    let arns = app
        .aws
        .ecs
        .list_clusters()
        .send()
        .await
        .unwrap()
        .cluster_arns()
        .to_vec();

    if arns.is_empty() {
        return vec![];
    }

    // describe_clusters takes multiple .clusters("arn") calls
    let mut builder = app.aws.ecs.describe_clusters();
    for arn in &arns {
        builder = builder.clusters(arn);
    }

    let resp = builder.send().await.unwrap();

    resp.clusters()
        .iter()
        .map(|c| EcsClusterInfo {
            arn: c.cluster_arn().unwrap_or("").into(),
            name: c.cluster_name().unwrap_or("").into(),
            running_tasks: c.running_tasks_count(),
            pending_tasks: c.pending_tasks_count(),
            active_services: c.active_services_count(),
            registered_container_instances: c.registered_container_instances_count(),
            cpu: c.registered_container_instances_count(), // placeholder
            memory: c.registered_container_instances_count(), // placeholder
        })
        .collect()
}
