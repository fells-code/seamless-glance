use aws_config::BehaviorVersion;
use aws_sdk_ec2::Client;

pub async fn fetch_enabled_regions() -> Vec<String> {
    let config = aws_config::load_defaults(BehaviorVersion::v2025_08_07()).await;
    let ec2 = Client::new(&config);

    match ec2.describe_regions().send().await {
        Ok(resp) => resp
            .regions()
            .iter()
            .filter_map(|r| r.region_name())
            .map(|r| r.to_string())
            .collect(),

        Err(err) => {
            eprintln!("Failed to discover regions: {:?}", err);
            vec!["us-east-1".into()] // safe fallback
        }
    }
}
