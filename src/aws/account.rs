use crate::aws::apigateway;
use crate::aws::{cost, ec2, ecs, elbv2, lambda, rds, sqs, vpc};
use crate::models::AccountOverview;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_sts::Client as StsClient;
use tokio::join;

pub async fn fetch_account_overview(region: &Region) -> AccountOverview {
    let config = aws_config::load_defaults(BehaviorVersion::v2025_08_07())
        .await
        .into_builder()
        .region(region.clone())
        .build();

    let sts = StsClient::new(&config);
    let ident = sts.get_caller_identity().send().await.unwrap();

    let (
        ecs_clusters,
        ec2_counts,
        rds_result,
        elb_result,
        lambda_result,
        apigw_result,
        sqs_result,
        vpc_result,
    ) = join!(
        ecs::fetch_ecs_clusters(),
        ec2::fetch_ec2_counts(),
        rds::fetch_rds_instance_count(),
        elbv2::fetch_load_balancer_count(),
        lambda::fetch_lambda_summary(),
        apigateway::fetch_apigateway_summary(),
        sqs::fetch_sqs_summary(),
        vpc::fetch_vpc_summary()
    );

    let ecs_clusters_count = ecs_clusters.len() as u32;
    let ecs_services_count: u32 = ecs_clusters.iter().map(|c| c.active_services as u32).sum();
    let budget = cost::fetch_budget().await;

    AccountOverview {
        account_id: ident.account().unwrap_or("unknown").to_string(),
        month_to_date_cost: budget.month_to_date_cost,
        region: region.as_ref().to_string(),
        role_name: ident
            .arn()
            .and_then(|arn| arn.split(":assumed-role/").nth(1))
            .and_then(|s| s.split('/').next())
            .map(|s| s.to_string()),

        ec2_running: ec2_counts.running,
        ec2_stopped: ec2_counts.stopped,

        ecs_clusters: ecs_clusters_count,
        ecs_services: ecs_services_count,

        rds_instances: rds_result.count,
        rds_status: rds_result.status,

        load_balancers: elb_result.count,
        elb_status: elb_result.status,

        lambda_functions: lambda_result.function_count,
        lambda_status: lambda_result.status,

        apigw_rest_apis: apigw_result.rest_count,
        apigw_http_apis: apigw_result.http_count,
        apigw_status: apigw_result.status,

        sqs_queues: sqs_result.queue_count,
        sqs_dlqs: sqs_result.dlq_count,
        sqs_status: sqs_result.status,

        vpc_count: vpc_result.vpc_count,
        subnet_count: vpc_result.subnet_count,
        vpc_status: vpc_result.status,
    }
}
