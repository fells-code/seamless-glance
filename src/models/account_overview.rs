use crate::models::{
    cloudwatch::CloudWatchSummary, rds::RdsSummary, secrets::SecretsSummary,
    service_status::ServiceStatus,
};

#[derive(Debug, Clone)]
pub struct AccountOverview {
    pub account_id: String,
    pub identity_kind: String,
    pub identity_name: String,

    pub role_name: Option<String>,
    pub region: String,

    pub ec2_running: u32,
    pub ec2_stopped: u32,

    pub ecs_clusters: u32,
    pub ecs_services: u32,

    pub target_groups_total: usize,
    pub target_groups_unhealthy: usize,

    pub rds_status: RdsSummary,

    pub lambda_functions: u32,
    pub lambda_status: ServiceStatus,

    pub apigw_rest_apis: u32,
    pub apigw_http_apis: u32,
    pub apigw_status: ServiceStatus,

    pub sqs_queues: u32,
    pub sqs_dlqs: u32,
    pub sqs_status: ServiceStatus,

    pub vpc_count: u32,
    pub subnet_count: u32,
    pub vpc_status: ServiceStatus,

    pub alarms: CloudWatchSummary,
    pub secrets: SecretsSummary,
}
