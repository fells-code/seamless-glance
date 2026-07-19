use crate::app::App;
use crate::aws::apigateway;
use crate::aws::{cloudwatch, ec2, ecs, lambda, rds, secrets, sqs, target_group, vpc};
use crate::models::AccountOverview;
use tokio::join;

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

pub async fn fetch_account_overview(app: &App) -> AccountOverview {
    // TODO(#16): surface a denied/throttled sts:GetCallerIdentity in the UI once
    // the error surface exists; for now degrade to an unknown identity.
    let ident = app.aws.sts.get_caller_identity().send().await.ok();
    let arn = ident.as_ref().and_then(|i| i.arn()).unwrap_or("");
    let identity = parse_identity_from_arn(arn);

    let (
        ecs_clusters,
        ec2_counts,
        rds_result,
        lambda_result,
        apigw_result,
        sqs_result,
        vpc_result,
        alarms,
        secrets,
        target_groups,
    ) = join!(
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

    let ecs_clusters_count = ecs_clusters.len() as u32;
    let ecs_services_count: u32 = ecs_clusters.iter().map(|c| c.active_services as u32).sum();

    // The account overview only needs the target-group list here; the fetch
    // status is surfaced in the dedicated Target Groups view.
    let (target_groups, _) = target_groups;

    let unhealthy = target_groups
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

        ec2_running: ec2_counts.running,
        ec2_stopped: ec2_counts.stopped,

        ecs_clusters: ecs_clusters_count,
        ecs_services: ecs_services_count,

        rds_status: rds_result.0,
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
        alarms: alarms.0,
        secrets: secrets.0,

        target_groups_total: target_groups.len(),
        target_groups_unhealthy: unhealthy,
    }
}
