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

/// The fixed attributes every finding from one rule shares.
///
/// Rules emit one finding per offending resource, so this carries the parts
/// that do not vary and leaves each finding to supply its own resource id and
/// summary.
struct Rule {
    id: &'static str,
    severity: FindingSeverity,
    category: FindingCategory,
    service: &'static str,
    route: FindingRoute,
}

impl Rule {
    fn resource(
        &self,
        region: &str,
        resource_id: impl Into<String>,
        summary: String,
        next_step: impl Into<String>,
    ) -> Finding {
        Finding {
            rule: self.id,
            resource_id: Some(resource_id.into()),
            severity: self.severity,
            category: self.category,
            service: self.service.into(),
            region: region.to_string(),
            summary,
            next_step: next_step.into(),
            route: self.route,
        }
    }

    /// A finding with no single underlying resource.
    ///
    /// Used by the account-level rollups and by the overview fallbacks, which
    /// fire from summary counters when the detail list could not be fetched and
    /// so have no resources to point at.
    fn aggregate(&self, region: &str, summary: String, next_step: impl Into<String>) -> Finding {
        Finding {
            rule: self.id,
            resource_id: None,
            severity: self.severity,
            category: self.category,
            service: self.service.into(),
            region: region.to_string(),
            summary,
            next_step: next_step.into(),
            route: self.route,
        }
    }
}

const CLOUDWATCH_ALARMS_IN_ALARM: Rule = Rule {
    id: "cloudwatch_alarms_in_alarm",
    severity: FindingSeverity::High,
    category: FindingCategory::Incident,
    service: "CloudWatch",
    route: FindingRoute::CloudWatch,
};

fn cloudwatch_alarms_in_alarm(ctx: &FindingContext) -> Vec<Finding> {
    let Some(overview) = ctx.account_overview else {
        return Vec::new();
    };

    let alarming = ctx
        .cloudwatch_alarms
        .iter()
        .filter(|alarm| alarm.state == "ALARM")
        .collect::<Vec<_>>();

    if !alarming.is_empty() {
        return alarming
            .iter()
            .map(|alarm| {
                CLOUDWATCH_ALARMS_IN_ALARM.resource(
                    ctx.region_label,
                    &alarm.name,
                    format!("Alarm {} is in ALARM ({})", alarm.name, alarm.metric),
                    "Open CloudWatch and inspect this alarm",
                )
            })
            .collect();
    }

    if overview.alarms.alarms_in_alarm > 0 {
        return vec![CLOUDWATCH_ALARMS_IN_ALARM.aggregate(
            &overview.region,
            format!(
                "{} alarm(s) are currently in ALARM",
                overview.alarms.alarms_in_alarm
            ),
            "Open CloudWatch and inspect failing alarms",
        )];
    }

    Vec::new()
}

const CLOUDWATCH_ALARM_COVERAGE_GAPS: Rule = Rule {
    id: "cloudwatch_alarm_coverage_gaps",
    severity: FindingSeverity::Medium,
    category: FindingCategory::Hygiene,
    service: "CloudWatch",
    route: FindingRoute::CloudWatch,
};

/// Stays aggregate: its subjects are service areas derived from account
/// counters, not resources with ids of their own.
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

    vec![CLOUDWATCH_ALARM_COVERAGE_GAPS.aggregate(
        ctx.region_label,
        format!(
            "{} deployed service area(s) appear to have no CloudWatch alarm coverage: {}",
            coverage_gaps.len(),
            sample_list(&coverage_gaps)
        ),
        "Open CloudWatch and add alarms for deployed services without namespace coverage",
    )]
}

const TARGET_GROUPS_ZERO_HEALTHY: Rule = Rule {
    id: "target_groups_zero_healthy_targets",
    severity: FindingSeverity::High,
    category: FindingCategory::Incident,
    service: "Target Groups",
    route: FindingRoute::TargetGroups,
};

fn target_groups_zero_healthy_targets(ctx: &FindingContext) -> Vec<Finding> {
    if ctx.account_overview.is_none() {
        return Vec::new();
    }

    ctx.target_groups
        .iter()
        .filter(|tg| tg.has_zero_healthy_targets())
        .map(|tg| {
            TARGET_GROUPS_ZERO_HEALTHY.resource(
                ctx.region_label,
                &tg.arn,
                format!(
                    "Target group {} has zero healthy targets ({} registered)",
                    tg.name, tg.total_targets
                ),
                "Open target groups and restore at least one healthy target",
            )
        })
        .collect()
}

const TARGET_GROUPS_UNHEALTHY: Rule = Rule {
    id: "target_groups_unhealthy_targets",
    severity: FindingSeverity::High,
    category: FindingCategory::Incident,
    service: "Target Groups",
    route: FindingRoute::TargetGroups,
};

fn target_groups_unhealthy_targets(ctx: &FindingContext) -> Vec<Finding> {
    let Some(overview) = ctx.account_overview else {
        return Vec::new();
    };

    let partially_unhealthy = ctx
        .target_groups
        .iter()
        .filter(|tg| tg.unhealthy_targets > 0 && !tg.has_zero_healthy_targets())
        .collect::<Vec<_>>();

    if !partially_unhealthy.is_empty() {
        return partially_unhealthy
            .iter()
            .map(|tg| {
                TARGET_GROUPS_UNHEALTHY.resource(
                    ctx.region_label,
                    &tg.arn,
                    format!(
                        "Target group {} has {} unhealthy target(s) of {}",
                        tg.name, tg.unhealthy_targets, tg.total_targets
                    ),
                    "Open target groups and inspect unhealthy target health",
                )
            })
            .collect();
    }

    if overview.target_groups_unhealthy > 0 && ctx.target_groups.is_empty() {
        return vec![TARGET_GROUPS_UNHEALTHY.aggregate(
            &overview.region,
            format!(
                "{} target group(s) have unhealthy targets",
                overview.target_groups_unhealthy
            ),
            "Open target groups and inspect target health",
        )];
    }

    Vec::new()
}

const TARGET_GROUPS_ORPHANED: Rule = Rule {
    id: "target_groups_orphaned",
    severity: FindingSeverity::Medium,
    category: FindingCategory::Waste,
    service: "Target Groups",
    route: FindingRoute::TargetGroups,
};

fn target_groups_orphaned(ctx: &FindingContext) -> Vec<Finding> {
    if ctx.account_overview.is_none() {
        return Vec::new();
    }

    ctx.target_groups
        .iter()
        .filter(|tg| tg.is_orphan_candidate())
        .map(|tg| {
            TARGET_GROUPS_ORPHANED.resource(
                ctx.region_label,
                &tg.arn,
                format!(
                    "Target group {} has no load balancer attachment and no registered targets",
                    tg.name
                ),
                "Open target groups and review orphan groups for cleanup or reattachment",
            )
        })
        .collect()
}

const SECRETS_PRODUCTION_ROTATION_DISABLED: Rule = Rule {
    id: "secrets_production_rotation_disabled",
    severity: FindingSeverity::High,
    category: FindingCategory::Hygiene,
    service: "Secrets Manager",
    route: FindingRoute::Secrets,
};

fn secrets_production_rotation_disabled(ctx: &FindingContext) -> Vec<Finding> {
    if ctx.account_overview.is_none() {
        return Vec::new();
    }

    ctx.secrets
        .iter()
        .filter(|secret| secret.needs_rotation_review())
        .map(|secret| {
            SECRETS_PRODUCTION_ROTATION_DISABLED.resource(
                ctx.region_label,
                &secret.name,
                format!(
                    "Production-like secret {} does not have rotation enabled",
                    secret.name
                ),
                "Open Secrets Manager and enable rotation on production-like secrets",
            )
        })
        .collect()
}

const SECRETS_ROTATION_DISABLED: Rule = Rule {
    id: "secrets_rotation_disabled",
    severity: FindingSeverity::Medium,
    category: FindingCategory::Hygiene,
    service: "Secrets Manager",
    route: FindingRoute::Secrets,
};

fn secrets_rotation_disabled(ctx: &FindingContext) -> Vec<Finding> {
    let Some(overview) = ctx.account_overview else {
        return Vec::new();
    };

    let rotation_disabled = ctx
        .secrets
        .iter()
        .filter(|secret| secret.rotation_disabled() && !secret.needs_rotation_review())
        .collect::<Vec<_>>();

    if !rotation_disabled.is_empty() {
        return rotation_disabled
            .iter()
            .map(|secret| {
                SECRETS_ROTATION_DISABLED.resource(
                    ctx.region_label,
                    &secret.name,
                    format!("Secret {} does not have rotation enabled", secret.name),
                    "Review secrets that should rotate automatically",
                )
            })
            .collect();
    }

    if overview.secrets.rotation_disabled > 0 && ctx.secrets.is_empty() {
        return vec![SECRETS_ROTATION_DISABLED.aggregate(
            &overview.region,
            format!(
                "{} secret(s) do not have rotation enabled",
                overview.secrets.rotation_disabled
            ),
            "Review secrets that should rotate automatically",
        )];
    }

    Vec::new()
}

const SECRETS_STALE_ROTATION: Rule = Rule {
    id: "secrets_stale_rotation",
    severity: FindingSeverity::Medium,
    category: FindingCategory::Hygiene,
    service: "Secrets Manager",
    route: FindingRoute::Secrets,
};

fn secrets_stale_rotation(ctx: &FindingContext) -> Vec<Finding> {
    if ctx.account_overview.is_none() {
        return Vec::new();
    }

    ctx.secrets
        .iter()
        .filter(|secret| secret.has_stale_rotation())
        .map(|secret| {
            SECRETS_STALE_ROTATION.resource(
                ctx.region_label,
                &secret.name,
                format!(
                    "Secret {} has not rotated in {}+ days",
                    secret.name,
                    SecretInfo::STALE_ROTATION_DAYS
                ),
                format!(
                    "Open Secrets Manager and review secrets that have not rotated in {}+ days",
                    SecretInfo::STALE_ROTATION_DAYS
                ),
            )
        })
        .collect()
}

const EC2_STOPPED_NEEDING_REVIEW: Rule = Rule {
    id: "ec2_stopped_instances_needing_review",
    severity: FindingSeverity::Medium,
    category: FindingCategory::Hygiene,
    service: "EC2",
    route: FindingRoute::Ec2,
};

fn ec2_stopped_instances_needing_review(ctx: &FindingContext) -> Vec<Finding> {
    if ctx.account_overview.is_none() {
        return Vec::new();
    }

    ctx.ec2_instances
        .iter()
        .filter(|instance| instance.needs_stopped_review())
        .map(|instance| {
            EC2_STOPPED_NEEDING_REVIEW.resource(
                &instance.region,
                &instance.id,
                format!(
                    "Stopped instance {} still looks important ({})",
                    instance.label(),
                    instance.review_signals().join("/")
                ),
                "Open EC2 and review stopped instances with public IPs or production-like names",
            )
        })
        .collect()
}

const EC2_TAG_COVERAGE_GAPS: Rule = Rule {
    id: "ec2_tag_coverage_gaps",
    severity: FindingSeverity::Low,
    category: FindingCategory::Hygiene,
    service: "EC2",
    route: FindingRoute::Ec2,
};

fn ec2_tag_coverage_gaps(ctx: &FindingContext) -> Vec<Finding> {
    if ctx.account_overview.is_none() {
        return Vec::new();
    }

    ctx.ec2_instances
        .iter()
        .filter(|instance| instance.has_tag_coverage_gap())
        .map(|instance| {
            let missing = instance
                .missing_required_tags()
                .unwrap_or_default()
                .join(", ");

            EC2_TAG_COVERAGE_GAPS.resource(
                &instance.region,
                &instance.id,
                format!("Instance {} is missing tags: {missing}", instance.label()),
                "Open EC2 and add Name, Owner, or Environment tags to unmanaged instances",
            )
        })
        .collect()
}

/// The tag that attributes a resource to a person or team.
const OWNER_TAG: &str = "Owner";

const RESOURCES_MISSING_OWNER_TAG: Rule = Rule {
    id: "resources_missing_owner_tag",
    severity: FindingSeverity::Low,
    category: FindingCategory::Hygiene,
    service: "",
    route: FindingRoute::Ec2,
};

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

    // (service label, route, resource id, display name) per unowned resource.
    let mut subjects: Vec<(&'static str, FindingRoute, String, String)> = Vec::new();

    for item in ctx.rds_instances.iter().filter(|i| unowned(&i.tags)) {
        subjects.push((
            "RDS",
            FindingRoute::Rds,
            item.identifier.clone(),
            item.identifier.clone(),
        ));
    }

    for item in ctx.secrets.iter().filter(|i| unowned(&i.tags)) {
        subjects.push((
            "Secrets Manager",
            FindingRoute::Secrets,
            item.name.clone(),
            item.name.clone(),
        ));
    }

    for item in ctx.apigateway_apis.iter().filter(|i| unowned(&i.tags)) {
        subjects.push((
            "API Gateway",
            FindingRoute::Apigateway,
            item.id.clone(),
            item.name.clone(),
        ));
    }

    for item in ctx.vpcs.iter().filter(|i| unowned(&i.tags)) {
        subjects.push((
            "VPC",
            FindingRoute::Vpc,
            item.vpc_id.clone(),
            item.vpc_id.clone(),
        ));
    }

    for item in ctx.security_groups.iter().filter(|i| unowned(&i.tags)) {
        subjects.push((
            "Security Groups",
            FindingRoute::SecurityGroups,
            item.id.clone(),
            item.name.clone(),
        ));
    }

    for item in ctx.lambda_functions.iter().filter(|i| unowned(&i.tags)) {
        subjects.push((
            "Lambda",
            FindingRoute::Lambda,
            item.name.clone(),
            item.name.clone(),
        ));
    }

    for item in ctx.sqs_queues_data.iter().filter(|i| unowned(&i.tags)) {
        subjects.push((
            "SQS",
            FindingRoute::Sqs,
            item.queue_url.clone(),
            item.name.clone(),
        ));
    }

    for item in ctx.cloudwatch_alarms.iter().filter(|i| unowned(&i.tags)) {
        subjects.push((
            "CloudWatch",
            FindingRoute::CloudWatch,
            item.name.clone(),
            item.name.clone(),
        ));
    }

    for item in ctx.load_balancers.iter().filter(|i| unowned(&i.tags)) {
        subjects.push((
            "Load Balancers",
            FindingRoute::LoadBalancers,
            item.arn.clone(),
            item.name.clone(),
        ));
    }

    for item in ctx.target_groups.iter().filter(|i| unowned(&i.tags)) {
        subjects.push((
            "Target Groups",
            FindingRoute::TargetGroups,
            item.arn.clone(),
            item.name.clone(),
        ));
    }

    subjects
        .into_iter()
        .map(|(service, route, resource_id, display)| Finding {
            service: service.into(),
            route,
            ..RESOURCES_MISSING_OWNER_TAG.resource(
                ctx.region_label,
                resource_id,
                format!("{service} resource {display} has no {OWNER_TAG} tag"),
                format!("Add an {OWNER_TAG} tag so this resource can be attributed to a team"),
            )
        })
        .collect()
}

const EC2_SUSTAINED_LOW_CPU: Rule = Rule {
    id: "ec2_sustained_low_cpu",
    severity: FindingSeverity::Medium,
    category: FindingCategory::Waste,
    service: "EC2",
    route: FindingRoute::Ec2,
};

fn ec2_sustained_low_cpu(ctx: &FindingContext) -> Vec<Finding> {
    if ctx.account_overview.is_none() {
        return Vec::new();
    }

    ctx.ec2_instances
        .iter()
        .filter(|instance| instance.has_sustained_low_cpu())
        .map(|instance| {
            EC2_SUSTAINED_LOW_CPU.resource(
                &instance.region,
                &instance.id,
                format!(
                    "Instance {} averaged {} CPU over the last {} days",
                    instance.label(),
                    instance.formatted_avg_cpu(),
                    Ec2InstanceInfo::LOW_CPU_LOOKBACK_DAYS
                ),
                format!(
                    "Open EC2 and review running instances averaging below {:.1}% CPU over the last {} days",
                    Ec2InstanceInfo::LOW_CPU_THRESHOLD_PERCENT,
                    Ec2InstanceInfo::LOW_CPU_LOOKBACK_DAYS
                ),
            )
        })
        .collect()
}

const EC2_STOPPED_UNUSED: Rule = Rule {
    id: "ec2_stopped_instances_unused",
    severity: FindingSeverity::Medium,
    category: FindingCategory::Waste,
    service: "EC2",
    route: FindingRoute::Ec2,
};

fn ec2_stopped_instances_unused(ctx: &FindingContext) -> Vec<Finding> {
    let Some(overview) = ctx.account_overview else {
        return Vec::new();
    };

    let plain_stopped = ctx
        .ec2_instances
        .iter()
        .filter(|instance| instance.is_stopped() && !instance.needs_stopped_review())
        .collect::<Vec<_>>();

    if !plain_stopped.is_empty() {
        return plain_stopped
            .iter()
            .map(|instance| {
                EC2_STOPPED_UNUSED.resource(
                    &instance.region,
                    &instance.id,
                    format!("Stopped instance {} may be unused", instance.label()),
                    "Review stopped instances for cleanup or restart",
                )
            })
            .collect();
    }

    if overview.ec2_stopped > 0 && ctx.ec2_instances.is_empty() {
        return vec![EC2_STOPPED_UNUSED.aggregate(
            &overview.region,
            format!("{} stopped instance(s) may be unused", overview.ec2_stopped),
            "Review stopped instances for cleanup or restart",
        )];
    }

    Vec::new()
}

const SECURITY_GROUPS_SENSITIVE_PORTS: Rule = Rule {
    id: "security_groups_sensitive_public_ports",
    severity: FindingSeverity::High,
    category: FindingCategory::Hygiene,
    service: "Security Groups",
    route: FindingRoute::SecurityGroups,
};

fn security_groups_sensitive_public_ports(ctx: &FindingContext) -> Vec<Finding> {
    ctx.security_groups
        .iter()
        .filter(|sg| !sg.sensitive_public_ports.is_empty())
        .map(|sg| {
            SECURITY_GROUPS_SENSITIVE_PORTS.resource(
                ctx.region_label,
                &sg.id,
                format!(
                    "Security group {} exposes sensitive port(s) publicly: {}",
                    sg.name,
                    sg.sensitive_ports_label()
                ),
                "Review public access on sensitive ports and narrow ingress",
            )
        })
        .collect()
}

const SECURITY_GROUPS_OPEN_TO_WORLD: Rule = Rule {
    id: "security_groups_open_to_world",
    severity: FindingSeverity::Medium,
    category: FindingCategory::Hygiene,
    service: "Security Groups",
    route: FindingRoute::SecurityGroups,
};

fn security_groups_open_to_world(ctx: &FindingContext) -> Vec<Finding> {
    ctx.security_groups
        .iter()
        .filter(|sg| sg.open_to_world && sg.sensitive_public_ports.is_empty())
        .map(|sg| {
            SECURITY_GROUPS_OPEN_TO_WORLD.resource(
                ctx.region_label,
                &sg.id,
                format!("Security group {} is open to the world", sg.name),
                "Review public ingress rules and narrow access",
            )
        })
        .collect()
}

const APIGATEWAY_GENERIC_OR_STALE: Rule = Rule {
    id: "apigateway_generic_or_stale_apis",
    severity: FindingSeverity::Medium,
    category: FindingCategory::Waste,
    service: "API Gateway",
    route: FindingRoute::Apigateway,
};

fn apigateway_generic_or_stale_apis(ctx: &FindingContext) -> Vec<Finding> {
    ctx.apigateway_apis
        .iter()
        .filter(|api| api.needs_review())
        .map(|api| {
            APIGATEWAY_GENERIC_OR_STALE.resource(
                ctx.region_label,
                &api.id,
                format!(
                    "API {} looks generic or stale ({})",
                    api.name,
                    api.review_signals().join("/")
                ),
                "Open API Gateway and review generic or year-old APIs for ownership and cleanup",
            )
        })
        .collect()
}

const SQS_QUEUES_WITHOUT_DLQ: Rule = Rule {
    id: "sqs_queues_without_dlq",
    severity: FindingSeverity::Medium,
    category: FindingCategory::Hygiene,
    service: "SQS",
    route: FindingRoute::Sqs,
};

fn sqs_queues_without_dlq(ctx: &FindingContext) -> Vec<Finding> {
    ctx.sqs_queues_data
        .iter()
        .filter(|queue| !queue.has_dlq)
        .map(|queue| {
            SQS_QUEUES_WITHOUT_DLQ.resource(
                ctx.region_label,
                &queue.queue_url,
                format!("Queue {} does not have a DLQ configured", queue.name),
                "Review queues without DLQs and add redrive policies where needed",
            )
        })
        .collect()
}

const SQS_QUEUE_BACKLOG: Rule = Rule {
    id: "sqs_queue_backlog",
    severity: FindingSeverity::High,
    category: FindingCategory::Incident,
    service: "SQS",
    route: FindingRoute::Sqs,
};

fn sqs_queue_backlog(ctx: &FindingContext) -> Vec<Finding> {
    ctx.sqs_queues_data
        .iter()
        .filter(|queue| queue.has_backlog_incident())
        .map(|queue| {
            SQS_QUEUE_BACKLOG.resource(
                ctx.region_label,
                &queue.queue_url,
                format!(
                    "Queue {} has high backlog or stuck work ({})",
                    queue.name,
                    queue.backlog_signals().join("/")
                ),
                format!(
                    "Open SQS and inspect queues with >= {} visible or >= {} in-flight messages",
                    SqsQueueInfo::HIGH_VISIBLE_THRESHOLD,
                    SqsQueueInfo::HIGH_IN_FLIGHT_THRESHOLD
                ),
            )
        })
        .collect()
}

const RDS_NOT_AVAILABLE: Rule = Rule {
    id: "rds_instances_not_available",
    severity: FindingSeverity::High,
    category: FindingCategory::Incident,
    service: "RDS",
    route: FindingRoute::Rds,
};

fn rds_instances_not_available(ctx: &FindingContext) -> Vec<Finding> {
    ctx.rds_instances
        .iter()
        .filter(|db| db.status != "available")
        .map(|db| {
            RDS_NOT_AVAILABLE.resource(
                &db.region,
                &db.identifier,
                format!("RDS instance {} is {}", db.identifier, db.status),
                "Open RDS and investigate instance status and recovery path",
            )
        })
        .collect()
}

const RDS_SINGLE_AZ_PRODUCTION_LIKE: Rule = Rule {
    id: "rds_single_az_production_like",
    severity: FindingSeverity::Medium,
    category: FindingCategory::Hygiene,
    service: "RDS",
    route: FindingRoute::Rds,
};

fn rds_single_az_production_like(ctx: &FindingContext) -> Vec<Finding> {
    ctx.rds_instances
        .iter()
        .filter(|db| db.needs_single_az_review())
        .map(|db| {
            RDS_SINGLE_AZ_PRODUCTION_LIKE.resource(
                &db.region,
                &db.identifier,
                format!(
                    "Single-AZ RDS instance {} looks production-like",
                    db.identifier
                ),
                "Open RDS and review production-like single-AZ databases for Multi-AZ coverage",
            )
        })
        .collect()
}

const VPC_DEFAULT_PRESENT: Rule = Rule {
    id: "vpc_default_vpcs_present",
    severity: FindingSeverity::Low,
    category: FindingCategory::Hygiene,
    service: "VPC",
    route: FindingRoute::Vpc,
};

fn vpc_default_vpcs_present(ctx: &FindingContext) -> Vec<Finding> {
    ctx.vpcs
        .iter()
        .filter(|vpc| vpc.is_default)
        .map(|vpc| {
            VPC_DEFAULT_PRESENT.resource(
                ctx.region_label,
                &vpc.vpc_id,
                format!("Default VPC {} is still present", vpc.vpc_id),
                "Review default VPC usage and remove or restrict it if unnecessary",
            )
        })
        .collect()
}

const LOAD_BALANCERS_ZERO_HEALTHY: Rule = Rule {
    id: "load_balancers_zero_healthy_targets",
    severity: FindingSeverity::High,
    category: FindingCategory::Incident,
    service: "Load Balancers",
    route: FindingRoute::LoadBalancers,
};

fn load_balancers_zero_healthy_targets(ctx: &FindingContext) -> Vec<Finding> {
    ctx.load_balancers
        .iter()
        .filter(|lb| lb.has_zero_healthy_targets())
        .map(|lb| {
            LOAD_BALANCERS_ZERO_HEALTHY.resource(
                ctx.region_label,
                &lb.arn,
                format!(
                    "Load balancer {} has target groups but zero healthy targets",
                    lb.name
                ),
                "Open load balancers and restore healthy registered targets behind the listener",
            )
        })
        .collect()
}

const LOAD_BALANCERS_NO_ACTIVE_TARGETS: Rule = Rule {
    id: "load_balancers_no_active_targets",
    severity: FindingSeverity::Medium,
    category: FindingCategory::Waste,
    service: "Load Balancers",
    route: FindingRoute::LoadBalancers,
};

fn load_balancers_no_active_targets(ctx: &FindingContext) -> Vec<Finding> {
    ctx.load_balancers
        .iter()
        .filter(|lb| lb.has_no_active_targets() && !lb.has_zero_healthy_targets())
        .map(|lb| {
            LOAD_BALANCERS_NO_ACTIVE_TARGETS.resource(
                ctx.region_label,
                &lb.arn,
                format!(
                    "Load balancer {} has no active target path ({})",
                    lb.name,
                    lb.review_signals().join("/")
                ),
                "Open load balancers and review listeners with no target groups or no registered targets",
            )
        })
        .collect()
}

const LAMBDA_HIGH_MEMORY: Rule = Rule {
    id: "lambda_high_memory_functions",
    severity: FindingSeverity::Medium,
    category: FindingCategory::Waste,
    service: "Lambda",
    route: FindingRoute::Lambda,
};

fn lambda_high_memory_functions(ctx: &FindingContext) -> Vec<Finding> {
    ctx.lambda_functions
        .iter()
        .filter(|f| f.has_high_memory())
        .map(|f| {
            LAMBDA_HIGH_MEMORY.resource(
                &f.region,
                &f.name,
                format!("Function {} is configured with {} MB", f.name, f.memory_mb),
                "Review high-memory Lambda functions for right-sizing",
            )
        })
        .collect()
}

const LAMBDA_STALE: Rule = Rule {
    id: "lambda_stale_functions",
    severity: FindingSeverity::Low,
    category: FindingCategory::Waste,
    service: "Lambda",
    route: FindingRoute::Lambda,
};

fn lambda_stale_functions(ctx: &FindingContext) -> Vec<Finding> {
    ctx.lambda_functions
        .iter()
        .filter(|f| f.is_stale())
        .map(|f| {
            LAMBDA_STALE.resource(
                &f.region,
                &f.name,
                format!(
                    "Function {} has not been modified in {}+ days",
                    f.name,
                    LambdaFunctionInfo::STALE_DEPLOY_DAYS
                ),
                "Review stale Lambda functions for ownership or cleanup",
            )
        })
        .collect()
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

    // One rule reporting the same resource twice is the same finding, not two.
    // Findings with no resource id are never collapsed, since they have no
    // identity to compare and each one is a distinct rollup.
    let mut seen = std::collections::HashSet::new();
    findings.retain(|finding| match finding.key() {
        Some(key) => seen.insert(key),
        None => true,
    });

    findings.sort_by(|a, b| {
        a.severity
            .rank()
            .cmp(&b.severity.rank())
            .then_with(|| a.category.rank().cmp(&b.category.rank()))
            .then_with(|| a.service.cmp(&b.service))
            .then_with(|| a.resource_id.cmp(&b.resource_id))
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
            tags: Tags::loaded([("Owner", "platform")]),
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
            tags: Tags::loaded([("Owner", "platform")]),
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
            tags: Tags::loaded([("Owner", "platform")]),
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
    fn services_needing_a_separate_tag_call_are_covered_by_the_owner_rule() {
        let overview = overview();
        let mut untagged_lambda = lambda(128);
        untagged_lambda.tags = Tags::empty();
        let functions = vec![untagged_lambda];

        let mut context = ctx(Some(&overview));
        context.lambda_functions = &functions;

        let findings = resources_missing_owner_tag(&context);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].service, "Lambda");
    }

    /// Lambda, SQS, CloudWatch, ELB, and target groups look tags up in a
    /// separate call that can fail on its own. A failure must not be reported
    /// as missing ownership.
    #[test]
    fn a_failed_tag_lookup_is_not_reported_as_unowned() {
        let overview = overview();
        let mut unreadable_lambda = lambda(128);
        unreadable_lambda.tags = Tags::Unavailable;
        let functions = vec![unreadable_lambda];

        let mut context = ctx(Some(&overview));
        context.lambda_functions = &functions;

        assert!(resources_missing_owner_tag(&context).is_empty());
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
        assert_eq!(
            found[0].summary,
            "Alarm cpu-high is in ALARM (CPUUtilization)"
        );
        assert_eq!(found[0].resource_id.as_deref(), Some("cpu-high"));
    }

    /// One finding per offending resource, each independently identifiable.
    #[test]
    fn a_rule_emits_one_finding_per_offending_resource() {
        let ov = overview();
        let alarms = vec![
            alarm("cpu-high", "ALARM", "AWS/EC2"),
            alarm("mem-high", "ALARM", "AWS/EC2"),
            alarm("disk-ok", "OK", "AWS/EC2"),
        ];
        let mut c = ctx(Some(&ov));
        c.cloudwatch_alarms = &alarms;

        let found = cloudwatch_alarms_in_alarm(&c);
        assert_eq!(found.len(), 2);

        let keys = found.iter().filter_map(|f| f.key()).collect::<Vec<_>>();
        assert_eq!(keys.len(), 2, "every per-resource finding has a key");
        assert_ne!(keys[0], keys[1], "keys distinguish the two alarms");
    }

    /// The overview fallback has no resources in hand, so it cannot carry an
    /// id and must not pretend to.
    #[test]
    fn an_overview_fallback_finding_has_no_resource_id() {
        let mut ov = overview();
        ov.alarms.alarms_in_alarm = 4;
        let c = ctx(Some(&ov));

        let found = cloudwatch_alarms_in_alarm(&c);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].resource_id, None);
        assert_eq!(found[0].key(), None);
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
            "Target group all-down has zero healthy targets (2 registered)"
        );
        assert!(zero[0].resource_id.as_deref().unwrap().contains("all-down"));

        let unhealthy = target_groups_unhealthy_targets(&c);
        assert_eq!(unhealthy.len(), 1);
        assert_eq!(
            unhealthy[0].summary,
            "Target group degraded has 1 unhealthy target(s) of 4"
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
            "Security group web exposes sensitive port(s) publicly: 22,3389"
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
        assert_eq!(found[0].summary, "Default VPC vpc-1 is still present");

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
        assert_eq!(found[1].severity, FindingSeverity::Low);
    }

    /// Alphabetical ordering on the category label would put Hygiene ahead of
    /// Incident, burying the urgent findings.
    #[test]
    fn incidents_sort_ahead_of_hygiene_at_equal_severity() {
        let ov = overview();
        let groups = vec![security_group(true, vec![22])];
        let alarms = vec![alarm("cpu-high", "ALARM", "AWS/EC2")];
        let mut c = ctx(Some(&ov));
        c.security_groups = &groups;
        c.cloudwatch_alarms = &alarms;

        let found = build_findings(&c);
        let high = found
            .iter()
            .filter(|f| f.severity == FindingSeverity::High)
            .collect::<Vec<_>>();

        assert!(high.len() >= 2);
        assert_eq!(high[0].category, FindingCategory::Incident);
        assert_eq!(high.last().unwrap().category, FindingCategory::Hygiene);
    }

    /// In global mode the view label is the same for every region, so a
    /// name-keyed resource must take its region from the resource itself or
    /// two different functions would collapse into one finding.
    #[test]
    fn same_named_resources_in_different_regions_stay_distinct() {
        let mut east = lambda(LambdaFunctionInfo::HIGH_MEMORY_THRESHOLD_MB);
        east.region = "us-east-1".into();
        let mut west = lambda(LambdaFunctionInfo::HIGH_MEMORY_THRESHOLD_MB);
        west.region = "eu-west-1".into();
        assert_eq!(east.name, west.name, "same function name in both regions");

        let functions = vec![east, west];
        let mut c = ctx(None);
        c.lambda_functions = &functions;

        let found = lambda_high_memory_functions(&c);
        assert_eq!(found.len(), 2);
        assert_ne!(found[0].key(), found[1].key());
        assert_eq!(found[0].region, "us-east-1");
        assert_eq!(found[1].region, "eu-west-1");
    }

    #[test]
    fn build_findings_collapses_a_repeated_resource() {
        let ov = overview();
        let duplicated = vec![vpc(true), vpc(true)];
        let mut c = ctx(Some(&ov));
        c.vpcs = &duplicated;

        // Both entries are the same VPC id, so they are one finding.
        let found = build_findings(&c);
        let vpc_findings = found
            .iter()
            .filter(|f| f.rule == "vpc_default_vpcs_present")
            .count();
        assert_eq!(vpc_findings, 1);
    }

    /// Rollups have no identity to compare, so they must survive dedup.
    #[test]
    fn build_findings_keeps_every_finding_without_a_resource_id() {
        let mut ov = overview();
        ov.alarms.alarms_in_alarm = 2;
        ov.ec2_stopped = 3;
        let c = ctx(Some(&ov));

        let found = build_findings(&c);
        let rollups = found.iter().filter(|f| f.resource_id.is_none()).count();
        assert!(
            rollups >= 2,
            "expected both overview fallbacks, got {rollups}"
        );
    }

    #[test]
    fn build_findings_is_empty_without_data() {
        assert!(build_findings(&ctx(None)).is_empty());
    }
}
