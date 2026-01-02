use aws_sdk_ec2::Client as Ec2Client;
use aws_sdk_ecs::Client as EcsClient;
use aws_sdk_s3::Client as S3Client;

use crate::models::AwsServiceItem;

pub async fn fetch_services() -> Vec<AwsServiceItem> {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::v2025_08_07()).await;

    let ec2 = Ec2Client::new(&config);
    let s3 = S3Client::new(&config);
    let ecs = EcsClient::new(&config);

    let mut results = vec![];

    // --- EC2 ---
    if let Ok(resp) = ec2.describe_instances().send().await {
        let count: usize = resp
            .reservations()
            .iter()
            .map(|res| res.instances().len())
            .sum();

        results.push(AwsServiceItem {
            name: "EC2 Instances".into(),
            resource_type: "EC2".into(),
            count,
        });
    }

    // --- S3 ---
    if let Ok(resp) = s3.list_buckets().send().await {
        let count = resp.buckets().len();

        results.push(AwsServiceItem {
            name: "S3 Buckets".into(),
            resource_type: "S3".into(),
            count,
        });
    }

    // --- ECS ---
    if let Ok(resp) = ecs.list_clusters().send().await {
        let count = resp.cluster_arns().len();

        results.push(AwsServiceItem {
            name: "ECS Clusters".into(),
            resource_type: "ECS".into(),
            count,
        });
    }

    results
}
