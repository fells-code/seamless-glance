use crate::{app::App, models::service_status::ServiceStatus};
use aws_sdk_rds::Client;

pub struct RdsResult {
    pub count: u32,
    pub status: ServiceStatus,
}

pub async fn fetch_rds_instance_count(app: &App) -> RdsResult {
    let config = aws_config::defaults(aws_config::BehaviorVersion::v2025_08_07())
        .region(app.current_region().clone())
        .load()
        .await;
    let rds = Client::new(&config);

    match rds.describe_db_instances().send().await {
        Ok(resp) => RdsResult {
            count: resp.db_instances().len() as u32,
            status: ServiceStatus::Ok,
        },

        Err(err) => {
            let msg = err.to_string();

            if msg.contains("AccessDenied") {
                RdsResult {
                    count: 0,
                    status: ServiceStatus::AccessDenied,
                }
            } else {
                RdsResult {
                    count: 0,
                    status: ServiceStatus::Unavailable(msg),
                }
            }
        }
    }
}
