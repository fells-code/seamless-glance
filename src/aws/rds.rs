use crate::app::App;
use crate::aws::clients::{build_sdk_config, AwsClients};
use crate::models::rds::{RdsInstanceInfo, RdsSummary};
use crate::models::service_status::ServiceStatus;
use aws_types::region::Region;
use futures::future::join_all;

async fn clients_for_region(region: &Region, profile: Option<&str>) -> AwsClients {
    let sdk_config = build_sdk_config(region.clone(), profile).await;
    AwsClients::new(&sdk_config)
}

async fn fetch_rds_for_region(
    region: Region,
    profile: Option<String>,
) -> Result<(Vec<RdsInstanceInfo>, usize), ServiceStatus> {
    let aws = clients_for_region(&region, profile.as_deref()).await;

    let resp = match aws.rds.describe_db_instances().send().await {
        Ok(r) => r,
        Err(err) => return Err(ServiceStatus::from_sdk_error(&err)),
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
            region: region.as_ref().to_string(),
            engine: db.engine().unwrap_or("unknown").to_string(),
            instance_class: db.db_instance_class().unwrap_or("unknown").to_string(),
            status,
            az: db.availability_zone().unwrap_or("-").to_string(),
            multi_az: db.multi_az().unwrap_or(false),
        });
    }

    Ok((instances, available))
}

pub async fn fetch_rds(app: &App) -> (RdsSummary, Vec<RdsInstanceInfo>) {
    if !app.is_global_region_selected() {
        return match fetch_rds_for_region(app.current_region().clone(), app.current_profile.clone())
            .await
        {
            Ok((instances, available)) => {
                let total = instances.len();
                (
                    RdsSummary {
                        status: ServiceStatus::Ok,
                        total,
                        available,
                    },
                    instances,
                )
            }
            Err(status) => (
                RdsSummary {
                    status,
                    total: 0,
                    available: 0,
                },
                vec![],
            ),
        };
    }

    let profile = app.current_profile.clone();
    let futures = app
        .regions
        .iter()
        .cloned()
        .map(|region| fetch_rds_for_region(region, profile.clone()));

    let results = join_all(futures).await;

    let mut all_instances = Vec::new();
    let mut total_available = 0usize;
    let mut any_success = false;
    let mut saw_access_denied = false;
    let mut saw_unavailable_msg: Option<String> = None;

    for result in results {
        match result {
            Ok((mut instances, available)) => {
                any_success = true;
                total_available += available;
                all_instances.append(&mut instances);
            }
            Err(ServiceStatus::AccessDenied) => {
                saw_access_denied = true;
            }
            Err(ServiceStatus::Unavailable(msg)) => {
                if saw_unavailable_msg.is_none() {
                    saw_unavailable_msg = Some(msg);
                }
            }
            Err(_) => {}
        }
    }

    all_instances.sort_by(|a, b| {
        a.region
            .cmp(&b.region)
            .then_with(|| a.identifier.cmp(&b.identifier))
    });

    let summary_status = if any_success {
        ServiceStatus::Ok
    } else if saw_access_denied {
        ServiceStatus::AccessDenied
    } else {
        ServiceStatus::Unavailable(
            saw_unavailable_msg.unwrap_or_else(|| "RDS unavailable in all regions".to_string()),
        )
    };

    let total = all_instances.len();

    (
        RdsSummary {
            status: summary_status,
            total,
            available: total_available,
        },
        all_instances,
    )
}
