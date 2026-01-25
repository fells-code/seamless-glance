use crate::{app::App, models::elb::LoadBalancerInfo};

pub async fn fetch_load_balancers(app: &App) -> Vec<LoadBalancerInfo> {
    let resp = match app.aws.elb.describe_load_balancers().send().await {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    resp.load_balancers()
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
        })
        .collect()
}
