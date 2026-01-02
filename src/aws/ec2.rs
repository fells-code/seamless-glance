use aws_sdk_ec2::{types::InstanceStateName, Client};

pub struct Ec2Counts {
    pub running: u32,
    pub stopped: u32,
}

pub async fn fetch_ec2_counts() -> Ec2Counts {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::v2025_08_07()).await;
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
