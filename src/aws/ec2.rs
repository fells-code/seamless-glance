use crate::{
    app::App, aws::clients::AwsClients, models::ec2::Ec2InstanceInfo,
    resources::region_aggregate::fetch_all_regions,
};
use aws_types::region::Region;

pub struct Ec2Counts {
    pub running: u32,
    pub stopped: u32,
}

async fn clients_for_region(region: &Region) -> AwsClients {
    let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::v2026_01_12())
        .region(region.clone())
        .load()
        .await;

    AwsClients::new(&sdk_config)
}

async fn fetch_instances_for_region(region: Region) -> Result<Vec<Ec2InstanceInfo>, String> {
    let aws = clients_for_region(&region).await;

    let resp = aws.ec2.describe_instances().send().await.map_err(|err| {
        format!(
            "EC2 describe_instances failed for {}: {:?}",
            region.as_ref(),
            err
        )
    })?;

    let mut instances = vec![];

    for reservation in resp.reservations() {
        for inst in reservation.instances() {
            let name = inst
                .tags()
                .iter()
                .find(|t| t.key().unwrap_or("") == "Name")
                .and_then(|t| t.value().map(|v| v.to_string()));

            instances.push(Ec2InstanceInfo {
                id: inst.instance_id().unwrap_or("").to_string(),
                name,
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

    Ok(instances)
}

pub async fn fetch_instances(app: &App) -> Vec<Ec2InstanceInfo> {
    let mut instances = if app.is_global_region_selected() {
        fetch_all_regions(&app.regions, fetch_instances_for_region).await
    } else {
        match fetch_instances_for_region(app.current_region().clone()).await {
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

pub async fn fetch_ec2_counts(app: &App) -> Ec2Counts {
    let instances = fetch_instances(app).await;

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
