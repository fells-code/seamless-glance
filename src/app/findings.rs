use std::collections::BTreeSet;

use crate::models::apigatway::ApiGatewayInfo;
use crate::models::cloudwatch::CloudWatchAlarm;
use crate::models::ec2::Ec2InstanceInfo;
use crate::models::elb::LoadBalancerInfo;
use crate::models::finding::{Finding, FindingCategory, FindingRoute, FindingSeverity};
use crate::models::lambda::LambdaFunctionInfo;
use crate::models::rds::RdsInstanceInfo;
use crate::models::secrets::SecretInfo;
use crate::models::security_group::SecurityGroupInfo;
use crate::models::sqs::SqsQueueInfo;
use crate::models::tags::Tags;
use crate::models::target_group::TargetGroupInfo;
use crate::models::vpc::VpcInfo;
use crate::models::AccountOverview;

/// Maximum number of resource labels named inline in a finding summary before
/// the remainder is collapsed into a `(+N more)` suffix.
const SAMPLE_LIMIT: usize = 3;

/// Borrowed view of the application state the finding rules read.
pub struct FindingContext<'a> {
    pub account_overview: Option<&'a AccountOverview>,
    pub region_label: &'a str,
    pub cloudwatch_alarms: &'a [CloudWatchAlarm],
    pub secrets: &'a [SecretInfo],
    pub ec2_instances: &'a [Ec2InstanceInfo],
    pub target_groups: &'a [TargetGroupInfo],
    pub load_balancers: &'a [LoadBalancerInfo],
    pub apigateway_apis: &'a [ApiGatewayInfo],
    pub sqs_queues_data: &'a [SqsQueueInfo],
    pub rds_instances: &'a [RdsInstanceInfo],
    pub security_groups: &'a [SecurityGroupInfo],
    pub vpcs: &'a [VpcInfo],
    pub lambda_functions: &'a [LambdaFunctionInfo],
}

/// Render up to [`SAMPLE_LIMIT`] labels as `a, b, c`, appending `(+N more)`
/// when the list is longer than the limit.
fn sample_list<S: AsRef<str>>(items: &[S]) -> String {
    let sample = items
        .iter()
        .take(SAMPLE_LIMIT)
        .map(|item| item.as_ref())
        .collect::<Vec<_>>()
        .join(", ");
    let remaining = items.len().saturating_sub(SAMPLE_LIMIT);

    if remaining > 0 {
        format!("{sample} (+{remaining} more)")
    } else {
        sample
    }
}

fn cloudwatch_alarms_in_alarm(ctx: &FindingContext) -> Vec<Finding> {
    let Some(overview) = ctx.account_overview else {
        return Vec::new();
    };

    let alarming_alarms = ctx
        .cloudwatch_alarms
        .iter()
        .filter(|alarm| alarm.state == "ALARM")
        .collect::<Vec<_>>();

    if !alarming_alarms.is_empty() {
        let names = alarming_alarms
            .iter()
            .map(|alarm| alarm.name.clone())
            .collect::<Vec<_>>();

        return vec![Finding {
            severity: FindingSeverity::High,
            category: FindingCategory::Incident,
            service: "CloudWatch".into(),
            region: ctx.region_label.to_string(),
            summary: format!(
                "{} alarm(s) are in ALARM: {}",
                alarming_alarms.len(),
                sample_list(&names)
            ),
            next_step: "Open CloudWatch and inspect failing alarms".into(),
            route: FindingRoute::CloudWatch,
        }];
    }

    if overview.alarms.alarms_in_alarm > 0 {
        return vec![Finding {
            severity: FindingSeverity::High,
            category: FindingCategory::Incident,
            service: "CloudWatch".into(),
            region: overview.region.clone(),
            summary: format!(
                "{} alarm(s) are currently in ALARM",
                overview.alarms.alarms_in_alarm
            ),
            next_step: "Open CloudWatch and inspect failing alarms".into(),
            route: FindingRoute::CloudWatch,
        }];
    }

    Vec::new()
}

fn cloudwatch_alarm_coverage_gaps(ctx: &FindingContext) -> Vec<Finding> {
    let Some(overview) = ctx.account_overview else {
        return Vec::new();
    };

    let alarm_namespaces = ctx
        .cloudwatch_alarms
        .iter()
        .map(|alarm| alarm.namespace.as_str())
        .collect::<BTreeSet<_>>();

    let mut coverage_gaps = Vec::new();

    if overview.ec2_running > 0 && !alarm_namespaces.contains("AWS/EC2") {
        coverage_gaps.push(format!("EC2 ({} running)", overview.ec2_running));
    }

    if overview.lambda_functions > 0 && !alarm_namespaces.contains("AWS/Lambda") {
        coverage_gaps.push(format!("Lambda ({} functions)", overview.lambda_functions));
    }

    if overview.rds_status.total > 0 && !alarm_namespaces.contains("AWS/RDS") {
        coverage_gaps.push(format!("RDS ({} instances)", overview.rds_status.total));
    }

    if overview.ecs_services > 0 && !alarm_namespaces.contains("AWS/ECS") {
        coverage_gaps.push(format!("ECS ({} services)", overview.ecs_services));
    }

    let api_total = overview.apigw_rest_apis + overview.apigw_http_apis;
    if api_total > 0 && !alarm_namespaces.contains("AWS/ApiGateway") {
        coverage_gaps.push(format!("API Gateway ({} APIs)", api_total));
    }

    if overview.sqs_queues > 0 && !alarm_namespaces.contains("AWS/SQS") {
        coverage_gaps.push(format!("SQS ({} queues)", overview.sqs_queues));
    }

    if coverage_gaps.is_empty() {
        return Vec::new();
    }

    vec![Finding {
        severity: FindingSeverity::Medium,
        category: FindingCategory::Hygiene,
        service: "CloudWatch".into(),
        region: ctx.region_label.to_string(),
        summary: format!(
            "{} deployed service area(s) appear to have no CloudWatch alarm coverage: {}",
            coverage_gaps.len(),
            sample_list(&coverage_gaps)
        ),
        next_step:
            "Open CloudWatch and add alarms for deployed services without namespace coverage".into(),
        route: FindingRoute::CloudWatch,
    }]
}

fn target_groups_zero_healthy_targets(ctx: &FindingContext) -> Vec<Finding> {
    if ctx.account_overview.is_none() {
        return Vec::new();
    }

    let zero_healthy_target_groups = ctx
        .target_groups
        .iter()
        .filter(|tg| tg.has_zero_healthy_targets())
        .count();

    if zero_healthy_target_groups == 0 {
        return Vec::new();
    }

    vec![Finding {
        severity: FindingSeverity::High,
        category: FindingCategory::Incident,
        service: "Target Groups".into(),
        region: ctx.region_label.to_string(),
        summary: format!("{zero_healthy_target_groups} target group(s) have zero healthy targets"),
        next_step: "Open target groups and restore at least one healthy target".into(),
        route: FindingRoute::TargetGroups,
    }]
}

fn target_groups_unhealthy_targets(ctx: &FindingContext) -> Vec<Finding> {
    let Some(overview) = ctx.account_overview else {
        return Vec::new();
    };

    let partially_unhealthy_target_groups = ctx
        .target_groups
        .iter()
        .filter(|tg| tg.unhealthy_targets > 0 && !tg.has_zero_healthy_targets())
        .count();

    if partially_unhealthy_target_groups > 0 {
        return vec![Finding {
            severity: FindingSeverity::High,
            category: FindingCategory::Incident,
            service: "Target Groups".into(),
            region: ctx.region_label.to_string(),
            summary: format!(
                "{partially_unhealthy_target_groups} target group(s) have unhealthy targets"
            ),
            next_step: "Open target groups and inspect unhealthy target health".into(),
            route: FindingRoute::TargetGroups,
        }];
    }

    if overview.target_groups_unhealthy > 0 && ctx.target_groups.is_empty() {
        return vec![Finding {
            severity: FindingSeverity::High,
            category: FindingCategory::Incident,
            service: "Target Groups".into(),
            region: overview.region.clone(),
            summary: format!(
                "{} target group(s) have unhealthy targets",
                overview.target_groups_unhealthy
            ),
            next_step: "Open target groups and inspect target health".into(),
            route: FindingRoute::TargetGroups,
        }];
    }

    Vec::new()
}

fn target_groups_orphaned(ctx: &FindingContext) -> Vec<Finding> {
    if ctx.account_overview.is_none() {
        return Vec::new();
    }

    let orphan_target_groups = ctx
        .target_groups
        .iter()
        .filter(|tg| tg.is_orphan_candidate())
        .collect::<Vec<_>>();

    if orphan_target_groups.is_empty() {
        return Vec::new();
    }

    let names = orphan_target_groups
        .iter()
        .map(|tg| tg.name.clone())
        .collect::<Vec<_>>();

    vec![Finding {
        severity: FindingSeverity::Medium,
        category: FindingCategory::Waste,
        service: "Target Groups".into(),
        region: ctx.region_label.to_string(),
        summary: format!(
            "{} target group(s) have no load balancer attachment and no registered targets: {}",
            orphan_target_groups.len(),
            sample_list(&names)
        ),
        next_step: "Open target groups and review orphan groups for cleanup or reattachment".into(),
        route: FindingRoute::TargetGroups,
    }]
}

fn secrets_production_rotation_disabled(ctx: &FindingContext) -> Vec<Finding> {
    if ctx.account_overview.is_none() {
        return Vec::new();
    }

    let production_like_rotation_disabled = ctx
        .secrets
        .iter()
        .filter(|secret| secret.needs_rotation_review())
        .collect::<Vec<_>>();

    if production_like_rotation_disabled.is_empty() {
        return Vec::new();
    }

    let names = production_like_rotation_disabled
        .iter()
        .map(|secret| secret.name.clone())
        .collect::<Vec<_>>();

    vec![Finding {
        severity: FindingSeverity::High,
        category: FindingCategory::Hygiene,
        service: "Secrets Manager".into(),
        region: ctx.region_label.to_string(),
        summary: format!(
            "{} production-like secret(s) do not have rotation enabled: {}",
            production_like_rotation_disabled.len(),
            sample_list(&names)
        ),
        next_step: "Open Secrets Manager and enable rotation on production-like secrets".into(),
        route: FindingRoute::Secrets,
    }]
}

fn secrets_rotation_disabled(ctx: &FindingContext) -> Vec<Finding> {
    let Some(overview) = ctx.account_overview else {
        return Vec::new();
    };

    let rotation_disabled = ctx
        .secrets
        .iter()
        .filter(|secret| secret.rotation_disabled() && !secret.needs_rotation_review())
        .count();

    if rotation_disabled > 0 {
        return vec![Finding {
            severity: FindingSeverity::Medium,
            category: FindingCategory::Hygiene,
            service: "Secrets Manager".into(),
            region: ctx.region_label.to_string(),
            summary: format!("{rotation_disabled} secret(s) do not have rotation enabled"),
            next_step: "Review secrets that should rotate automatically".into(),
            route: FindingRoute::Secrets,
        }];
    }

    if overview.secrets.rotation_disabled > 0 && ctx.secrets.is_empty() {
        return vec![Finding {
            severity: FindingSeverity::Medium,
            category: FindingCategory::Hygiene,
            service: "Secrets Manager".into(),
            region: overview.region.clone(),
            summary: format!(
                "{} secret(s) do not have rotation enabled",
                overview.secrets.rotation_disabled
            ),
            next_step: "Review secrets that should rotate automatically".into(),
            route: FindingRoute::Secrets,
        }];
    }

    Vec::new()
}

fn secrets_stale_rotation(ctx: &FindingContext) -> Vec<Finding> {
    if ctx.account_overview.is_none() {
        return Vec::new();
    }

    let stale_rotation_secrets = ctx
        .secrets
        .iter()
        .filter(|secret| secret.has_stale_rotation())
        .collect::<Vec<_>>();

    if stale_rotation_secrets.is_empty() {
        return Vec::new();
    }

    let names = stale_rotation_secrets
        .iter()
        .map(|secret| secret.name.clone())
        .collect::<Vec<_>>();

    vec![Finding {
        severity: FindingSeverity::Medium,
        category: FindingCategory::Hygiene,
        service: "Secrets Manager".into(),
        region: ctx.region_label.to_string(),
        summary: format!(
            "{} secret(s) have not rotated in {}+ days: {}",
            stale_rotation_secrets.len(),
            SecretInfo::STALE_ROTATION_DAYS,
            sample_list(&names)
        ),
        next_step: format!(
            "Open Secrets Manager and review secrets that have not rotated in {}+ days",
            SecretInfo::STALE_ROTATION_DAYS
        ),
        route: FindingRoute::Secrets,
    }]
}

fn ec2_stopped_instances_needing_review(ctx: &FindingContext) -> Vec<Finding> {
    if ctx.account_overview.is_none() {
        return Vec::new();
    }

    let stopped_instances_needing_review = ctx
        .ec2_instances
        .iter()
        .filter(|instance| instance.needs_stopped_review())
        .collect::<Vec<_>>();

    if stopped_instances_needing_review.is_empty() {
        return Vec::new();
    }

    let names = stopped_instances_needing_review
        .iter()
        .map(|instance| instance.label())
        .collect::<Vec<_>>();

    vec![Finding {
        severity: FindingSeverity::Medium,
        category: FindingCategory::Hygiene,
        service: "EC2".into(),
        region: ctx.region_label.to_string(),
        summary: format!(
            "{} stopped instance(s) still look important: {}",
            stopped_instances_needing_review.len(),
            sample_list(&names)
        ),
        next_step: "Open EC2 and review stopped instances with public IPs or production-like names"
            .into(),
        route: FindingRoute::Ec2,
    }]
}

fn ec2_tag_coverage_gaps(ctx: &FindingContext) -> Vec<Finding> {
    if ctx.account_overview.is_none() {
        return Vec::new();
    }

    let instances_with_tag_gaps = ctx
        .ec2_instances
        .iter()
        .filter(|instance| instance.has_tag_coverage_gap())
        .collect::<Vec<_>>();

    if instances_with_tag_gaps.is_empty() {
        return Vec::new();
    }

    let labels = instances_with_tag_gaps
        .iter()
        .map(|instance| {
            let label = instance.label();
            let missing = instance
                .missing_required_tags()
                .unwrap_or_default()
                .join("/");
            format!("{label} ({missing})")
        })
        .collect::<Vec<_>>();

    vec![Finding {
        severity: FindingSeverity::Medium,
        category: FindingCategory::Hygiene,
        service: "EC2".into(),
        region: ctx.region_label.to_string(),
        summary: format!(
            "{} EC2 instance(s) are missing Name, Owner, or Environment tags: {}",
            instances_with_tag_gaps.len(),
            sample_list(&labels)
        ),
        next_step: "Open EC2 and add Name, Owner, or Environment tags to unmanaged instances"
            .into(),
        route: FindingRoute::Ec2,
    }]
}

/// The tag that attributes a resource to a person or team.
const OWNER_TAG: &str = "Owner";

/// Resources with no `Owner` tag, across every service that carries tags.
///
/// EC2 has its own stricter rule covering Name/Owner/Environment, so it is not
/// repeated here. Services whose tags could not be read are skipped rather than
/// reported as unowned, since a failed tag lookup is not evidence of anything.
fn resources_missing_owner_tag(ctx: &FindingContext) -> Vec<Finding> {
    if ctx.account_overview.is_none() {
        return Vec::new();
    }

    let unowned = |tags: &Tags| tags.is_available() && tags.value(OWNER_TAG).is_none();

    let mut groups: Vec<(&str, FindingRoute, Vec<String>)> = vec![
        (
            "RDS",
            FindingRoute::Rds,
            ctx.rds_instances
                .iter()
                .filter(|item| unowned(&item.tags))
                .map(|item| item.identifier.clone())
                .collect(),
        ),
        (
            "Secrets Manager",
            FindingRoute::Secrets,
            ctx.secrets
                .iter()
                .filter(|item| unowned(&item.tags))
                .map(|item| item.name.clone())
                .collect(),
        ),
        (
            "API Gateway",
            FindingRoute::Apigateway,
            ctx.apigateway_apis
                .iter()
                .filter(|item| unowned(&item.tags))
                .map(|item| item.name.clone())
                .collect(),
        ),
        (
            "VPC",
            FindingRoute::Vpc,
            ctx.vpcs
                .iter()
                .filter(|item| unowned(&item.tags))
                .map(|item| item.vpc_id.clone())
                .collect(),
        ),
        (
            "Security Groups",
            FindingRoute::SecurityGroups,
            ctx.security_groups
                .iter()
                .filter(|item| unowned(&item.tags))
                .map(|item| item.name.clone())
                .collect(),
        ),
    ];

    groups.retain(|(_, _, labels)| !labels.is_empty());

    groups
        .into_iter()
        .map(|(service, route, labels)| Finding {
            severity: FindingSeverity::Medium,
            category: FindingCategory::Hygiene,
            service: service.into(),
            region: ctx.region_label.to_string(),
            summary: format!(
                "{} {service} resource(s) have no {OWNER_TAG} tag: {}",
                labels.len(),
                sample_list(&labels)
            ),
            next_step: format!(
                "Add an {OWNER_TAG} tag so these {service} resources can be attributed to a team"
            ),
            route,
        })
        .collect()
}

fn ec2_sustained_low_cpu(ctx: &FindingContext) -> Vec<Finding> {
    if ctx.account_overview.is_none() {
        return Vec::new();
    }

    let low_cpu_instances = ctx
        .ec2_instances
        .iter()
        .filter(|instance| instance.has_sustained_low_cpu())
        .collect::<Vec<_>>();

    if low_cpu_instances.is_empty() {
        return Vec::new();
    }

    let labels = low_cpu_instances
        .iter()
        .map(|instance| {
            let label = instance.label();
            format!("{label} ({})", instance.formatted_avg_cpu())
        })
        .collect::<Vec<_>>();

    vec![Finding {
        severity: FindingSeverity::Medium,
        category: FindingCategory::Waste,
        service: "EC2".into(),
        region: ctx.region_label.to_string(),
        summary: format!(
            "{} running EC2 instance(s) averaged below {:.1}% CPU over the last {} days: {}",
            low_cpu_instances.len(),
            Ec2InstanceInfo::LOW_CPU_THRESHOLD_PERCENT,
            Ec2InstanceInfo::LOW_CPU_LOOKBACK_DAYS,
            sample_list(&labels)
        ),
        next_step: format!(
            "Open EC2 and review running instances averaging below {:.1}% CPU over the last {} days",
            Ec2InstanceInfo::LOW_CPU_THRESHOLD_PERCENT,
            Ec2InstanceInfo::LOW_CPU_LOOKBACK_DAYS
        ),
        route: FindingRoute::Ec2,
    }]
}

fn ec2_stopped_instances_unused(ctx: &FindingContext) -> Vec<Finding> {
    let Some(overview) = ctx.account_overview else {
        return Vec::new();
    };

    let plain_stopped_instances = ctx
        .ec2_instances
        .iter()
        .filter(|instance| instance.is_stopped() && !instance.needs_stopped_review())
        .count();

    if plain_stopped_instances > 0 {
        return vec![Finding {
            severity: FindingSeverity::Medium,
            category: FindingCategory::Waste,
            service: "EC2".into(),
            region: ctx.region_label.to_string(),
            summary: format!("{plain_stopped_instances} stopped instance(s) may be unused"),
            next_step: "Review stopped instances for cleanup or restart".into(),
            route: FindingRoute::Ec2,
        }];
    }

    if overview.ec2_stopped > 0 && ctx.ec2_instances.is_empty() {
        return vec![Finding {
            severity: FindingSeverity::Medium,
            category: FindingCategory::Waste,
            service: "EC2".into(),
            region: overview.region.clone(),
            summary: format!("{} stopped instance(s) may be unused", overview.ec2_stopped),
            next_step: "Review stopped instances for cleanup or restart".into(),
            route: FindingRoute::Ec2,
        }];
    }

    Vec::new()
}

fn security_groups_sensitive_public_ports(ctx: &FindingContext) -> Vec<Finding> {
    let sensitive_port_groups = ctx
        .security_groups
        .iter()
        .filter(|sg| !sg.sensitive_public_ports.is_empty())
        .count();

    if sensitive_port_groups == 0 {
        return Vec::new();
    }

    let sensitive_ports = ctx
        .security_groups
        .iter()
        .flat_map(|sg| sg.sensitive_public_ports.iter().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|port| port.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    vec![Finding {
        severity: FindingSeverity::High,
        category: FindingCategory::Hygiene,
        service: "Security Groups".into(),
        region: ctx.region_label.to_string(),
        summary: format!(
            "{sensitive_port_groups} security group(s) expose sensitive ports publicly ({sensitive_ports})"
        ),
        next_step: "Review public access on sensitive ports and narrow ingress".into(),
        route: FindingRoute::SecurityGroups,
    }]
}

fn security_groups_open_to_world(ctx: &FindingContext) -> Vec<Finding> {
    let open_to_world = ctx
        .security_groups
        .iter()
        .filter(|sg| sg.open_to_world && sg.sensitive_public_ports.is_empty())
        .count();

    if open_to_world == 0 {
        return Vec::new();
    }

    vec![Finding {
        severity: FindingSeverity::Medium,
        category: FindingCategory::Hygiene,
        service: "Security Groups".into(),
        region: ctx.region_label.to_string(),
        summary: format!("{open_to_world} security group(s) are open to the world"),
        next_step: "Review public ingress rules and narrow access".into(),
        route: FindingRoute::SecurityGroups,
    }]
}

fn apigateway_generic_or_stale_apis(ctx: &FindingContext) -> Vec<Finding> {
    let apis_needing_review = ctx
        .apigateway_apis
        .iter()
        .filter(|api| api.needs_review())
        .collect::<Vec<_>>();

    if apis_needing_review.is_empty() {
        return Vec::new();
    }

    let labels = apis_needing_review
        .iter()
        .map(|api| {
            let signals = api.review_signals().join("/");
            format!("{} ({signals})", api.name)
        })
        .collect::<Vec<_>>();

    vec![Finding {
        severity: FindingSeverity::Medium,
        category: FindingCategory::Waste,
        service: "API Gateway".into(),
        region: ctx.region_label.to_string(),
        summary: format!(
            "{} API Gateway API(s) look generic or stale: {}",
            apis_needing_review.len(),
            sample_list(&labels)
        ),
        next_step: "Open API Gateway and review generic or year-old APIs for ownership and cleanup"
            .into(),
        route: FindingRoute::Apigateway,
    }]
}

fn sqs_queues_without_dlq(ctx: &FindingContext) -> Vec<Finding> {
    let queues_without_dlq = ctx.sqs_queues_data.iter().filter(|q| !q.has_dlq).count();

    if queues_without_dlq == 0 {
        return Vec::new();
    }

    vec![Finding {
        severity: FindingSeverity::Medium,
        category: FindingCategory::Hygiene,
        service: "SQS".into(),
        region: ctx.region_label.to_string(),
        summary: format!("{queues_without_dlq} queue(s) do not have a DLQ configured"),
        next_step: "Review queues without DLQs and add redrive policies where needed".into(),
        route: FindingRoute::Sqs,
    }]
}

fn sqs_queue_backlog(ctx: &FindingContext) -> Vec<Finding> {
    let backlog_queues = ctx
        .sqs_queues_data
        .iter()
        .filter(|queue| queue.has_backlog_incident())
        .collect::<Vec<_>>();

    if backlog_queues.is_empty() {
        return Vec::new();
    }

    let labels = backlog_queues
        .iter()
        .map(|queue| {
            let signals = queue.backlog_signals().join("/");
            format!("{} ({signals})", queue.name)
        })
        .collect::<Vec<_>>();

    vec![Finding {
        severity: FindingSeverity::High,
        category: FindingCategory::Incident,
        service: "SQS".into(),
        region: ctx.region_label.to_string(),
        summary: format!(
            "{} queue(s) have high backlog or stuck work: {}",
            backlog_queues.len(),
            sample_list(&labels)
        ),
        next_step: format!(
            "Open SQS and inspect queues with >= {} visible or >= {} in-flight messages",
            SqsQueueInfo::HIGH_VISIBLE_THRESHOLD,
            SqsQueueInfo::HIGH_IN_FLIGHT_THRESHOLD
        ),
        route: FindingRoute::Sqs,
    }]
}

fn rds_instances_not_available(ctx: &FindingContext) -> Vec<Finding> {
    let rds_not_available = ctx
        .rds_instances
        .iter()
        .filter(|db| db.status != "available")
        .count();

    if rds_not_available == 0 {
        return Vec::new();
    }

    vec![Finding {
        severity: FindingSeverity::High,
        category: FindingCategory::Incident,
        service: "RDS".into(),
        region: ctx.region_label.to_string(),
        summary: format!("{rds_not_available} RDS instance(s) are not available"),
        next_step: "Open RDS and investigate instance status and recovery path".into(),
        route: FindingRoute::Rds,
    }]
}

fn rds_single_az_production_like(ctx: &FindingContext) -> Vec<Finding> {
    let single_az_review_instances = ctx
        .rds_instances
        .iter()
        .filter(|db| db.needs_single_az_review())
        .collect::<Vec<_>>();

    if single_az_review_instances.is_empty() {
        return Vec::new();
    }

    let identifiers = single_az_review_instances
        .iter()
        .map(|db| db.identifier.clone())
        .collect::<Vec<_>>();

    vec![Finding {
        severity: FindingSeverity::Medium,
        category: FindingCategory::Hygiene,
        service: "RDS".into(),
        region: ctx.region_label.to_string(),
        summary: format!(
            "{} single-AZ RDS instance(s) look production-like: {}",
            single_az_review_instances.len(),
            sample_list(&identifiers)
        ),
        next_step: "Open RDS and review production-like single-AZ databases for Multi-AZ coverage"
            .into(),
        route: FindingRoute::Rds,
    }]
}

fn vpc_default_vpcs_present(ctx: &FindingContext) -> Vec<Finding> {
    let default_vpcs = ctx.vpcs.iter().filter(|vpc| vpc.is_default).count();

    if default_vpcs == 0 {
        return Vec::new();
    }

    vec![Finding {
        severity: FindingSeverity::Medium,
        category: FindingCategory::Hygiene,
        service: "VPC".into(),
        region: ctx.region_label.to_string(),
        summary: format!("{default_vpcs} default VPC(s) are still present"),
        next_step: "Review default VPC usage and remove or restrict it if unnecessary".into(),
        route: FindingRoute::Vpc,
    }]
}

fn load_balancers_zero_healthy_targets(ctx: &FindingContext) -> Vec<Finding> {
    let load_balancers_with_zero_healthy_targets = ctx
        .load_balancers
        .iter()
        .filter(|lb| lb.has_zero_healthy_targets())
        .collect::<Vec<_>>();

    if load_balancers_with_zero_healthy_targets.is_empty() {
        return Vec::new();
    }

    let names = load_balancers_with_zero_healthy_targets
        .iter()
        .map(|lb| lb.name.clone())
        .collect::<Vec<_>>();

    vec![Finding {
        severity: FindingSeverity::High,
        category: FindingCategory::Incident,
        service: "Load Balancers".into(),
        region: ctx.region_label.to_string(),
        summary: format!(
            "{} load balancer(s) have target groups but zero healthy targets: {}",
            load_balancers_with_zero_healthy_targets.len(),
            sample_list(&names)
        ),
        next_step: "Open load balancers and restore healthy registered targets behind the listener"
            .into(),
        route: FindingRoute::LoadBalancers,
    }]
}

fn load_balancers_no_active_targets(ctx: &FindingContext) -> Vec<Finding> {
    let load_balancers_with_no_active_targets = ctx
        .load_balancers
        .iter()
        .filter(|lb| lb.has_no_active_targets() && !lb.has_zero_healthy_targets())
        .collect::<Vec<_>>();

    if load_balancers_with_no_active_targets.is_empty() {
        return Vec::new();
    }

    let labels = load_balancers_with_no_active_targets
        .iter()
        .map(|lb| {
            let signals = lb.review_signals().join("/");
            format!("{} ({signals})", lb.name)
        })
        .collect::<Vec<_>>();

    vec![Finding {
        severity: FindingSeverity::Medium,
        category: FindingCategory::Waste,
        service: "Load Balancers".into(),
        region: ctx.region_label.to_string(),
        summary: format!(
            "{} load balancer(s) have no active target path: {}",
            load_balancers_with_no_active_targets.len(),
            sample_list(&labels)
        ),
        next_step:
            "Open load balancers and review listeners with no target groups or no registered targets"
                .into(),
        route: FindingRoute::LoadBalancers,
    }]
}

fn lambda_high_memory_functions(ctx: &FindingContext) -> Vec<Finding> {
    let high_memory_functions = ctx
        .lambda_functions
        .iter()
        .filter(|f| f.has_high_memory())
        .count();

    if high_memory_functions == 0 {
        return Vec::new();
    }

    vec![Finding {
        severity: FindingSeverity::Medium,
        category: FindingCategory::Waste,
        service: "Lambda".into(),
        region: ctx.region_label.to_string(),
        summary: format!(
            "{high_memory_functions} function(s) have memory >= {} MB",
            LambdaFunctionInfo::HIGH_MEMORY_THRESHOLD_MB
        ),
        next_step: "Review high-memory Lambda functions for right-sizing".into(),
        route: FindingRoute::Lambda,
    }]
}

fn lambda_stale_functions(ctx: &FindingContext) -> Vec<Finding> {
    let stale_functions = ctx.lambda_functions.iter().filter(|f| f.is_stale()).count();

    if stale_functions == 0 {
        return Vec::new();
    }

    vec![Finding {
        severity: FindingSeverity::Medium,
        category: FindingCategory::Waste,
        service: "Lambda".into(),
        region: ctx.region_label.to_string(),
        summary: format!(
            "{stale_functions} function(s) have not been modified in {}+ days",
            LambdaFunctionInfo::STALE_DEPLOY_DAYS
        ),
        next_step: "Review stale Lambda functions for ownership or cleanup".into(),
        route: FindingRoute::Lambda,
    }]
}

/// Every finding rule, in evaluation order. The order matters: the final sort
/// is stable, so rules that tie on severity, category, and service keep the
/// relative order they appear in here.
pub const FINDING_RULES: &[fn(&FindingContext) -> Vec<Finding>] = &[
    cloudwatch_alarms_in_alarm,
    cloudwatch_alarm_coverage_gaps,
    target_groups_zero_healthy_targets,
    target_groups_unhealthy_targets,
    target_groups_orphaned,
    secrets_production_rotation_disabled,
    secrets_rotation_disabled,
    secrets_stale_rotation,
    ec2_stopped_instances_needing_review,
    ec2_tag_coverage_gaps,
    resources_missing_owner_tag,
    ec2_sustained_low_cpu,
    ec2_stopped_instances_unused,
    security_groups_sensitive_public_ports,
    security_groups_open_to_world,
    apigateway_generic_or_stale_apis,
    sqs_queues_without_dlq,
    sqs_queue_backlog,
    rds_instances_not_available,
    rds_single_az_production_like,
    vpc_default_vpcs_present,
    load_balancers_zero_healthy_targets,
    load_balancers_no_active_targets,
    lambda_high_memory_functions,
    lambda_stale_functions,
];

/// Run every rule and return the findings ordered by severity, then category,
/// then service.
pub fn build_findings(ctx: &FindingContext) -> Vec<Finding> {
    let mut findings = FINDING_RULES
        .iter()
        .flat_map(|rule| rule(ctx))
        .collect::<Vec<_>>();

    findings.sort_by(|a, b| {
        a.severity
            .rank()
            .cmp(&b.severity.rank())
            .then_with(|| a.category.as_str().cmp(b.category.as_str()))
            .then_with(|| a.service.cmp(&b.service))
    });

    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::cloudwatch::CloudWatchSummary;
    use crate::models::rds::RdsSummary;
    use crate::models::secrets::SecretsSummary;
    use crate::models::service_status::ServiceStatus;

    fn overview() -> AccountOverview {
        AccountOverview {
            account_id: "123456789012".into(),
            identity_kind: "user".into(),
            identity_name: "tester".into(),
            role_name: None,
            region: "us-east-1".into(),
            ec2_running: 0,
            ec2_stopped: 0,
            ecs_clusters: 0,
            ecs_services: 0,
            target_groups_total: 0,
            target_groups_unhealthy: 0,
            rds_status: RdsSummary {
                status: ServiceStatus::Ok,
                total: 0,
                available: 0,
            },
            lambda_functions: 0,
            lambda_status: ServiceStatus::Ok,
            apigw_rest_apis: 0,
            apigw_http_apis: 0,
            apigw_status: ServiceStatus::Ok,
            sqs_queues: 0,
            sqs_dlqs: 0,
            sqs_status: ServiceStatus::Ok,
            vpc_count: 0,
            subnet_count: 0,
            vpc_status: ServiceStatus::Ok,
            alarms: CloudWatchSummary {
                status: ServiceStatus::Ok,
                total_alarms: 0,
                alarms_in_alarm: 0,
            },
            secrets: SecretsSummary {
                status: ServiceStatus::Ok,
                total: 0,
                rotation_disabled: 0,
            },
        }
    }

    fn ctx<'a>(account_overview: Option<&'a AccountOverview>) -> FindingContext<'a> {
        FindingContext {
            account_overview,
            region_label: "us-east-1",
            cloudwatch_alarms: &[],
            secrets: &[],
            ec2_instances: &[],
            target_groups: &[],
            load_balancers: &[],
            apigateway_apis: &[],
            sqs_queues_data: &[],
            rds_instances: &[],
            security_groups: &[],
            vpcs: &[],
            lambda_functions: &[],
        }
    }

    fn alarm(name: &str, state: &str, namespace: &str) -> CloudWatchAlarm {
        CloudWatchAlarm {
            name: name.into(),
            state: state.into(),
            namespace: namespace.into(),
            metric: "CPUUtilization".into(),
        }
    }

    fn target_group(name: &str, total: usize, unhealthy: usize, attached: bool) -> TargetGroupInfo {
        TargetGroupInfo {
            arn: format!("arn:aws:elasticloadbalancing:::targetgroup/{name}"),
            name: name.into(),
            protocol: "HTTP".into(),
            port: 80,
            target_type: "instance".into(),
            attached_load_balancer_arns: if attached {
                vec!["arn:aws:elasticloadbalancing:::loadbalancer/app".into()]
            } else {
                Vec::new()
            },
            total_targets: total,
            unhealthy_targets: unhealthy,
        }
    }

    fn security_group(open_to_world: bool, sensitive_ports: Vec<i32>) -> SecurityGroupInfo {
        SecurityGroupInfo {
            id: "sg-1".into(),
            name: "web".into(),
            vpc_id: "vpc-1".into(),
            inbound_rules: 1,
            outbound_rules: 1,
            open_to_world,
            sensitive_public_ports: sensitive_ports,
            tags: Tags::loaded([("Owner", "platform")]),
        }
    }

    fn lambda(memory_mb: i32) -> LambdaFunctionInfo {
        LambdaFunctionInfo {
            name: "fn".into(),
            region: "us-east-1".into(),
            runtime: "python3.12".into(),
            memory_mb,
            timeout_sec: 30,
            last_modified: "2026-07-01T00:00:00.000+0000".into(),
        }
    }

    fn vpc(is_default: bool) -> VpcInfo {
        VpcInfo {
            vpc_id: "vpc-1".into(),
            cidr: "10.0.0.0/16".into(),
            state: "available".into(),
            is_default,
            subnet_count: 3,
            tags: Tags::loaded([("Owner", "platform")]),
        }
    }

    fn vpc_with_tags(vpc_id: &str, tags: Tags) -> VpcInfo {
        VpcInfo {
            vpc_id: vpc_id.into(),
            cidr: "10.0.0.0/16".into(),
            state: "available".into(),
            is_default: false,
            subnet_count: 1,
            tags,
        }
    }

    #[test]
    fn a_resource_without_an_owner_tag_is_reported() {
        let overview = overview();
        let vpcs = vec![vpc_with_tags("vpc-unowned", Tags::empty())];
        let mut context = ctx(Some(&overview));
        context.vpcs = &vpcs;

        let findings = resources_missing_owner_tag(&context);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].service, "VPC");
        assert!(findings[0].summary.contains("vpc-unowned"));
    }

    #[test]
    fn a_resource_with_an_owner_tag_is_not_reported() {
        let overview = overview();
        let vpcs = vec![vpc_with_tags(
            "vpc-1",
            Tags::loaded([("Owner", "platform")]),
        )];
        let mut context = ctx(Some(&overview));
        context.vpcs = &vpcs;

        assert!(resources_missing_owner_tag(&context).is_empty());
    }

    /// A failed tag lookup is not evidence that a resource is unowned, so it
    /// must not be reported as such.
    #[test]
    fn a_resource_whose_tags_could_not_be_read_is_not_reported() {
        let overview = overview();
        let vpcs = vec![vpc_with_tags("vpc-unknown", Tags::Unavailable)];
        let mut context = ctx(Some(&overview));
        context.vpcs = &vpcs;

        assert!(resources_missing_owner_tag(&context).is_empty());
    }

    /// A blank Owner value attributes the resource to nobody, so it counts as
    /// missing rather than present.
    #[test]
    fn a_blank_owner_tag_counts_as_missing() {
        let overview = overview();
        let vpcs = vec![vpc_with_tags("vpc-blank", Tags::loaded([("Owner", "  ")]))];
        let mut context = ctx(Some(&overview));
        context.vpcs = &vpcs;

        assert_eq!(resources_missing_owner_tag(&context).len(), 1);
    }

    #[test]
    fn each_service_reports_its_own_finding() {
        let overview = overview();
        let vpcs = vec![vpc_with_tags("vpc-1", Tags::empty())];
        let groups = vec![security_group(false, vec![])];
        let mut context = ctx(Some(&overview));
        context.vpcs = &vpcs;
        context.security_groups = &groups;

        // The security group fixture carries an Owner tag, so only VPC reports.
        let findings = resources_missing_owner_tag(&context);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].service, "VPC");
    }

    #[test]
    fn sample_list_under_limit() {
        assert_eq!(sample_list(&["a", "b"]), "a, b");
    }

    #[test]
    fn sample_list_at_limit() {
        assert_eq!(sample_list(&["a", "b", "c"]), "a, b, c");
    }

    #[test]
    fn sample_list_over_limit() {
        assert_eq!(sample_list(&["a", "b", "c", "d", "e"]), "a, b, c (+2 more)");
    }

    #[test]
    fn sample_list_empty() {
        let empty: [&str; 0] = [];
        assert_eq!(sample_list(&empty), "");
    }

    #[test]
    fn alarms_in_alarm_rule_fires_on_live_alarms() {
        let ov = overview();
        let alarms = vec![
            alarm("cpu-high", "ALARM", "AWS/EC2"),
            alarm("disk-ok", "OK", "AWS/EC2"),
        ];
        let mut c = ctx(Some(&ov));
        c.cloudwatch_alarms = &alarms;

        let found = cloudwatch_alarms_in_alarm(&c);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].severity, FindingSeverity::High);
        assert_eq!(found[0].category, FindingCategory::Incident);
        assert_eq!(found[0].route, FindingRoute::CloudWatch);
        assert_eq!(found[0].summary, "1 alarm(s) are in ALARM: cpu-high");
    }

    #[test]
    fn alarms_in_alarm_rule_falls_back_to_overview_counts() {
        let mut ov = overview();
        ov.alarms.alarms_in_alarm = 4;
        let c = ctx(Some(&ov));

        let found = cloudwatch_alarms_in_alarm(&c);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].summary, "4 alarm(s) are currently in ALARM");
        assert_eq!(found[0].region, "us-east-1");
    }

    #[test]
    fn alarms_in_alarm_rule_is_silent_without_overview() {
        let alarms = vec![alarm("cpu-high", "ALARM", "AWS/EC2")];
        let mut c = ctx(None);
        c.cloudwatch_alarms = &alarms;

        assert!(cloudwatch_alarms_in_alarm(&c).is_empty());
    }

    #[test]
    fn alarm_coverage_gap_rule_reports_uncovered_namespaces() {
        let mut ov = overview();
        ov.ec2_running = 2;
        ov.lambda_functions = 3;
        let alarms = vec![alarm("cpu-high", "OK", "AWS/EC2")];
        let mut c = ctx(Some(&ov));
        c.cloudwatch_alarms = &alarms;

        let found = cloudwatch_alarm_coverage_gaps(&c);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].severity, FindingSeverity::Medium);
        assert_eq!(found[0].category, FindingCategory::Hygiene);
        assert_eq!(
            found[0].summary,
            "1 deployed service area(s) appear to have no CloudWatch alarm coverage: Lambda (3 functions)"
        );
    }

    #[test]
    fn target_group_rules_split_zero_healthy_from_partially_unhealthy() {
        let ov = overview();
        let groups = vec![
            target_group("all-down", 2, 2, true),
            target_group("degraded", 4, 1, true),
        ];
        let mut c = ctx(Some(&ov));
        c.target_groups = &groups;

        let zero = target_groups_zero_healthy_targets(&c);
        assert_eq!(zero.len(), 1);
        assert_eq!(zero[0].severity, FindingSeverity::High);
        assert_eq!(zero[0].route, FindingRoute::TargetGroups);
        assert_eq!(
            zero[0].summary,
            "1 target group(s) have zero healthy targets"
        );

        let unhealthy = target_groups_unhealthy_targets(&c);
        assert_eq!(unhealthy.len(), 1);
        assert_eq!(
            unhealthy[0].summary,
            "1 target group(s) have unhealthy targets"
        );
    }

    #[test]
    fn security_group_rules_do_not_double_report() {
        let groups = vec![security_group(true, vec![22, 3389])];
        let mut c = ctx(None);
        c.security_groups = &groups;

        let sensitive = security_groups_sensitive_public_ports(&c);
        assert_eq!(sensitive.len(), 1);
        assert_eq!(sensitive[0].severity, FindingSeverity::High);
        assert_eq!(sensitive[0].route, FindingRoute::SecurityGroups);
        assert_eq!(
            sensitive[0].summary,
            "1 security group(s) expose sensitive ports publicly (22, 3389)"
        );

        assert!(security_groups_open_to_world(&c).is_empty());
    }

    #[test]
    fn lambda_high_memory_rule_fires_at_threshold() {
        let functions = vec![
            lambda(LambdaFunctionInfo::HIGH_MEMORY_THRESHOLD_MB),
            lambda(128),
        ];
        let mut c = ctx(None);
        c.lambda_functions = &functions;

        let found = lambda_high_memory_functions(&c);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].severity, FindingSeverity::Medium);
        assert_eq!(found[0].category, FindingCategory::Waste);
        assert_eq!(found[0].route, FindingRoute::Lambda);
    }

    #[test]
    fn default_vpc_rule_only_counts_default_vpcs() {
        let vpcs = vec![vpc(true), vpc(false)];
        let mut c = ctx(None);
        c.vpcs = &vpcs;

        let found = vpc_default_vpcs_present(&c);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].route, FindingRoute::Vpc);
        assert_eq!(found[0].summary, "1 default VPC(s) are still present");

        let empty = ctx(None);
        assert!(vpc_default_vpcs_present(&empty).is_empty());
    }

    #[test]
    fn build_findings_sorts_high_severity_first() {
        let ov = overview();
        let alarms = vec![alarm("cpu-high", "ALARM", "AWS/EC2")];
        let vpcs = vec![vpc(true)];
        let mut c = ctx(Some(&ov));
        c.cloudwatch_alarms = &alarms;
        c.vpcs = &vpcs;

        let found = build_findings(&c);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].severity, FindingSeverity::High);
        assert_eq!(found[1].severity, FindingSeverity::Medium);
    }

    #[test]
    fn build_findings_is_empty_without_data() {
        assert!(build_findings(&ctx(None)).is_empty());
    }
}
