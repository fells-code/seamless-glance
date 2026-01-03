use crate::{app::App, models::ec2::Ec2InstanceInfo};
use aws_sdk_ec2::{types::InstanceStateName, Client};

pub struct Ec2Counts {
    pub running: u32,
    pub stopped: u32,
}

pub async fn fetch_ec2_counts(app: &App) -> Ec2Counts {
    let config = aws_config::defaults(aws_config::BehaviorVersion::v2025_08_07())
        .region(app.current_region().clone())
        .load()
        .await;
    let ec2 = Client::new(&config);

    let resp = match ec2.describe_instances().send().await {
        Ok(r) => r,
        Err(err) => {
            eprintln!("EC2 describe_instances failed: {:?}", err);
            return Ec2Counts {
                running: 0,
                stopped: 0,
            };
        }
    };

    let mut running = 0;
    let mut stopped = 0;

    for reservation in resp.reservations() {
        for instance in reservation.instances() {
            match instance.state().and_then(|s| s.name()) {
                Some(InstanceStateName::Running) => running += 1,
                Some(InstanceStateName::Stopped) => stopped += 1,
                _ => {}
            }
        }
    }

    Ec2Counts { running, stopped }
}

pub async fn fetch_instances(app: &App) -> Vec<Ec2InstanceInfo> {
    let config = aws_config::defaults(aws_config::BehaviorVersion::v2025_08_07())
        .region(app.current_region().clone())
        .load()
        .await;
    let client = Client::new(&config);

    let resp = match client.describe_instances().send().await {
        Ok(r) => r,
        Err(_) => return vec![],
    };

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
                instance_type: inst.instance_type().unwrap().as_str().to_string(),
                state: inst
                    .state()
                    .and_then(|s| s.name())
                    .map(|s| s.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                az: inst
                    .placement()
                    .and_then(|p| p.availability_zone())
                    .unwrap_or("")
                    .to_string(),
                private_ip: inst.private_ip_address().map(|s| s.to_string()),
            });
        }
    }

    instances
}
