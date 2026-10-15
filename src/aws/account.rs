use aws_sdk_sts::operation::get_caller_identity::GetCallerIdentityOutput;
use tokio::join;

use crate::app::App;
use crate::aws::apigateway;
use crate::aws::ec2::Ec2Counts;
use crate::aws::{cloudwatch, ec2, ecs, lambda, rds, secrets, sqs, target_group, vpc};
use crate::models::apigateway::{ApiGatewayInfo, ApiGatewaySummary};
use crate::models::cloudwatch::CloudWatchSummary;
use crate::models::ec2::Ec2InstanceInfo;
use crate::models::lambda::{LambdaFunctionInfo, LambdaSummary};
use crate::models::rds::RdsSummary;
use crate::models::secrets::SecretsSummary;
use crate::models::service_status::ServiceStatus;
use crate::models::sqs::{SqsQueueInfo, SqsSummary};
use crate::models::target_group::TargetGroupInfo;
use crate::models::vpc::{VpcInfo, VpcSummary};
use crate::models::{AccountOverview, EcsClusterInfo};

pub struct IdentityInfo {
    pub kind: String,
    pub name: String,
}

fn parse_identity_from_arn(arn: &str) -> IdentityInfo {
    if arn.contains(":assumed-role/") {
        let parts: Vec<&str> = arn.split(":assumed-role/").collect();
        let role_part = parts.get(1).unwrap_or(&"unknown/unknown");
        let role_name = role_part.split('/').next().unwrap_or("unknown");

        IdentityInfo {
            kind: "Assumed Role".into(),
            name: role_name.into(),
        }
    } else if arn.contains(":user/") {
        let user_name = arn.split(":user/").last().unwrap_or("unknown");
        IdentityInfo {
            kind: "IAM User".into(),
            name: user_name.into(),
        }
    } else if arn.ends_with(":root") {
        IdentityInfo {
            kind: "Root".into(),
            name: "root".into(),
        }
    } else {
        IdentityInfo {
            kind: "Unknown".into(),
            name: arn.to_string(),
        }
    }
}

/// Every service summary the account overview is assembled from.
///
/// The overview can either fetch these itself or be handed them by a view that
/// already fetched the full inventories, which is how Findings avoids fetching
/// the same services twice.
pub struct OverviewParts {
    pub ecs_clusters: Vec<EcsClusterInfo>,
    pub ec2: Ec2Counts,
    pub rds: RdsSummary,
    pub lambda: LambdaSummary,
    pub apigw: ApiGatewaySummary,
    pub sqs: SqsSummary,
    pub vpc: VpcSummary,
    pub alarms: CloudWatchSummary,
    pub secrets: SecretsSummary,
    pub target_groups: Vec<TargetGroupInfo>,
}

pub async fn fetch_identity(app: &App) -> Option<GetCallerIdentityOutput> {
    // TODO(#16): surface a denied/throttled sts:GetCallerIdentity in the UI once
    // the error surface exists; for now degrade to an unknown identity.
    app.aws.sts.get_caller_identity().send().await.ok()
}

pub async fn fetch_account_overview(app: &App) -> AccountOverview {
    let (
        ident,
        ecs_clusters,
        ec2,
        (rds, _),
        lambda,
        apigw,
        sqs,
        vpc,
        (alarms, _),
        (secrets, _),
        (target_groups, _),
    ) = join!(
        fetch_identity(app),
        ecs::fetch_ecs_clusters(app),
        ec2::fetch_ec2_counts(app),
        rds::fetch_rds(app),
        lambda::fetch_lambda_summary(app),
        apigateway::fetch_apigateway_summary(app),
        sqs::fetch_sqs_summary(app),
        vpc::fetch_vpc_summary(app),
        cloudwatch::fetch_cloudwatch(app),
        secrets::fetch_secrets(app),
        target_group::fetch_target_groups(app)
    );

    assemble_overview(
        app,
        ident,
        OverviewParts {
            ecs_clusters,
            ec2,
            rds,
            lambda,
            apigw,
            sqs,
            vpc,
            alarms,
            secrets,
            // The overview only needs the list here; the fetch status is
            // surfaced in the dedicated Target Groups view.
            target_groups,
        },
    )
}

pub fn assemble_overview(
    app: &App,
    ident: Option<GetCallerIdentityOutput>,
    parts: OverviewParts,
) -> AccountOverview {
    let arn = ident.as_ref().and_then(|i| i.arn()).unwrap_or("");
    let identity = parse_identity_from_arn(arn);

    let ecs_clusters_count = parts.ecs_clusters.len() as u32;
    let ecs_services_count: u32 = parts
        .ecs_clusters
        .iter()
        .map(|c| c.active_services as u32)
        .sum();

    let unhealthy = parts
        .target_groups
        .iter()
        .filter(|tg| tg.unhealthy_targets > 0)
        .count();

    AccountOverview {
        account_id: ident
            .as_ref()
            .and_then(|i| i.account())
            .unwrap_or("unknown")
            .to_string(),
        identity_kind: identity.kind,
        identity_name: identity.name,
        region: app.current_region_label().to_string(),
        role_name: ident
            .as_ref()
            .and_then(|i| i.arn())
            .and_then(|arn| arn.split(":assumed-role/").nth(1))
            .and_then(|s| s.split('/').next())
            .map(|s| s.to_string()),

        ec2_running: parts.ec2.running,
        ec2_stopped: parts.ec2.stopped,

        ecs_clusters: ecs_clusters_count,
        ecs_services: ecs_services_count,

        rds_status: parts.rds,
        lambda_functions: parts.lambda.function_count,
        lambda_status: parts.lambda.status,

        apigw_rest_apis: parts.apigw.rest_count,
        apigw_http_apis: parts.apigw.http_count,
        apigw_status: parts.apigw.status,

        sqs_queues: parts.sqs.queue_count,
        sqs_dlqs: parts.sqs.dlq_count,
        sqs_status: parts.sqs.status,

        vpc_count: parts.vpc.vpc_count,
        subnet_count: parts.vpc.subnet_count,
        vpc_status: parts.vpc.status,
        alarms: parts.alarms,
        secrets: parts.secrets,

        target_groups_total: parts.target_groups.len(),
        target_groups_unhealthy: unhealthy,
    }
}

pub fn lambda_summary(functions: &[LambdaFunctionInfo], status: ServiceStatus) -> LambdaSummary {
    LambdaSummary {
        function_count: functions.len() as u32,
        status,
    }
}

/// HTTP and WebSocket APIs both come from API Gateway v2, and the overview
/// counts every v2 API as HTTP, as its own summary fetch does.
pub fn apigateway_summary(apis: &[ApiGatewayInfo], status: ServiceStatus) -> ApiGatewaySummary {
    let rest_count = apis.iter().filter(|api| api.api_type == "REST").count() as u32;

    ApiGatewaySummary {
        rest_count,
        http_count: apis.len() as u32 - rest_count,
        status,
    }
}

pub fn sqs_summary(queues: &[SqsQueueInfo], status: ServiceStatus) -> SqsSummary {
    SqsSummary {
        queue_count: queues.len() as u32,
        dlq_count: queues.iter().filter(|queue| queue.has_dlq).count() as u32,
        status,
    }
}

pub fn vpc_summary(vpcs: &[VpcInfo], status: ServiceStatus) -> VpcSummary {
    VpcSummary {
        vpc_count: vpcs.len() as u32,
        subnet_count: vpcs.iter().map(|vpc| vpc.subnet_count).sum(),
        status,
    }
}

pub fn ec2_counts(instances: &[Ec2InstanceInfo]) -> Ec2Counts {
    let mut counts = Ec2Counts {
        running: 0,
        stopped: 0,
    };

    for instance in instances {
        match instance.state.as_str() {
            "running" => counts.running += 1,
            "stopped" => counts.stopped += 1,
            _ => {}
        }
    }

    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tags::Tags;

    fn api(api_type: &str) -> ApiGatewayInfo {
        ApiGatewayInfo {
            id: "a1".into(),
            name: "orders".into(),
            api_type: api_type.into(),
            created_at: "-".into(),
            tags: Tags::empty(),
        }
    }

    fn queue(has_dlq: bool) -> SqsQueueInfo {
        SqsQueueInfo {
            name: "orders".into(),
            queue_url: "https://sqs.us-east-1.amazonaws.com/123456789012/orders".into(),
            is_fifo: false,
            messages_available: 0,
            messages_in_flight: 0,
            has_dlq,
            dead_letter_target_arn: None,
            tags: Tags::empty(),
        }
    }

    fn vpc(subnet_count: u32) -> VpcInfo {
        VpcInfo {
            vpc_id: "vpc-1".into(),
            cidr: "10.0.0.0/16".into(),
            state: "Available".into(),
            is_default: false,
            subnet_count,
            tags: Tags::empty(),
        }
    }

    fn instance(state: &str) -> Ec2InstanceInfo {
        Ec2InstanceInfo {
            id: "i-1".into(),
            tags: Tags::empty(),
            avg_cpu_utilization: None,
            instance_type: "t3.micro".into(),
            state: state.into(),
            region: "us-east-1".into(),
            az: "us-east-1a".into(),
            private_ip: None,
            public_ip: None,
            key_name: None,
        }
    }

    /// The summary fetch counts every API Gateway v2 API as HTTP, including
    /// WebSocket APIs, so the derived summary must match it.
    #[test]
    fn every_non_rest_api_counts_as_http() {
        let apis = vec![api("REST"), api("REST"), api("Http"), api("Websocket")];

        let summary = apigateway_summary(&apis, ServiceStatus::Ok);

        assert_eq!(summary.rest_count, 2);
        assert_eq!(summary.http_count, 2);
    }

    #[test]
    fn dlq_count_is_queues_with_a_redrive_policy() {
        let queues = vec![queue(true), queue(false), queue(true)];

        let summary = sqs_summary(&queues, ServiceStatus::Ok);

        assert_eq!(summary.queue_count, 3);
        assert_eq!(summary.dlq_count, 2);
    }

    #[test]
    fn subnet_count_sums_across_vpcs() {
        let summary = vpc_summary(&[vpc(3), vpc(2)], ServiceStatus::Ok);

        assert_eq!(summary.vpc_count, 2);
        assert_eq!(summary.subnet_count, 5);
    }

    #[test]
    fn only_running_and_stopped_instances_are_counted() {
        let instances = vec![
            instance("running"),
            instance("running"),
            instance("stopped"),
            instance("terminated"),
        ];

        let counts = ec2_counts(&instances);

        assert_eq!(counts.running, 2);
        assert_eq!(counts.stopped, 1);
    }

    #[test]
    fn a_failed_fetch_carries_its_status_into_the_summary() {
        let summary = lambda_summary(&[], ServiceStatus::AccessDenied);

        assert_eq!(summary.function_count, 0);
        assert!(matches!(summary.status, ServiceStatus::AccessDenied));
    }
}
