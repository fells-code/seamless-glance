use crate::models::rds::{RdsInstanceInfo, RdsSummary};
use crate::models::service_status::ServiceStatus;
use aws_sdk_rds::Client;

pub async fn fetch_rds(app: &crate::app::App) -> (RdsSummary, Vec<RdsInstanceInfo>) {
    let config = aws_config::defaults(aws_config::BehaviorVersion::v2025_08_07())
        .region(app.current_region().clone())
        .load()
        .await;

    let client = Client::new(&config);

    let resp = match client.describe_db_instances().send().await {
        Ok(r) => r,
        Err(err) => {
            let msg = err.to_string();
            let status = if msg.contains("AccessDenied") {
                ServiceStatus::AccessDenied
            } else {
                ServiceStatus::Unavailable(msg)
            };

            return (
                RdsSummary {
                    status,
                    total: 0,
                    available: 0,
                },
                vec![],
            );
        }
    };

    let mut instances = Vec::new();
    let mut available = 0;

    for db in resp.db_instances() {
        let status = db.db_instance_status().unwrap_or("unknown").to_string();
        if status == "available" {
            available += 1;
        }

        instances.push(RdsInstanceInfo {
            identifier: db.db_instance_identifier().unwrap_or("unknown").to_string(),
            engine: db.engine().unwrap_or("unknown").to_string(),
            instance_class: db.db_instance_class().unwrap_or("unknown").to_string(),
            status,
            az: db.availability_zone().unwrap_or("-").to_string(),
            multi_az: db.multi_az().unwrap_or(false),
        });
    }

    (
        RdsSummary {
            status: ServiceStatus::Ok,
            total: instances.len(),
            available,
        },
        instances,
    )
}
