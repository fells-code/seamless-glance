use aws_sdk_ec2::Client;

use crate::aws::clients::build_sdk_config;

pub async fn fetch_enabled_regions(profile: Option<&str>) -> Vec<String> {
    let config = build_sdk_config(aws_config::Region::new("us-east-1"), profile).await;
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
