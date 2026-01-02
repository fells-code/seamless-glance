use crate::models::service_status::ServiceStatus;
use aws_sdk_elasticloadbalancingv2::Client;

pub struct ElbResult {
    pub count: u32,
    pub status: ServiceStatus,
}

pub async fn fetch_load_balancer_count() -> ElbResult {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::v2025_08_07()).await;
    let elb = Client::new(&config);

    match elb.describe_load_balancers().send().await {
        Ok(resp) => ElbResult {
            count: resp.load_balancers().len() as u32,
            status: ServiceStatus::Ok,
        },

        Err(err) => {
            let msg = err.to_string();

            if msg.contains("AccessDenied") {
                ElbResult {
                    count: 0,
                    status: ServiceStatus::AccessDenied,
                }
            } else {
                ElbResult {
                    count: 0,
                    status: ServiceStatus::Unavailable(msg),
                }
            }
        }
    }
}
