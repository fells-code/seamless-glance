use crate::{app::App, models::EcsClusterInfo};

pub async fn fetch_ecs_clusters(app: &App) -> Vec<EcsClusterInfo> {
    // TODO(#16): surface throttle/denied errors in the UI instead of degrading
    // to an empty cluster list once the in-UI error surface exists.
    let arns = match app.aws.ecs.list_clusters().send().await {
        Ok(resp) => resp.cluster_arns().to_vec(),
        Err(_) => return vec![],
    };

    if arns.is_empty() {
        return vec![];
    }

    // describe_clusters takes multiple .clusters("arn") calls
    let mut builder = app.aws.ecs.describe_clusters();
    for arn in &arns {
        builder = builder.clusters(arn);
    }

    let resp = match builder.send().await {
        Ok(resp) => resp,
        Err(_) => return vec![],
    };

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
