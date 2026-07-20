use crate::app::App;
use crate::aws::clients::clients_for_region;
use crate::aws::tags;
use crate::models::rds::{RdsInstanceInfo, RdsSummary};
use crate::models::service_status::ServiceStatus;
use crate::resources::region_aggregate::fetch_all_regions;
use aws_types::region::Region;

const UNAVAILABLE_EVERYWHERE: &str = "RDS unavailable in all regions";

async fn fetch_rds_for_region(
    region: Region,
    profile: Option<String>,
) -> Result<Vec<RdsInstanceInfo>, ServiceStatus> {
    let aws = clients_for_region(&region, profile.as_deref()).await;

    let mut pages = aws
        .rds
        .describe_db_instances()
        .into_paginator()
        .items()
        .send();

    let mut instances = Vec::new();

    while let Some(item) = pages.next().await {
        let db = match item {
            Ok(db) => db,
            Err(err) => return Err(ServiceStatus::from_sdk_error(&err)),
        };

        let status = db.db_instance_status().unwrap_or("unknown").to_string();

        instances.push(RdsInstanceInfo {
            tags: tags::from_pairs(db.tag_list().iter().map(|t| (t.key(), t.value()))),
            identifier: db.db_instance_identifier().unwrap_or("unknown").to_string(),
            region: region.as_ref().to_string(),
            engine: db.engine().unwrap_or("unknown").to_string(),
            instance_class: db.db_instance_class().unwrap_or("unknown").to_string(),
            status,
            az: db.availability_zone().unwrap_or("-").to_string(),
            multi_az: db.multi_az().unwrap_or(false),
        });
    }

    Ok(instances)
}

/// Instances reporting the `available` state, derived from the rows rather than
/// tallied per region so the single-region and global paths agree.
fn available_count(instances: &[RdsInstanceInfo]) -> usize {
    instances
        .iter()
        .filter(|instance| instance.status == "available")
        .count()
}

pub async fn fetch_rds(app: &App) -> (RdsSummary, Vec<RdsInstanceInfo>) {
    let profile = app.current_profile.clone();

    let (mut instances, status) = if app.is_global_region_selected() {
        fetch_all_regions(&app.regions, UNAVAILABLE_EVERYWHERE, move |region| {
            fetch_rds_for_region(region, profile.clone())
        })
        .await
    } else {
        match fetch_rds_for_region(app.current_region().clone(), profile).await {
            Ok(instances) => (instances, ServiceStatus::Ok),
            Err(status) => (vec![], status),
        }
    };

    instances.sort_by(|a, b| {
        a.region
            .cmp(&b.region)
            .then_with(|| a.identifier.cmp(&b.identifier))
    });

    let summary = RdsSummary {
        status,
        total: instances.len(),
        available: available_count(&instances),
    };

    (summary, instances)
}
