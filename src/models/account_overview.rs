use crate::models::{cloudwatch::CloudWatchSummary, service_status::ServiceStatus};

#[derive(Debug, Clone)]
pub struct AccountOverview {
    pub account_id: String,
    pub month_to_date_cost: f64,

    pub role_name: Option<String>,
    pub region: String,

    pub ecs_clusters: u32,
    pub ecs_services: u32,

    pub rds_instances: u32,
    pub load_balancers: u32,

    pub rds_status: ServiceStatus,
    pub elb_status: ServiceStatus,

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
}
