use crate::{
    app::App,
    models::{elb::ElbResult, service_status::ServiceStatus},
};

pub async fn fetch_load_balancer_count(app: &App) -> ElbResult {
    match app.aws.elb.describe_load_balancers().send().await {
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
