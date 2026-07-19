use aws_sdk_ec2::Client;

use crate::aws::clients::build_sdk_config;

/// Commercial regions enabled by default on a standard AWS account. Used as a
/// documented fallback when `ec2:DescribeRegions` cannot be reached, so global
/// views still span more than one region instead of silently collapsing.
pub const FALLBACK_REGIONS: [&str; 17] = [
    "us-east-1",
    "us-east-2",
    "us-west-1",
    "us-west-2",
    "ca-central-1",
    "eu-west-1",
    "eu-west-2",
    "eu-west-3",
    "eu-central-1",
    "eu-north-1",
    "ap-south-1",
    "ap-southeast-1",
    "ap-southeast-2",
    "ap-northeast-1",
    "ap-northeast-2",
    "ap-northeast-3",
    "sa-east-1",
];

pub fn fallback_regions() -> Vec<String> {
    FALLBACK_REGIONS.iter().map(|r| r.to_string()).collect()
}

/// Discover the regions enabled for the active account. Returns an error string
/// (rather than a silent single-region fallback) so the caller can surface the
/// failure and choose a documented fallback.
pub async fn fetch_enabled_regions(profile: Option<&str>) -> Result<Vec<String>, String> {
    let config = build_sdk_config(aws_config::Region::new("us-east-1"), profile).await;
    let ec2 = Client::new(&config);

    match ec2.describe_regions().send().await {
        Ok(resp) => Ok(resp
            .regions()
            .iter()
            .filter_map(|r| r.region_name())
            .map(|r| r.to_string())
            .collect()),

        Err(err) => Err(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn fallback_spans_multiple_regions_without_duplicates() {
        let regions = fallback_regions();
        assert!(
            regions.len() > 1,
            "fallback must span more than one region so global views are not silently collapsed"
        );
        assert!(regions.iter().any(|r| r == "us-east-1"));

        let unique: HashSet<&String> = regions.iter().collect();
        assert_eq!(
            unique.len(),
            regions.len(),
            "fallback has duplicate regions"
        );
    }
}
