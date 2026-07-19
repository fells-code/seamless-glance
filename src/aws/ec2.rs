use crate::{
    app::App,
    aws::clients::{build_sdk_config, AwsClients},
    models::ec2::Ec2InstanceInfo,
    resources::region_aggregate::fetch_all_regions,
};
use aws_sdk_cloudwatch::{
    primitives::DateTime,
    types::{Dimension, Metric, MetricDataQuery, MetricStat, StandardUnit},
};
use aws_types::region::Region;
use chrono::Utc;
use std::collections::HashMap;

pub struct Ec2Counts {
    pub running: u32,
    pub stopped: u32,
}

async fn clients_for_region(region: &Region, profile: Option<&str>) -> AwsClients {
    let sdk_config = build_sdk_config(region.clone(), profile).await;
    AwsClients::new(&sdk_config)
}

async fn fetch_average_cpu_utilization(
    aws: &AwsClients,
    instances: &[Ec2InstanceInfo],
) -> Result<HashMap<String, f64>, String> {
    let running_instances = instances
        .iter()
        .filter(|instance| instance.is_running())
        .collect::<Vec<_>>();

    if running_instances.is_empty() {
        return Ok(HashMap::new());
    }

    let end = Utc::now();
    let start = end - chrono::Duration::days(Ec2InstanceInfo::LOW_CPU_LOOKBACK_DAYS);
    let mut averages_by_instance = HashMap::new();

    for chunk in running_instances.chunks(100) {
        let mut instance_ids_by_query = HashMap::new();
        let queries = chunk
            .iter()
            .enumerate()
            .map(|(index, instance)| {
                let query_id = format!("m{index}");
                instance_ids_by_query.insert(query_id.clone(), instance.id.clone());

                MetricDataQuery::builder()
                    .id(query_id)
                    .metric_stat(
                        MetricStat::builder()
                            .metric(
                                Metric::builder()
                                    .namespace("AWS/EC2")
                                    .metric_name("CPUUtilization")
                                    .dimensions(
                                        Dimension::builder()
                                            .name("InstanceId")
                                            .value(instance.id.clone())
                                            .build(),
                                    )
                                    .build(),
                            )
                            .period(Ec2InstanceInfo::LOW_CPU_PERIOD_SECONDS)
                            .stat("Average")
                            .unit(StandardUnit::Percent)
                            .build(),
                    )
                    .return_data(true)
                    .build()
            })
            .collect::<Vec<_>>();

        let response = aws
            .cw
            .get_metric_data()
            .set_metric_data_queries(Some(queries))
            .start_time(DateTime::from_secs(start.timestamp()))
            .end_time(DateTime::from_secs(end.timestamp()))
            .send()
            .await
            .map_err(|err| {
                format!("CloudWatch get_metric_data failed for EC2 CPU lookup: {err:?}")
            })?;

        for result in response.metric_data_results() {
            let Some(query_id) = result.id() else {
                continue;
            };

            let Some(instance_id) = instance_ids_by_query.get(query_id) else {
                continue;
            };

            let values = result.values();
            if values.is_empty() {
                continue;
            }

            let average = values.iter().sum::<f64>() / values.len() as f64;
            averages_by_instance.insert(instance_id.clone(), average);
        }
    }

    Ok(averages_by_instance)
}

async fn fetch_instances_for_region(
    region: Region,
    include_cpu_metrics: bool,
    profile: Option<String>,
) -> Result<Vec<Ec2InstanceInfo>, String> {
    let aws = clients_for_region(&region, profile.as_deref()).await;

    let mut pages = aws.ec2.describe_instances().into_paginator().items().send();

    let mut instances = vec![];

    while let Some(item) = pages.next().await {
        let reservation = item.map_err(|err| {
            format!(
                "EC2 describe_instances failed for {}: {:?}",
                region.as_ref(),
                err
            )
        })?;

        for inst in reservation.instances() {
            let tag_value = |key: &str| {
                inst.tags()
                    .iter()
                    .find(|t| t.key().unwrap_or("") == key)
                    .and_then(|t| t.value().map(|v| v.to_string()))
            };

            let name = inst
                .tags()
                .iter()
                .find(|t| t.key().unwrap_or("") == "Name")
                .and_then(|t| t.value().map(|v| v.to_string()));

            instances.push(Ec2InstanceInfo {
                id: inst.instance_id().unwrap_or("").to_string(),
                name,
                owner: tag_value("Owner"),
                environment: tag_value("Environment"),
                avg_cpu_utilization: None,
                region: region.as_ref().to_string(),
                instance_type: inst
                    .instance_type()
                    .map(|t| t.as_str().to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                state: inst
                    .state()
                    .and_then(|s| s.name())
                    .map(|s| s.as_str().to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                az: inst
                    .placement()
                    .and_then(|p| p.availability_zone())
                    .unwrap_or("")
                    .to_string(),
                private_ip: inst.private_ip_address().map(|s| s.to_string()),
                public_ip: inst.public_ip_address().map(|s| s.to_string()),
                key_name: inst.key_name().map(|k| k.to_string()),
            });
        }
    }

    if include_cpu_metrics {
        let averages_by_instance = fetch_average_cpu_utilization(&aws, &instances).await?;
        for instance in &mut instances {
            instance.avg_cpu_utilization = averages_by_instance.get(&instance.id).copied();
        }
    }

    Ok(instances)
}

async fn fetch_instances_with_metrics(
    app: &App,
    include_cpu_metrics: bool,
) -> Vec<Ec2InstanceInfo> {
    let profile = app.current_profile.clone();
    let mut instances = if app.is_global_region_selected() {
        fetch_all_regions(&app.regions, move |region| {
            fetch_instances_for_region(region, include_cpu_metrics, profile.clone())
        })
        .await
    } else {
        match fetch_instances_for_region(app.current_region().clone(), include_cpu_metrics, profile)
            .await
        {
            Ok(items) => items,
            Err(err) => {
                eprintln!("{}", err);
                vec![]
            }
        }
    };

    instances.sort_by(|a, b| {
        a.region
            .cmp(&b.region)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.id.cmp(&b.id))
    });

    instances
}

pub async fn fetch_instances(app: &App) -> Vec<Ec2InstanceInfo> {
    fetch_instances_with_metrics(app, true).await
}

pub async fn fetch_ec2_counts(app: &App) -> Ec2Counts {
    let instances = fetch_instances_with_metrics(app, false).await;

    let mut running = 0;
    let mut stopped = 0;

    for instance in instances {
        match instance.state.as_str() {
            "running" => running += 1,
            "stopped" => stopped += 1,
            _ => {}
        }
    }

    Ec2Counts { running, stopped }
}
