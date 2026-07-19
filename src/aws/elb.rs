use std::collections::HashMap;

use crate::{
    app::App,
    models::{elb::LoadBalancerInfo, service_status::ServiceStatus, target_group::TargetGroupInfo},
};

pub async fn fetch_load_balancers(app: &App) -> (Vec<LoadBalancerInfo>, ServiceStatus) {
    let resp = match app.aws.elb.describe_load_balancers().send().await {
        Ok(r) => r,
        Err(err) => return (vec![], ServiceStatus::from_error_message(err.to_string())),
    };

    let load_balancers = resp
        .load_balancers()
        .iter()
        .map(|lb| LoadBalancerInfo {
            arn: lb.load_balancer_arn().unwrap_or_default().to_string(),
            name: lb.load_balancer_name().unwrap_or("unknown").to_string(),
            lb_type: lb
                .r#type()
                .map(|t| format!("{:?}", t))
                .unwrap_or_else(|| "unknown".into()),
            scheme: lb
                .scheme()
                .map(|s| format!("{:?}", s))
                .unwrap_or_else(|| "unknown".into()),
            state: lb
                .state()
                .and_then(|s| s.code())
                .map(|c| format!("{:?}", c))
                .unwrap_or_else(|| "unknown".into()),
            az_count: lb.availability_zones().len(),
            attached_target_groups: 0,
            total_targets: 0,
            healthy_targets: 0,
        })
        .collect();

    (load_balancers, ServiceStatus::Ok)
}

pub fn apply_target_group_health(
    load_balancers: &mut [LoadBalancerInfo],
    target_groups: &[TargetGroupInfo],
) {
    let mut health_by_lb: HashMap<&str, (usize, usize, usize)> = HashMap::new();

    for target_group in target_groups {
        for lb_arn in &target_group.attached_load_balancer_arns {
            let entry = health_by_lb.entry(lb_arn.as_str()).or_insert((0, 0, 0));
            entry.0 += 1;
            entry.1 += target_group.total_targets;
            entry.2 += target_group.healthy_targets();
        }
    }

    for load_balancer in load_balancers {
        if let Some((attached_target_groups, total_targets, healthy_targets)) =
            health_by_lb.get(load_balancer.arn.as_str())
        {
            load_balancer.attached_target_groups = *attached_target_groups;
            load_balancer.total_targets = *total_targets;
            load_balancer.healthy_targets = *healthy_targets;
        }
    }
}
