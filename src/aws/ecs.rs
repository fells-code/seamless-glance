use chrono::Utc;

use crate::{
    app::App,
    models::{EcsClusterInfo, EcsDeploymentInfo, EcsServiceInfo, EcsTaskInfo},
};

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

pub async fn fetch_cluster_services(cluster_arn: &str, app: &App) -> Vec<EcsServiceInfo> {
    let service_arns = app
        .aws
        .ecs
        .list_services()
        .cluster(cluster_arn)
        .send()
        .await
        .unwrap()
        .service_arns()
        .to_vec();

    if service_arns.is_empty() {
        return vec![];
    }

    let mut builder = app.aws.ecs.describe_services().cluster(cluster_arn);
    for arn in &service_arns {
        builder = builder.services(arn);
    }

    let resp = builder.send().await.unwrap();

    resp.services()
        .iter()
        .map(|svc| {
            let deployments = svc
                .deployments()
                .iter()
                .map(|d| EcsDeploymentInfo {
                    id: d.id().unwrap_or("").into(),
                    status: d.status().unwrap_or("").into(),
                    rollout_state: d
                        .rollout_state()
                        .map(|r| format!("{:?}", r))
                        .unwrap_or("UNKNOWN".into()),
                    created_at: format_datetime(d.created_at()),
                    updated_at: format_datetime(d.updated_at()),
                })
                .collect();

            EcsServiceInfo {
                arn: svc.service_arn().unwrap_or("").into(),
                name: svc.service_name().unwrap_or("").into(),
                desired_count: svc.desired_count(),
                running_count: svc.running_count(),
                pending_count: svc.pending_count(),
                deployments,
            }
        })
        .collect()
}

pub async fn fetch_service_tasks(
    cluster_arn: &str,
    service_arn: &str,
    app: &App,
) -> Vec<EcsTaskInfo> {
    let task_arns = app
        .aws
        .ecs
        .list_tasks()
        .cluster(cluster_arn)
        .service_name(service_arn)
        .send()
        .await
        .unwrap()
        .task_arns()
        .to_vec();

    if task_arns.is_empty() {
        return vec![];
    }

    let mut builder = app.aws.ecs.describe_tasks().cluster(cluster_arn);
    for arn in &task_arns {
        builder = builder.tasks(arn);
    }

    let resp = builder.send().await.unwrap();

    resp.tasks()
        .iter()
        .map(|t| EcsTaskInfo {
            arn: t.task_arn().unwrap_or("").into(),
            task_definition: t.task_definition_arn().unwrap_or("").into(),
            last_status: t.last_status().unwrap_or("").into(),
            desired_status: t.desired_status().unwrap_or("").into(),
            cpu: t.cpu().map(|c| c.to_string()),
            memory: t.memory().map(|m| m.to_string()),
            container_instance_arn: t.container_instance_arn().map(|s| s.to_string()),
        })
        .collect()
}

fn format_datetime(opt: Option<&aws_sdk_ecs::primitives::DateTime>) -> String {
    match opt {
        Some(dt) => {
            // extract epoch seconds + nanos
            let secs = dt.secs();
            let nanos = dt.as_nanos();

            // build SystemTime
            let sys = std::time::UNIX_EPOCH
                + std::time::Duration::from_secs(secs as u64)
                + std::time::Duration::from_nanos(nanos as u64);

            // convert to chrono
            let chrono_dt: chrono::DateTime<Utc> = chrono::DateTime::<Utc>::from(sys);

            chrono_dt.to_rfc3339()
        }
        None => "N/A".into(),
    }
}
