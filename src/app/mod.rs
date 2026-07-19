use aws_config::Region;
use chrono::Utc;
use std::collections::BTreeSet;

use crate::aws::clients::AwsClients;
use crate::cache::cost::{load_if_fresh, save, CostCache};
use crate::models::apigatway::ApiGatewayInfo;
use crate::models::cloudwatch::{CloudWatchAlarm, CloudWatchSummary};
use crate::models::describable::DescribableResource;
use crate::models::ec2::Ec2InstanceInfo;
use crate::models::elb::LoadBalancerInfo;
use crate::models::finding::{Finding, FindingCategory, FindingRoute, FindingSeverity};
use crate::models::lambda::LambdaFunctionInfo;
use crate::models::rds::{RdsInstanceInfo, RdsSummary};
use crate::models::secrets::{SecretInfo, SecretsSummary};
use crate::models::security_group::SecurityGroupInfo;
use crate::models::service_status::ServiceStatus;
use crate::models::sqs::SqsQueueInfo;
use crate::models::target_group::TargetGroupInfo;
use crate::models::vpc::VpcInfo;
use crate::models::{
    AccountOverview, BudgetInfo, CostSavingsOpportunity, EcsClusterInfo, SavingsRoute,
    ServiceCostInsight,
};
use crate::resources::ssh;
use crate::ui::footer::FooterMode;
use crate::ui::notification::{Notification, NotificationLevel};
use crate::ui::open::open_in_browser;
use crate::ui::overlay::overlays::{
    ConfirmCommandState, DescribeOverlayState, OverlayState, SelectProfileState, SelectSshKeyState,
};
use crate::ui::theme::{Theme, ThemeName};
use crate::{aws, config};

#[derive(Debug, Clone)]
pub enum RefreshPhase {
    Idle,
    Overview,
    Services(Vec<&'static str>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Findings,
    AccountOverview,
    CostOverview,
    CostSavings,
    Ecs,
    Ec2,
    Rds,
    Lambda,
    Apigateway,
    Sqs,
    Vpc,
    Secrets,
    CloudWatch,
    LoadBalancers,
    TargetGroups,
    SecurityGroups,
}
pub struct App {
    pub aws: AwsClients,
    pub overlay: Option<OverlayState>,

    // Global App properties
    pub cost_loaded: bool,
    pub regions: Vec<Region>,
    pub current_region_index: usize,
    pub profiles: Vec<String>,
    pub current_profile: Option<String>,
    pub active_view: ActiveView,
    pub should_quit: bool,
    pub command_mode: bool,
    pub command_input: String,
    pub show_help: bool,
    pub scroll_offset: u16,
    pub selected_row: usize,
    pub wrap_text: bool,
    pub detail_scroll_offset: u16,
    pub last_refresh: Option<chrono::DateTime<chrono::Utc>>,
    pub is_refreshing: bool,
    pub refresh_phase: RefreshPhase,

    pub footer_mode: FooterMode,
    pub notification: Option<Notification>,

    pub theme: Theme,
    pub theme_name: ThemeName,
    pub findings: Vec<Finding>,
    // Account overview
    pub account_overview: Option<AccountOverview>,
    pub budget: BudgetInfo,
    pub monthly_costs: Vec<f64>,
    pub service_costs: Vec<(String, f64)>,
    pub service_cost_insights: Vec<ServiceCostInsight>,
    pub cost_savings_opportunities: Vec<CostSavingsOpportunity>,

    // ECS
    pub ecs_clusters: Vec<EcsClusterInfo>,

    // Lambda
    pub lambda_functions: Vec<LambdaFunctionInfo>,
    pub lambda_status: ServiceStatus,

    // APIGW
    pub apigateway_apis: Vec<ApiGatewayInfo>,
    pub apigateway_status: ServiceStatus,

    // SQS
    pub sqs_queues_data: Vec<SqsQueueInfo>,
    pub sqs_status: ServiceStatus,

    // VPC
    pub vpcs: Vec<VpcInfo>,
    pub vpc_status: ServiceStatus,

    // EC2
    pub ec2_instances: Vec<Ec2InstanceInfo>,

    // Cloudwatch
    pub cloudwatch_summary: CloudWatchSummary,
    pub cloudwatch_alarms: Vec<CloudWatchAlarm>,

    // Secrets Manager
    pub secrets_summary: SecretsSummary,
    pub secrets: Vec<SecretInfo>,

    // RDS
    pub rds_summary: RdsSummary,
    pub rds_instances: Vec<RdsInstanceInfo>,

    // Load Balancers
    pub load_balancers: Vec<LoadBalancerInfo>,
    pub load_balancers_status: ServiceStatus,

    // Target Groups
    pub target_groups: Vec<TargetGroupInfo>,
    pub target_groups_status: ServiceStatus,

    // Security Groups
    pub security_groups: Vec<SecurityGroupInfo>,
    pub security_groups_status: ServiceStatus,
}

impl App {
    pub fn new(aws: AwsClients) -> Self {
        Self {
            aws,
            overlay: None,
            scroll_offset: 0,
            selected_row: 0,
            wrap_text: false,
            detail_scroll_offset: 0,
            cost_loaded: false,
            account_overview: None,
            regions: vec![],
            current_region_index: 0,
            profiles: vec![],
            current_profile: None,
            should_quit: false,
            command_mode: false,
            command_input: String::new(),
            active_view: ActiveView::Findings,
            budget: BudgetInfo {
                monthly_budget: 0.0,
                month_to_date_cost: 0.0,
                forecast: 0.0,
                forecast_low: None,
                forecast_high: None,
            },
            monthly_costs: vec![0.0; 6],
            service_costs: vec![],
            service_cost_insights: vec![],
            cost_savings_opportunities: vec![],
            theme: Theme::autumn(),
            theme_name: ThemeName::Autumn,
            findings: vec![],
            lambda_functions: vec![],
            lambda_status: ServiceStatus::Unavailable("Not loaded".into()),
            apigateway_apis: vec![],
            apigateway_status: ServiceStatus::Unavailable("Not loaded".into()),
            sqs_queues_data: vec![],
            sqs_status: ServiceStatus::Unavailable("Not loaded".into()),
            vpcs: vec![],
            vpc_status: ServiceStatus::Unavailable("Not loaded".into()),
            cloudwatch_summary: CloudWatchSummary {
                status: ServiceStatus::Unavailable("Not loaded".into()),
                total_alarms: 0,
                alarms_in_alarm: 0,
            },
            cloudwatch_alarms: vec![],
            ec2_instances: vec![],
            ecs_clusters: vec![],

            secrets_summary: SecretsSummary {
                status: ServiceStatus::Unavailable("Not loaded".into()),
                total: 0,
                rotation_disabled: 0,
            },
            secrets: vec![],

            load_balancers: vec![],
            load_balancers_status: ServiceStatus::Unavailable("Not loaded".into()),

            rds_summary: RdsSummary {
                status: ServiceStatus::Unavailable("Not loaded".into()),
                total: 0,
                available: 0,
            },
            rds_instances: vec![],

            last_refresh: None,
            is_refreshing: false,
            refresh_phase: RefreshPhase::Idle,
            footer_mode: FooterMode::Normal,
            notification: None,
            show_help: false,

            target_groups: vec![],
            target_groups_status: ServiceStatus::Unavailable("Not loaded".into()),
            security_groups: vec![],
            security_groups_status: ServiceStatus::Unavailable("Not loaded".into()),
        }
    }

    pub fn notify_error(&mut self, message: impl Into<String>) {
        self.notification = Some(Notification::new(message, NotificationLevel::Error));
    }

    pub fn notify_warning(&mut self, message: impl Into<String>) {
        self.notification = Some(Notification::new(message, NotificationLevel::Warning));
    }

    /// Drop the active notification once it has outlived its display window.
    /// Called from the event loop so a toast auto-dismisses on the next tick.
    pub fn clear_expired_notification(&mut self) {
        if self
            .notification
            .as_ref()
            .is_some_and(Notification::is_expired)
        {
            self.notification = None;
        }
    }

    pub fn current_region(&self) -> &Region {
        let idx = self
            .current_region_index
            .min(self.regions.len().saturating_sub(1));

        &self.regions[idx]
    }

    pub fn is_global_region_selected(&self) -> bool {
        !self.regions.is_empty() && self.current_region_index == self.regions.len()
    }

    pub fn region_slot_count(&self) -> usize {
        if self.regions.is_empty() {
            0
        } else {
            self.regions.len() + 1
        }
    }

    pub fn current_region_label(&self) -> String {
        if self.is_global_region_selected() {
            "global".to_string()
        } else {
            self.current_region().as_ref().to_string()
        }
    }

    pub fn set_global_region(&mut self) {
        if self.regions.is_empty() {
            return;
        }

        self.current_region_index = self.regions.len();
    }

    pub fn persist_region_selection(&self) {
        self.persist_preferences();
    }

    pub fn persist_preferences(&self) {
        let mut cfg = config::load_config();
        cfg.region = Some(self.current_region_label());
        cfg.theme = Some(self.theme_name.as_str().to_string());
        cfg.profile = self.current_profile.clone();
        config::save_config(&cfg);
    }

    /// Rebuild every AWS client for the current region and profile. Region and
    /// profile switches both route through here so a selected profile is
    /// preserved across region changes.
    async fn rebuild_aws_clients(&mut self) {
        let sdk_config = aws::clients::build_sdk_config(
            self.current_region().clone(),
            self.current_profile.as_deref(),
        )
        .await;

        self.aws = AwsClients::new(&sdk_config);
    }

    /// Switch to a named AWS profile if it was discovered, rebuilding clients
    /// and refreshing. Returns false for an unknown profile name.
    pub async fn set_profile_by_name(&mut self, name: &str) -> bool {
        if !self.profiles.is_empty() && !self.profiles.iter().any(|p| p == name) {
            return false;
        }

        self.apply_profile(Some(name.to_string())).await;
        true
    }

    async fn apply_profile(&mut self, profile: Option<String>) {
        self.current_profile = profile;
        self.rebuild_aws_clients().await;
        self.persist_preferences();
        self.trigger_refresh();
    }

    pub fn open_profile_picker(&mut self) {
        let selected = self
            .current_profile
            .as_ref()
            .and_then(|current| self.profiles.iter().position(|p| p == current))
            .unwrap_or(0);

        self.overlay = Some(OverlayState::SelectProfile(SelectProfileState {
            profiles: self.profiles.clone(),
            selected,
        }));
        self.footer_mode = FooterMode::Overlay;
    }

    pub async fn commit_profile_selection(&mut self) {
        let selected = match &self.overlay {
            Some(OverlayState::SelectProfile(state)) => state.profiles.get(state.selected).cloned(),
            _ => None,
        };

        self.overlay = None;
        self.footer_mode = FooterMode::Normal;

        if let Some(name) = selected {
            self.apply_profile(Some(name)).await;
        }
    }

    pub fn set_theme_name(&mut self, theme_name: ThemeName) {
        self.theme_name = theme_name;
        self.theme = Theme::from_name(theme_name);
        self.persist_preferences();
    }

    pub fn cycle_theme(&mut self) {
        self.set_theme_name(self.theme_name.next());
    }

    pub async fn set_region_by_index(&mut self, index: usize) {
        if self.regions.is_empty() {
            return;
        }

        if index >= self.regions.len() {
            return;
        }

        self.current_region_index = index;
        self.rebuild_aws_clients().await;
    }

    pub async fn set_region_by_name(&mut self, name: &str) -> bool {
        if name.eq_ignore_ascii_case("global") {
            self.set_global_region();
            return true;
        }

        if let Some(idx) = self.regions.iter().position(|r| r.as_ref() == name) {
            self.set_region_by_index(idx).await;
            return true;
        }

        false
    }

    pub async fn on_view_enter(&mut self) {
        self.selected_row = 0;
        self.scroll_offset = 0;
        self.detail_scroll_offset = 0;
        self.trigger_refresh();
    }

    fn view_uses_free_scroll(&self) -> bool {
        matches!(self.active_view, ActiveView::AccountOverview)
    }

    pub fn active_view_supports_wrap(&self) -> bool {
        matches!(
            self.active_view,
            ActiveView::Findings | ActiveView::CostOverview | ActiveView::CostSavings
        )
    }

    pub fn wrap_mode_active(&self) -> bool {
        self.wrap_text && self.active_view_supports_wrap()
    }

    pub fn toggle_wrap_mode(&mut self) {
        if !self.active_view_supports_wrap() {
            return;
        }

        self.wrap_text = !self.wrap_text;
        self.detail_scroll_offset = 0;
    }

    fn active_view_item_count(&self) -> usize {
        match self.active_view {
            ActiveView::Findings => self.findings.len(),
            ActiveView::Ecs => self.ecs_clusters.len(),
            ActiveView::Ec2 => self.ec2_instances.len(),
            ActiveView::CostSavings => self.cost_savings_opportunities.len(),
            ActiveView::Rds => self.rds_instances.len(),
            ActiveView::Lambda => self.lambda_functions.len(),
            ActiveView::Apigateway => self.apigateway_apis.len(),
            ActiveView::Sqs => self.sqs_queues_data.len(),
            ActiveView::Vpc => self.vpcs.len(),
            ActiveView::Secrets => self.secrets.len(),
            ActiveView::CloudWatch => self.cloudwatch_alarms.len(),
            ActiveView::LoadBalancers => self.load_balancers.len(),
            ActiveView::TargetGroups => self.target_groups.len(),
            ActiveView::SecurityGroups => self.security_groups.len(),
            ActiveView::AccountOverview => 10,
            ActiveView::CostOverview => self.service_cost_insights.len(),
        }
    }

    pub fn scroll_active_view_up(&mut self, lines: usize) {
        if self.view_uses_free_scroll() {
            self.scroll_offset = self.scroll_offset.saturating_sub(lines as u16);
        } else {
            let previous = self.selected_row;
            self.selected_row = self.selected_row.saturating_sub(lines);
            if self.selected_row != previous {
                self.detail_scroll_offset = 0;
            }
        }
    }

    pub fn scroll_active_view_down(&mut self, lines: usize) {
        if self.view_uses_free_scroll() {
            self.scroll_offset = self.scroll_offset.saturating_add(lines as u16);
            return;
        }

        let total = self.active_view_item_count();
        if total == 0 {
            self.selected_row = 0;
            return;
        }

        let previous = self.selected_row;
        self.selected_row = self.selected_row.saturating_add(lines).min(total - 1);
        if self.selected_row != previous {
            self.detail_scroll_offset = 0;
        }
    }

    pub fn scroll_active_view_to_top(&mut self) {
        self.scroll_offset = 0;
        self.selected_row = 0;
        self.detail_scroll_offset = 0;
    }

    pub fn scroll_active_view_to_bottom(&mut self) {
        if self.view_uses_free_scroll() {
            self.scroll_offset = u16::MAX;
            return;
        }

        let total = self.active_view_item_count();
        self.selected_row = total.saturating_sub(1);
        self.detail_scroll_offset = 0;
    }

    pub fn scroll_wrapped_detail_up(&mut self, lines: usize) {
        self.detail_scroll_offset = self.detail_scroll_offset.saturating_sub(lines as u16);
    }

    pub fn scroll_wrapped_detail_down(&mut self, lines: usize) {
        self.detail_scroll_offset = self.detail_scroll_offset.saturating_add(lines as u16);
    }

    pub fn scroll_wrapped_detail_to_top(&mut self) {
        self.detail_scroll_offset = 0;
    }

    pub fn scroll_wrapped_detail_to_bottom(&mut self) {
        self.detail_scroll_offset = u16::MAX;
    }

    pub fn rebuild_findings(&mut self) {
        let mut findings = Vec::new();

        if let Some(overview) = &self.account_overview {
            let alarming_alarms = self
                .cloudwatch_alarms
                .iter()
                .filter(|alarm| alarm.state == "ALARM")
                .collect::<Vec<_>>();

            if !alarming_alarms.is_empty() {
                let sample_names = alarming_alarms
                    .iter()
                    .take(3)
                    .map(|alarm| alarm.name.clone())
                    .collect::<Vec<_>>();
                let sample_count = sample_names.len();
                let remaining = alarming_alarms.len().saturating_sub(sample_count);
                let sample_summary = sample_names.join(", ");
                let summary = if remaining > 0 {
                    format!(
                        "{} alarm(s) are in ALARM: {} (+{} more)",
                        alarming_alarms.len(),
                        sample_summary,
                        remaining
                    )
                } else {
                    format!(
                        "{} alarm(s) are in ALARM: {}",
                        alarming_alarms.len(),
                        sample_summary
                    )
                };

                findings.push(Finding {
                    severity: FindingSeverity::High,
                    category: FindingCategory::Incident,
                    service: "CloudWatch".into(),
                    region: self.current_region_label(),
                    summary,
                    next_step: "Open CloudWatch and inspect failing alarms".into(),
                    route: FindingRoute::CloudWatch,
                });
            } else if overview.alarms.alarms_in_alarm > 0 {
                findings.push(Finding {
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
                });
            }

            let alarm_namespaces = self
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

            if !coverage_gaps.is_empty() {
                let sample_services = coverage_gaps.iter().take(3).cloned().collect::<Vec<_>>();
                let sample_count = sample_services.len();
                let remaining = coverage_gaps.len().saturating_sub(sample_count);
                let summary = if remaining > 0 {
                    format!(
                        "{} deployed service area(s) appear to have no CloudWatch alarm coverage: {} (+{} more)",
                        coverage_gaps.len(),
                        sample_services.join(", "),
                        remaining
                    )
                } else {
                    format!(
                        "{} deployed service area(s) appear to have no CloudWatch alarm coverage: {}",
                        coverage_gaps.len(),
                        sample_services.join(", ")
                    )
                };

                findings.push(Finding {
                    severity: FindingSeverity::Medium,
                    category: FindingCategory::Hygiene,
                    service: "CloudWatch".into(),
                    region: self.current_region_label(),
                    summary,
                    next_step:
                        "Open CloudWatch and add alarms for deployed services without namespace coverage"
                            .into(),
                    route: FindingRoute::CloudWatch,
                });
            }

            let zero_healthy_target_groups = self
                .target_groups
                .iter()
                .filter(|tg| tg.has_zero_healthy_targets())
                .count();

            if zero_healthy_target_groups > 0 {
                findings.push(Finding {
                    severity: FindingSeverity::High,
                    category: FindingCategory::Incident,
                    service: "Target Groups".into(),
                    region: self.current_region_label(),
                    summary: format!(
                        "{zero_healthy_target_groups} target group(s) have zero healthy targets"
                    ),
                    next_step: "Open target groups and restore at least one healthy target".into(),
                    route: FindingRoute::TargetGroups,
                });
            }

            let partially_unhealthy_target_groups = self
                .target_groups
                .iter()
                .filter(|tg| tg.unhealthy_targets > 0 && !tg.has_zero_healthy_targets())
                .count();

            if partially_unhealthy_target_groups > 0 {
                findings.push(Finding {
                    severity: FindingSeverity::High,
                    category: FindingCategory::Incident,
                    service: "Target Groups".into(),
                    region: self.current_region_label(),
                    summary: format!(
                        "{partially_unhealthy_target_groups} target group(s) have unhealthy targets"
                    ),
                    next_step: "Open target groups and inspect unhealthy target health".into(),
                    route: FindingRoute::TargetGroups,
                });
            } else if overview.target_groups_unhealthy > 0 && self.target_groups.is_empty() {
                findings.push(Finding {
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
                });
            }

            let orphan_target_groups = self
                .target_groups
                .iter()
                .filter(|tg| tg.is_orphan_candidate())
                .collect::<Vec<_>>();

            if !orphan_target_groups.is_empty() {
                let sample_groups = orphan_target_groups
                    .iter()
                    .take(3)
                    .map(|tg| tg.name.clone())
                    .collect::<Vec<_>>();
                let sample_count = sample_groups.len();
                let remaining = orphan_target_groups.len().saturating_sub(sample_count);
                let summary = if remaining > 0 {
                    format!(
                        "{} target group(s) have no load balancer attachment and no registered targets: {} (+{} more)",
                        orphan_target_groups.len(),
                        sample_groups.join(", "),
                        remaining
                    )
                } else {
                    format!(
                        "{} target group(s) have no load balancer attachment and no registered targets: {}",
                        orphan_target_groups.len(),
                        sample_groups.join(", ")
                    )
                };

                findings.push(Finding {
                    severity: FindingSeverity::Medium,
                    category: FindingCategory::Waste,
                    service: "Target Groups".into(),
                    region: self.current_region_label(),
                    summary,
                    next_step:
                        "Open target groups and review orphan groups for cleanup or reattachment"
                            .into(),
                    route: FindingRoute::TargetGroups,
                });
            }

            let production_like_rotation_disabled = self
                .secrets
                .iter()
                .filter(|secret| secret.needs_rotation_review())
                .collect::<Vec<_>>();

            if !production_like_rotation_disabled.is_empty() {
                let sample_secrets = production_like_rotation_disabled
                    .iter()
                    .take(3)
                    .map(|secret| secret.name.clone())
                    .collect::<Vec<_>>();
                let sample_count = sample_secrets.len();
                let remaining = production_like_rotation_disabled
                    .len()
                    .saturating_sub(sample_count);
                let summary = if remaining > 0 {
                    format!(
                        "{} production-like secret(s) do not have rotation enabled: {} (+{} more)",
                        production_like_rotation_disabled.len(),
                        sample_secrets.join(", "),
                        remaining
                    )
                } else {
                    format!(
                        "{} production-like secret(s) do not have rotation enabled: {}",
                        production_like_rotation_disabled.len(),
                        sample_secrets.join(", ")
                    )
                };

                findings.push(Finding {
                    severity: FindingSeverity::High,
                    category: FindingCategory::Hygiene,
                    service: "Secrets Manager".into(),
                    region: self.current_region_label(),
                    summary,
                    next_step:
                        "Open Secrets Manager and enable rotation on production-like secrets"
                            .into(),
                    route: FindingRoute::Secrets,
                });
            }

            let rotation_disabled = self
                .secrets
                .iter()
                .filter(|secret| secret.rotation_disabled() && !secret.needs_rotation_review())
                .count();

            if rotation_disabled > 0 {
                findings.push(Finding {
                    severity: FindingSeverity::Medium,
                    category: FindingCategory::Hygiene,
                    service: "Secrets Manager".into(),
                    region: self.current_region_label(),
                    summary: format!("{rotation_disabled} secret(s) do not have rotation enabled"),
                    next_step: "Review secrets that should rotate automatically".into(),
                    route: FindingRoute::Secrets,
                });
            } else if overview.secrets.rotation_disabled > 0 && self.secrets.is_empty() {
                findings.push(Finding {
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
                });
            }

            let stale_rotation_secrets = self
                .secrets
                .iter()
                .filter(|secret| secret.has_stale_rotation())
                .collect::<Vec<_>>();

            if !stale_rotation_secrets.is_empty() {
                let sample_secrets = stale_rotation_secrets
                    .iter()
                    .take(3)
                    .map(|secret| secret.name.clone())
                    .collect::<Vec<_>>();
                let sample_count = sample_secrets.len();
                let remaining = stale_rotation_secrets.len().saturating_sub(sample_count);
                let summary = if remaining > 0 {
                    format!(
                        "{} secret(s) have not rotated in {}+ days: {} (+{} more)",
                        stale_rotation_secrets.len(),
                        SecretInfo::STALE_ROTATION_DAYS,
                        sample_secrets.join(", "),
                        remaining
                    )
                } else {
                    format!(
                        "{} secret(s) have not rotated in {}+ days: {}",
                        stale_rotation_secrets.len(),
                        SecretInfo::STALE_ROTATION_DAYS,
                        sample_secrets.join(", ")
                    )
                };

                findings.push(Finding {
                    severity: FindingSeverity::Medium,
                    category: FindingCategory::Hygiene,
                    service: "Secrets Manager".into(),
                    region: self.current_region_label(),
                    summary,
                    next_step: format!(
                        "Open Secrets Manager and review secrets that have not rotated in {}+ days",
                        SecretInfo::STALE_ROTATION_DAYS
                    ),
                    route: FindingRoute::Secrets,
                });
            }

            let stopped_instances_needing_review = self
                .ec2_instances
                .iter()
                .filter(|instance| instance.needs_stopped_review())
                .collect::<Vec<_>>();

            if !stopped_instances_needing_review.is_empty() {
                let sample_names = stopped_instances_needing_review
                    .iter()
                    .take(3)
                    .map(|instance| instance.name.clone().unwrap_or_else(|| instance.id.clone()))
                    .collect::<Vec<_>>();
                let sample_count = sample_names.len();
                let remaining = stopped_instances_needing_review
                    .len()
                    .saturating_sub(sample_count);
                let summary = if remaining > 0 {
                    format!(
                        "{} stopped instance(s) still look important: {} (+{} more)",
                        stopped_instances_needing_review.len(),
                        sample_names.join(", "),
                        remaining
                    )
                } else {
                    format!(
                        "{} stopped instance(s) still look important: {}",
                        stopped_instances_needing_review.len(),
                        sample_names.join(", ")
                    )
                };

                findings.push(Finding {
                    severity: FindingSeverity::Medium,
                    category: FindingCategory::Hygiene,
                    service: "EC2".into(),
                    region: self.current_region_label(),
                    summary,
                    next_step:
                        "Open EC2 and review stopped instances with public IPs or production-like names"
                            .into(),
                    route: FindingRoute::Ec2,
                });
            }

            let instances_with_tag_gaps = self
                .ec2_instances
                .iter()
                .filter(|instance| instance.has_tag_coverage_gap())
                .collect::<Vec<_>>();

            if !instances_with_tag_gaps.is_empty() {
                let sample_instances = instances_with_tag_gaps
                    .iter()
                    .take(3)
                    .map(|instance| {
                        let label = instance.name.clone().unwrap_or_else(|| instance.id.clone());
                        let missing = instance.missing_required_tags().join("/");
                        format!("{label} ({missing})")
                    })
                    .collect::<Vec<_>>();
                let sample_count = sample_instances.len();
                let remaining = instances_with_tag_gaps.len().saturating_sub(sample_count);
                let summary = if remaining > 0 {
                    format!(
                        "{} EC2 instance(s) are missing Name, Owner, or Environment tags: {} (+{} more)",
                        instances_with_tag_gaps.len(),
                        sample_instances.join(", "),
                        remaining
                    )
                } else {
                    format!(
                        "{} EC2 instance(s) are missing Name, Owner, or Environment tags: {}",
                        instances_with_tag_gaps.len(),
                        sample_instances.join(", ")
                    )
                };

                findings.push(Finding {
                    severity: FindingSeverity::Medium,
                    category: FindingCategory::Hygiene,
                    service: "EC2".into(),
                    region: self.current_region_label(),
                    summary,
                    next_step:
                        "Open EC2 and add Name, Owner, or Environment tags to unmanaged instances"
                            .into(),
                    route: FindingRoute::Ec2,
                });
            }

            let low_cpu_instances = self
                .ec2_instances
                .iter()
                .filter(|instance| instance.has_sustained_low_cpu())
                .collect::<Vec<_>>();

            if !low_cpu_instances.is_empty() {
                let sample_instances = low_cpu_instances
                    .iter()
                    .take(3)
                    .map(|instance| {
                        let label = instance.name.clone().unwrap_or_else(|| instance.id.clone());
                        format!("{label} ({})", instance.formatted_avg_cpu())
                    })
                    .collect::<Vec<_>>();
                let sample_count = sample_instances.len();
                let remaining = low_cpu_instances.len().saturating_sub(sample_count);
                let summary = if remaining > 0 {
                    format!(
                        "{} running EC2 instance(s) averaged below {:.1}% CPU over the last {} days: {} (+{} more)",
                        low_cpu_instances.len(),
                        Ec2InstanceInfo::LOW_CPU_THRESHOLD_PERCENT,
                        Ec2InstanceInfo::LOW_CPU_LOOKBACK_DAYS,
                        sample_instances.join(", "),
                        remaining
                    )
                } else {
                    format!(
                        "{} running EC2 instance(s) averaged below {:.1}% CPU over the last {} days: {}",
                        low_cpu_instances.len(),
                        Ec2InstanceInfo::LOW_CPU_THRESHOLD_PERCENT,
                        Ec2InstanceInfo::LOW_CPU_LOOKBACK_DAYS,
                        sample_instances.join(", ")
                    )
                };

                findings.push(Finding {
                    severity: FindingSeverity::Medium,
                    category: FindingCategory::Waste,
                    service: "EC2".into(),
                    region: self.current_region_label(),
                    summary,
                    next_step: format!(
                        "Open EC2 and review running instances averaging below {:.1}% CPU over the last {} days",
                        Ec2InstanceInfo::LOW_CPU_THRESHOLD_PERCENT,
                        Ec2InstanceInfo::LOW_CPU_LOOKBACK_DAYS
                    ),
                    route: FindingRoute::Ec2,
                });
            }

            let plain_stopped_instances = self
                .ec2_instances
                .iter()
                .filter(|instance| instance.is_stopped() && !instance.needs_stopped_review())
                .count();

            if plain_stopped_instances > 0 {
                findings.push(Finding {
                    severity: FindingSeverity::Medium,
                    category: FindingCategory::Waste,
                    service: "EC2".into(),
                    region: self.current_region_label(),
                    summary: format!("{plain_stopped_instances} stopped instance(s) may be unused"),
                    next_step: "Review stopped instances for cleanup or restart".into(),
                    route: FindingRoute::Ec2,
                });
            } else if overview.ec2_stopped > 0 && self.ec2_instances.is_empty() {
                findings.push(Finding {
                    severity: FindingSeverity::Medium,
                    category: FindingCategory::Waste,
                    service: "EC2".into(),
                    region: overview.region.clone(),
                    summary: format!("{} stopped instance(s) may be unused", overview.ec2_stopped),
                    next_step: "Review stopped instances for cleanup or restart".into(),
                    route: FindingRoute::Ec2,
                });
            }
        }

        let sensitive_port_groups = self
            .security_groups
            .iter()
            .filter(|sg| !sg.sensitive_public_ports.is_empty())
            .count();

        if sensitive_port_groups > 0 {
            let sensitive_ports = self
                .security_groups
                .iter()
                .flat_map(|sg| sg.sensitive_public_ports.iter().copied())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .map(|port| port.to_string())
                .collect::<Vec<_>>()
                .join(", ");

            findings.push(Finding {
                severity: FindingSeverity::High,
                category: FindingCategory::Hygiene,
                service: "Security Groups".into(),
                region: self.current_region_label(),
                summary: format!(
                    "{sensitive_port_groups} security group(s) expose sensitive ports publicly ({sensitive_ports})"
                ),
                next_step: "Review public access on sensitive ports and narrow ingress".into(),
                route: FindingRoute::SecurityGroups,
            });
        }

        let open_to_world = self
            .security_groups
            .iter()
            .filter(|sg| sg.open_to_world && sg.sensitive_public_ports.is_empty())
            .count();

        if open_to_world > 0 {
            findings.push(Finding {
                severity: FindingSeverity::Medium,
                category: FindingCategory::Hygiene,
                service: "Security Groups".into(),
                region: self.current_region_label(),
                summary: format!("{open_to_world} security group(s) are open to the world"),
                next_step: "Review public ingress rules and narrow access".into(),
                route: FindingRoute::SecurityGroups,
            });
        }

        let apis_needing_review = self
            .apigateway_apis
            .iter()
            .filter(|api| api.needs_review())
            .collect::<Vec<_>>();

        if !apis_needing_review.is_empty() {
            let sample_apis = apis_needing_review
                .iter()
                .take(3)
                .map(|api| {
                    let signals = api.review_signals().join("/");
                    format!("{} ({signals})", api.name)
                })
                .collect::<Vec<_>>();
            let sample_count = sample_apis.len();
            let remaining = apis_needing_review.len().saturating_sub(sample_count);
            let summary = if remaining > 0 {
                format!(
                    "{} API Gateway API(s) look generic or stale: {} (+{} more)",
                    apis_needing_review.len(),
                    sample_apis.join(", "),
                    remaining
                )
            } else {
                format!(
                    "{} API Gateway API(s) look generic or stale: {}",
                    apis_needing_review.len(),
                    sample_apis.join(", ")
                )
            };

            findings.push(Finding {
                severity: FindingSeverity::Medium,
                category: FindingCategory::Waste,
                service: "API Gateway".into(),
                region: self.current_region_label(),
                summary,
                next_step:
                    "Open API Gateway and review generic or year-old APIs for ownership and cleanup"
                        .into(),
                route: FindingRoute::Apigateway,
            });
        }

        let queues_without_dlq = self.sqs_queues_data.iter().filter(|q| !q.has_dlq).count();

        if queues_without_dlq > 0 {
            findings.push(Finding {
                severity: FindingSeverity::Medium,
                category: FindingCategory::Hygiene,
                service: "SQS".into(),
                region: self.current_region_label(),
                summary: format!("{queues_without_dlq} queue(s) do not have a DLQ configured"),
                next_step: "Review queues without DLQs and add redrive policies where needed"
                    .into(),
                route: FindingRoute::Sqs,
            });
        }

        let backlog_queues = self
            .sqs_queues_data
            .iter()
            .filter(|queue| queue.has_backlog_incident())
            .collect::<Vec<_>>();

        if !backlog_queues.is_empty() {
            let sample_queues = backlog_queues
                .iter()
                .take(3)
                .map(|queue| {
                    let signals = queue.backlog_signals().join("/");
                    format!("{} ({signals})", queue.name)
                })
                .collect::<Vec<_>>();
            let sample_count = sample_queues.len();
            let remaining = backlog_queues.len().saturating_sub(sample_count);
            let summary = if remaining > 0 {
                format!(
                    "{} queue(s) have high backlog or stuck work: {} (+{} more)",
                    backlog_queues.len(),
                    sample_queues.join(", "),
                    remaining
                )
            } else {
                format!(
                    "{} queue(s) have high backlog or stuck work: {}",
                    backlog_queues.len(),
                    sample_queues.join(", ")
                )
            };

            findings.push(Finding {
                severity: FindingSeverity::High,
                category: FindingCategory::Incident,
                service: "SQS".into(),
                region: self.current_region_label(),
                summary,
                next_step: format!(
                    "Open SQS and inspect queues with >= {} visible or >= {} in-flight messages",
                    SqsQueueInfo::HIGH_VISIBLE_THRESHOLD,
                    SqsQueueInfo::HIGH_IN_FLIGHT_THRESHOLD
                ),
                route: FindingRoute::Sqs,
            });
        }

        let rds_not_available = self
            .rds_instances
            .iter()
            .filter(|db| db.status != "available")
            .count();

        if rds_not_available > 0 {
            findings.push(Finding {
                severity: FindingSeverity::High,
                category: FindingCategory::Incident,
                service: "RDS".into(),
                region: self.current_region_label(),
                summary: format!("{rds_not_available} RDS instance(s) are not available"),
                next_step: "Open RDS and investigate instance status and recovery path".into(),
                route: FindingRoute::Rds,
            });
        }

        let single_az_review_instances = self
            .rds_instances
            .iter()
            .filter(|db| db.needs_single_az_review())
            .collect::<Vec<_>>();

        if !single_az_review_instances.is_empty() {
            let sample_instances = single_az_review_instances
                .iter()
                .take(3)
                .map(|db| db.identifier.clone())
                .collect::<Vec<_>>();
            let sample_count = sample_instances.len();
            let remaining = single_az_review_instances
                .len()
                .saturating_sub(sample_count);
            let summary = if remaining > 0 {
                format!(
                    "{} single-AZ RDS instance(s) look production-like: {} (+{} more)",
                    single_az_review_instances.len(),
                    sample_instances.join(", "),
                    remaining
                )
            } else {
                format!(
                    "{} single-AZ RDS instance(s) look production-like: {}",
                    single_az_review_instances.len(),
                    sample_instances.join(", ")
                )
            };

            findings.push(Finding {
                severity: FindingSeverity::Medium,
                category: FindingCategory::Hygiene,
                service: "RDS".into(),
                region: self.current_region_label(),
                summary,
                next_step:
                    "Open RDS and review production-like single-AZ databases for Multi-AZ coverage"
                        .into(),
                route: FindingRoute::Rds,
            });
        }

        let default_vpcs = self.vpcs.iter().filter(|vpc| vpc.is_default).count();

        if default_vpcs > 0 {
            findings.push(Finding {
                severity: FindingSeverity::Medium,
                category: FindingCategory::Hygiene,
                service: "VPC".into(),
                region: self.current_region_label(),
                summary: format!("{default_vpcs} default VPC(s) are still present"),
                next_step: "Review default VPC usage and remove or restrict it if unnecessary"
                    .into(),
                route: FindingRoute::Vpc,
            });
        }

        let load_balancers_with_zero_healthy_targets = self
            .load_balancers
            .iter()
            .filter(|lb| lb.has_zero_healthy_targets())
            .collect::<Vec<_>>();

        if !load_balancers_with_zero_healthy_targets.is_empty() {
            let sample_load_balancers = load_balancers_with_zero_healthy_targets
                .iter()
                .take(3)
                .map(|lb| lb.name.clone())
                .collect::<Vec<_>>();
            let sample_count = sample_load_balancers.len();
            let remaining = load_balancers_with_zero_healthy_targets
                .len()
                .saturating_sub(sample_count);
            let summary = if remaining > 0 {
                format!(
                    "{} load balancer(s) have target groups but zero healthy targets: {} (+{} more)",
                    load_balancers_with_zero_healthy_targets.len(),
                    sample_load_balancers.join(", "),
                    remaining
                )
            } else {
                format!(
                    "{} load balancer(s) have target groups but zero healthy targets: {}",
                    load_balancers_with_zero_healthy_targets.len(),
                    sample_load_balancers.join(", ")
                )
            };

            findings.push(Finding {
                severity: FindingSeverity::High,
                category: FindingCategory::Incident,
                service: "Load Balancers".into(),
                region: self.current_region_label(),
                summary,
                next_step:
                    "Open load balancers and restore healthy registered targets behind the listener"
                        .into(),
                route: FindingRoute::LoadBalancers,
            });
        }

        let load_balancers_with_no_active_targets = self
            .load_balancers
            .iter()
            .filter(|lb| lb.has_no_active_targets() && !lb.has_zero_healthy_targets())
            .collect::<Vec<_>>();

        if !load_balancers_with_no_active_targets.is_empty() {
            let sample_load_balancers = load_balancers_with_no_active_targets
                .iter()
                .take(3)
                .map(|lb| {
                    let signals = lb.review_signals().join("/");
                    format!("{} ({signals})", lb.name)
                })
                .collect::<Vec<_>>();
            let sample_count = sample_load_balancers.len();
            let remaining = load_balancers_with_no_active_targets
                .len()
                .saturating_sub(sample_count);
            let summary = if remaining > 0 {
                format!(
                    "{} load balancer(s) have no active target path: {} (+{} more)",
                    load_balancers_with_no_active_targets.len(),
                    sample_load_balancers.join(", "),
                    remaining
                )
            } else {
                format!(
                    "{} load balancer(s) have no active target path: {}",
                    load_balancers_with_no_active_targets.len(),
                    sample_load_balancers.join(", ")
                )
            };

            findings.push(Finding {
                severity: FindingSeverity::Medium,
                category: FindingCategory::Waste,
                service: "Load Balancers".into(),
                region: self.current_region_label(),
                summary,
                next_step:
                    "Open load balancers and review listeners with no target groups or no registered targets"
                        .into(),
                route: FindingRoute::LoadBalancers,
            });
        }

        let high_memory_functions = self
            .lambda_functions
            .iter()
            .filter(|f| f.has_high_memory())
            .count();

        if high_memory_functions > 0 {
            findings.push(Finding {
                severity: FindingSeverity::Medium,
                category: FindingCategory::Waste,
                service: "Lambda".into(),
                region: self.current_region_label(),
                summary: format!(
                    "{high_memory_functions} function(s) have memory >= {} MB",
                    LambdaFunctionInfo::HIGH_MEMORY_THRESHOLD_MB
                ),
                next_step: "Review high-memory Lambda functions for right-sizing".into(),
                route: FindingRoute::Lambda,
            });
        }

        let stale_functions = self
            .lambda_functions
            .iter()
            .filter(|f| f.is_stale())
            .count();

        if stale_functions > 0 {
            findings.push(Finding {
                severity: FindingSeverity::Medium,
                category: FindingCategory::Waste,
                service: "Lambda".into(),
                region: self.current_region_label(),
                summary: format!(
                    "{stale_functions} function(s) have not been modified in {}+ days",
                    LambdaFunctionInfo::STALE_DEPLOY_DAYS
                ),
                next_step: "Review stale Lambda functions for ownership or cleanup".into(),
                route: FindingRoute::Lambda,
            });
        }

        findings.sort_by(|a, b| {
            a.severity
                .rank()
                .cmp(&b.severity.rank())
                .then_with(|| a.category.as_str().cmp(b.category.as_str()))
                .then_with(|| a.service.cmp(&b.service))
        });

        self.findings = findings;
        self.rebuild_cost_savings();
    }

    fn service_insights_for_aliases<'a>(&'a self, aliases: &[&str]) -> Vec<&'a ServiceCostInsight> {
        self.service_cost_insights
            .iter()
            .filter(|insight| {
                aliases
                    .iter()
                    .any(|alias| insight.service.eq_ignore_ascii_case(alias))
            })
            .collect()
    }

    fn service_cost_for_aliases(&self, aliases: &[&str]) -> f64 {
        self.service_insights_for_aliases(aliases)
            .into_iter()
            .map(|insight| insight.monthly_cost)
            .sum()
    }

    fn service_usage_preview_for_aliases(&self, aliases: &[&str]) -> String {
        let mut usage_lines = self
            .service_insights_for_aliases(aliases)
            .into_iter()
            .flat_map(|insight| insight.top_usage_types.iter().map(|usage| usage.summary()))
            .collect::<Vec<_>>();

        usage_lines.dedup();

        if usage_lines.is_empty() {
            "No usage detail available in cache".into()
        } else {
            usage_lines
                .into_iter()
                .take(2)
                .collect::<Vec<_>>()
                .join(" | ")
        }
    }

    pub fn rebuild_cost_savings(&mut self) {
        let mut opportunities = Vec::new();

        let running_instances = self
            .ec2_instances
            .iter()
            .filter(|instance| instance.is_running())
            .count();
        let low_cpu_instances = self
            .ec2_instances
            .iter()
            .filter(|instance| instance.has_sustained_low_cpu())
            .count();
        let stopped_instances = self
            .ec2_instances
            .iter()
            .filter(|instance| instance.is_stopped())
            .count();
        let total_instances = self.ec2_instances.len();

        let ec2_compute_cost =
            self.service_cost_for_aliases(&["Amazon Elastic Compute Cloud - Compute"]);
        let ec2_other_cost = self.service_cost_for_aliases(&["EC2 - Other"]);
        let ec2_usage = self.service_usage_preview_for_aliases(&[
            "Amazon Elastic Compute Cloud - Compute",
            "EC2 - Other",
        ]);

        if low_cpu_instances > 0 && running_instances > 0 && ec2_compute_cost > 0.0 {
            let ratio = low_cpu_instances as f64 / running_instances as f64;
            let estimated_monthly_savings = ec2_compute_cost * (ratio * 0.5).min(0.40);

            if estimated_monthly_savings > 0.0 {
                opportunities.push(CostSavingsOpportunity {
                    title: "Right-size underused EC2".into(),
                    service: "EC2".into(),
                    monthly_cost: ec2_compute_cost,
                    estimated_monthly_savings,
                    evidence: format!(
                        "{low_cpu_instances} running instance(s) averaged below {:.1}% CPU over the last {} days",
                        Ec2InstanceInfo::LOW_CPU_THRESHOLD_PERCENT,
                        Ec2InstanceInfo::LOW_CPU_LOOKBACK_DAYS
                    ),
                    usage_context: ec2_usage.clone(),
                    recommendation:
                        "Open EC2 and right-size or stop persistently underused instances."
                            .into(),
                    route: SavingsRoute::Ec2,
                });
            }
        }

        if stopped_instances > 0 && total_instances > 0 && ec2_other_cost > 0.0 {
            let ratio = stopped_instances as f64 / total_instances as f64;
            let estimated_monthly_savings = ec2_other_cost * (ratio * 0.6).min(0.35);

            if estimated_monthly_savings > 0.0 {
                opportunities.push(CostSavingsOpportunity {
                    title: "Trim stopped-instance drag".into(),
                    service: "EC2".into(),
                    monthly_cost: ec2_other_cost,
                    estimated_monthly_savings,
                    evidence: format!(
                        "{stopped_instances} stopped instance(s) may still be carrying EBS, IP, or ancillary EC2-Other charges"
                    ),
                    usage_context: self
                        .service_usage_preview_for_aliases(&["EC2 - Other"]),
                    recommendation:
                        "Review stopped EC2 instances and remove unused volumes, IPs, or old support resources."
                            .into(),
                    route: SavingsRoute::Ec2,
                });
            }
        }

        let total_load_balancers = self.load_balancers.len();
        let load_balancers_without_path = self
            .load_balancers
            .iter()
            .filter(|lb| lb.has_no_active_targets())
            .count();
        let orphan_target_groups = self
            .target_groups
            .iter()
            .filter(|tg| tg.is_orphan_candidate())
            .count();
        let elb_cost = self.service_cost_for_aliases(&["Amazon Elastic Load Balancing"]);

        if (load_balancers_without_path > 0 || orphan_target_groups > 0)
            && total_load_balancers > 0
            && elb_cost > 0.0
        {
            let ratio = load_balancers_without_path.max(orphan_target_groups) as f64
                / total_load_balancers.max(1) as f64;
            let estimated_monthly_savings = elb_cost * (ratio * 0.75).min(0.60);

            if estimated_monthly_savings > 0.0 {
                opportunities.push(CostSavingsOpportunity {
                    title: "Clean up idle load-balancing paths".into(),
                    service: "Load Balancing".into(),
                    monthly_cost: elb_cost,
                    estimated_monthly_savings,
                    evidence: format!(
                        "{load_balancers_without_path} load balancer(s) have no active target path and {orphan_target_groups} target group(s) look orphaned"
                    ),
                    usage_context: self
                        .service_usage_preview_for_aliases(&["Amazon Elastic Load Balancing"]),
                    recommendation:
                        "Open load balancers and target groups, then remove idle listeners or detached paths."
                            .into(),
                    route: SavingsRoute::LoadBalancers,
                });
            }
        }

        let lambda_high_memory = self
            .lambda_functions
            .iter()
            .filter(|function| function.has_high_memory())
            .count();
        let lambda_cost = self.service_cost_for_aliases(&["AWS Lambda"]);

        if lambda_high_memory > 0 && !self.lambda_functions.is_empty() && lambda_cost > 0.0 {
            let ratio = lambda_high_memory as f64 / self.lambda_functions.len() as f64;
            let estimated_monthly_savings = lambda_cost * (ratio * 0.5).min(0.35);

            if estimated_monthly_savings > 0.0 {
                opportunities.push(CostSavingsOpportunity {
                    title: "Right-size Lambda memory".into(),
                    service: "Lambda".into(),
                    monthly_cost: lambda_cost,
                    estimated_monthly_savings,
                    evidence: format!(
                        "{lambda_high_memory} function(s) are provisioned at or above {} MB",
                        LambdaFunctionInfo::HIGH_MEMORY_THRESHOLD_MB
                    ),
                    usage_context: self.service_usage_preview_for_aliases(&["AWS Lambda"]),
                    recommendation:
                        "Open Lambda and reduce memory on overprovisioned functions where latency allows."
                            .into(),
                    route: SavingsRoute::Lambda,
                });
            }
        }

        let apis_needing_review = self
            .apigateway_apis
            .iter()
            .filter(|api| api.needs_review())
            .count();
        let api_gateway_cost = self.service_cost_for_aliases(&["Amazon API Gateway"]);

        if apis_needing_review > 0 && !self.apigateway_apis.is_empty() && api_gateway_cost > 0.0 {
            let ratio = apis_needing_review as f64 / self.apigateway_apis.len() as f64;
            let estimated_monthly_savings = api_gateway_cost * (ratio * 0.5).min(0.30);

            if estimated_monthly_savings > 0.0 {
                opportunities.push(CostSavingsOpportunity {
                    title: "Retire stale API Gateway surfaces".into(),
                    service: "API Gateway".into(),
                    monthly_cost: api_gateway_cost,
                    estimated_monthly_savings,
                    evidence: format!(
                        "{apis_needing_review} API(s) look generic or stale enough to review for retirement"
                    ),
                    usage_context: self
                        .service_usage_preview_for_aliases(&["Amazon API Gateway"]),
                    recommendation:
                        "Open API Gateway and remove abandoned APIs or consolidate duplicated endpoints."
                            .into(),
                    route: SavingsRoute::Apigateway,
                });
            }
        }

        opportunities.sort_by(|left, right| {
            right
                .estimated_monthly_savings
                .partial_cmp(&left.estimated_monthly_savings)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        self.cost_savings_opportunities = opportunities;
    }

    pub fn trigger_refresh(&mut self) {
        if self.is_refreshing {
            return;
        }

        self.is_refreshing = true;
        self.account_overview = None;
        self.refresh_phase = RefreshPhase::Overview;
    }

    pub async fn next_region(&mut self) {
        if self.regions.is_empty() {
            return;
        }

        let total_slots = self.region_slot_count();
        self.current_region_index = (self.current_region_index + 1) % total_slots;

        self.persist_region_selection();

        if !self.is_global_region_selected() {
            self.rebuild_aws_clients().await;
        }

        self.trigger_refresh();
    }

    pub async fn previous_region(&mut self) {
        if self.regions.is_empty() {
            return;
        }

        let total_slots = self.region_slot_count();

        if self.current_region_index == 0 {
            self.current_region_index = total_slots - 1;
        } else {
            self.current_region_index -= 1;
        }

        self.persist_region_selection();

        if !self.is_global_region_selected() {
            self.rebuild_aws_clients().await;
        }

        self.trigger_refresh();
    }

    async fn load_lambda(&mut self) {
        let (functions, status) = aws::lambda::fetch_lambda_functions(self).await;
        self.lambda_functions = functions;
        self.lambda_status = status;
    }

    async fn load_apigateway(&mut self) {
        let (apis, status) = aws::apigateway::fetch_apigateway_apis(self).await;
        self.apigateway_apis = apis;
        self.apigateway_status = status;
    }

    async fn load_sqs(&mut self) {
        let (queues, status) = aws::sqs::fetch_sqs_queues(self).await;
        self.sqs_queues_data = queues;
        self.sqs_status = status;
    }

    async fn load_vpcs(&mut self) {
        let (vpcs, status) = aws::vpc::fetch_vpcs(self).await;
        self.vpcs = vpcs;
        self.vpc_status = status;
    }

    async fn load_load_balancers(&mut self) {
        let (load_balancers, status) = aws::elb::fetch_load_balancers(self).await;
        self.load_balancers = load_balancers;
        self.load_balancers_status = status;
    }

    async fn load_target_groups(&mut self) {
        let (target_groups, status) = aws::target_group::fetch_target_groups(self).await;
        self.target_groups = target_groups;
        self.target_groups_status = status;
    }

    async fn load_security_groups(&mut self) {
        let (security_groups, status) = aws::security_group::fetch_security_groups(self).await;
        self.security_groups = security_groups;
        self.security_groups_status = status;
    }

    pub async fn refresh_active(&mut self) {
        self.refresh_phase = RefreshPhase::Overview;
        self.account_overview = None;

        // Always refresh overview (header correctness)
        self.account_overview = Some(aws::account::fetch_account_overview(self).await);

        match self.active_view {
            ActiveView::Findings => {
                self.refresh_phase = RefreshPhase::Services(vec![
                    "CloudWatch",
                    "EC2",
                    "API Gateway",
                    "Secrets",
                    "Security Groups",
                    "Target Groups",
                    "Load Balancers",
                    "SQS",
                    "RDS",
                    "Lambda",
                    "VPC",
                    "Findings",
                ]);
                let (summary, alarms) = aws::cloudwatch::fetch_cloudwatch(self).await;
                self.cloudwatch_summary = summary;
                self.cloudwatch_alarms = alarms;
                self.ec2_instances = aws::ec2::fetch_instances(self).await;
                self.load_apigateway().await;
                let (summary, secrets) = aws::secrets::fetch_secrets(self).await;
                self.secrets_summary = summary;
                self.secrets = secrets;
                self.load_security_groups().await;
                self.load_target_groups().await;
                self.load_load_balancers().await;
                aws::elb::apply_target_group_health(&mut self.load_balancers, &self.target_groups);
                self.load_sqs().await;
                let (summary, instances) = aws::rds::fetch_rds(self).await;
                self.rds_summary = summary;
                self.rds_instances = instances;
                self.load_lambda().await;
                self.load_vpcs().await;
            }
            ActiveView::CostSavings => {
                self.refresh_phase = RefreshPhase::Services(vec![
                    "Cost Explorer",
                    "EC2",
                    "API Gateway",
                    "Lambda",
                    "Load Balancers",
                    "Target Groups",
                ]);
                self.refresh_cost_data(true).await;
                self.ec2_instances = aws::ec2::fetch_instances(self).await;
                self.load_apigateway().await;
                self.load_lambda().await;
                self.load_target_groups().await;
                self.load_load_balancers().await;
                aws::elb::apply_target_group_health(&mut self.load_balancers, &self.target_groups);
            }
            ActiveView::Ec2 => {
                self.refresh_phase = RefreshPhase::Services(vec!["EC2"]);
                self.ec2_instances = aws::ec2::fetch_instances(self).await;
            }

            ActiveView::Lambda => {
                self.refresh_phase = RefreshPhase::Services(vec!["Lambda"]);
                self.load_lambda().await;
            }

            ActiveView::CloudWatch => {
                self.refresh_phase = RefreshPhase::Services(vec!["CloudWatch"]);
                let (summary, alarms) = aws::cloudwatch::fetch_cloudwatch(self).await;
                self.cloudwatch_summary = summary;
                self.cloudwatch_alarms = alarms;
            }

            ActiveView::Vpc => {
                self.refresh_phase = RefreshPhase::Services(vec!["VPC"]);
                self.load_vpcs().await;
            }

            ActiveView::Sqs => {
                self.refresh_phase = RefreshPhase::Services(vec!["SQS"]);
                self.load_sqs().await;
            }

            ActiveView::Apigateway => {
                self.refresh_phase = RefreshPhase::Services(vec!["API Gateway"]);
                self.load_apigateway().await;
            }

            ActiveView::Ecs => {
                self.refresh_phase = RefreshPhase::Services(vec!["ECS"]);
                self.ecs_clusters = aws::ecs::fetch_ecs_clusters(self).await;
            }

            ActiveView::Secrets => {
                self.refresh_phase = RefreshPhase::Services(vec!["Secrets"]);
                let (summary, secrets) = aws::secrets::fetch_secrets(self).await;
                self.secrets_summary = summary;
                self.secrets = secrets;
            }

            ActiveView::Rds => {
                self.refresh_phase = RefreshPhase::Services(vec!["RDS"]);
                let (summary, instances) = aws::rds::fetch_rds(self).await;
                self.rds_summary = summary;
                self.rds_instances = instances;
            }

            ActiveView::LoadBalancers => {
                self.refresh_phase =
                    RefreshPhase::Services(vec!["Load Balancers", "Target Groups"]);
                self.load_target_groups().await;
                self.load_load_balancers().await;
                aws::elb::apply_target_group_health(&mut self.load_balancers, &self.target_groups);
            }

            ActiveView::TargetGroups => {
                self.refresh_phase = RefreshPhase::Services(vec!["Target Groups"]);
                self.load_target_groups().await;
            }

            ActiveView::SecurityGroups => {
                self.load_security_groups().await;
            }

            // Views with no region-scoped data
            ActiveView::AccountOverview => {}
            ActiveView::CostOverview => {
                self.refresh_phase = RefreshPhase::Services(vec!["Cost Explorer"]);
                self.refresh_cost_data(true).await;
            }
        }

        self.rebuild_findings();
        self.refresh_phase = RefreshPhase::Idle;
        self.last_refresh = Some(Utc::now());
        self.is_refreshing = false;
        self.selected_row = 0;
        self.scroll_offset = 0;
    }

    pub async fn load_cost_data(&mut self) {
        self.refresh_cost_data(false).await;
    }

    pub async fn refresh_cost_data(&mut self, force_refresh: bool) {
        if !force_refresh {
            if let Some(cache) = load_if_fresh() {
                let cache_has_usage_insight =
                    !cache.service_cost_insights.is_empty() || cache.service_costs.is_empty();

                if cache_has_usage_insight {
                    self.budget = cache.budget;
                    self.monthly_costs = cache.monthly_costs;
                    self.service_costs = cache.service_costs;
                    self.service_cost_insights = cache.service_cost_insights;
                    self.cost_loaded = true;
                    self.rebuild_cost_savings();
                    return;
                }
            }
        }

        let budget = aws::cost::fetch_budget(self).await;
        let monthly_costs = aws::cost::fetch_last_6_month_costs(self).await;
        let service_cost_insights = aws::cost::fetch_service_cost_insights(self).await;
        let service_costs = service_cost_insights
            .iter()
            .map(|insight| (insight.service.clone(), insight.monthly_cost))
            .collect::<Vec<_>>();

        self.budget = budget.clone();
        self.monthly_costs = monthly_costs.clone();
        self.service_costs = service_costs.clone();
        self.service_cost_insights = service_cost_insights.clone();
        self.cost_loaded = true;
        self.rebuild_cost_savings();

        save(&CostCache {
            fetched_at: Utc::now(),
            budget,
            monthly_costs,
            service_costs,
            service_cost_insights,
        });
    }

    pub fn selected_cost_savings_opportunity(&self) -> Option<&CostSavingsOpportunity> {
        self.cost_savings_opportunities.get(self.selected_row)
    }

    pub async fn open_selected_cost_savings_opportunity(&mut self) {
        let Some(opportunity) = self.selected_cost_savings_opportunity().cloned() else {
            return;
        };

        self.active_view = match opportunity.route {
            SavingsRoute::Ec2 => ActiveView::Ec2,
            SavingsRoute::Lambda => ActiveView::Lambda,
            SavingsRoute::Apigateway => ActiveView::Apigateway,
            SavingsRoute::LoadBalancers => ActiveView::LoadBalancers,
        };

        self.selected_row = 0;
        self.scroll_offset = 0;
        self.on_view_enter().await;
    }

    pub fn selected_resource<'a, T: DescribableResource>(
        &'a self,
        items: &'a [T],
    ) -> Option<&'a T> {
        items.get(self.selected_row)
    }

    pub fn selected_finding(&self) -> Option<&Finding> {
        self.findings.get(self.selected_row)
    }

    fn action_region_for_resource<T: DescribableResource + ?Sized>(&self, resource: &T) -> String {
        resource
            .action_region()
            .unwrap_or_else(|| self.current_region().as_ref())
            .to_string()
    }

    async fn describe_from_resource<T: DescribableResource + ?Sized>(&mut self, resource: &T) {
        let action_region = self.action_region_for_resource(resource);

        let result = if action_region == self.current_region().as_ref() {
            resource.describe(&self.aws).await
        } else {
            let sdk_config = aws::clients::build_sdk_config(
                Region::new(action_region.clone()),
                self.current_profile.as_deref(),
            )
            .await;

            let aws = AwsClients::new(&sdk_config);
            resource.describe(&aws).await
        };

        match result {
            Ok(text) => {
                self.overlay = Some(OverlayState::Describe(DescribeOverlayState::new(
                    resource.resource_name(),
                    text,
                )));
            }
            Err(err) => {
                self.overlay = Some(OverlayState::Describe(DescribeOverlayState::new(
                    "Error".into(),
                    err.to_string(),
                )));
            }
        }
    }

    pub async fn trigger_describe(&mut self) {
        self.footer_mode = FooterMode::Overlay;
        match self.active_view {
            ActiveView::Ec2 => {
                if let Some(instance) = self.selected_resource(&self.ec2_instances).cloned() {
                    self.describe_from_resource(&instance).await;
                    self.footer_mode = FooterMode::Overlay;
                }
            }

            ActiveView::Lambda => {
                if let Some(func) = self.selected_resource(&self.lambda_functions).cloned() {
                    self.describe_from_resource(&func).await;
                }
            }

            ActiveView::CloudWatch => {
                if let Some(items) = self.selected_resource(&self.cloudwatch_alarms).cloned() {
                    self.describe_from_resource(&items).await;
                }
            }

            ActiveView::Secrets => {
                if let Some(items) = self.selected_resource(&self.secrets).cloned() {
                    self.describe_from_resource(&items).await;
                }
            }

            ActiveView::Vpc => {
                if let Some(items) = self.selected_resource(&self.vpcs).cloned() {
                    self.describe_from_resource(&items).await;
                }
            }

            ActiveView::Ecs => {
                if let Some(items) = self.selected_resource(&self.ecs_clusters).cloned() {
                    self.describe_from_resource(&items).await;
                }
            }

            ActiveView::Rds => {
                if let Some(items) = self.selected_resource(&self.rds_instances).cloned() {
                    self.describe_from_resource(&items).await;
                }
            }

            ActiveView::Apigateway => {
                if let Some(items) = self.selected_resource(&self.apigateway_apis).cloned() {
                    self.describe_from_resource(&items).await;
                }
            }

            ActiveView::LoadBalancers => {
                if let Some(lb) = self.selected_resource(&self.load_balancers).cloned() {
                    self.describe_from_resource(&lb).await;
                }
            }

            ActiveView::TargetGroups => {
                if let Some(tg) = self.selected_resource(&self.target_groups).cloned() {
                    self.describe_from_resource(&tg).await;
                }
            }

            ActiveView::SecurityGroups => {
                if let Some(sg) = self.selected_resource(&self.security_groups).cloned() {
                    self.describe_from_resource(&sg).await;
                }
            }
            _ => self.footer_mode = FooterMode::Normal,
        }
    }

    pub fn trigger_open(&mut self) {
        // Always close overlay first
        self.overlay = None;

        match self.active_view {
            ActiveView::Ec2 => {
                if let Some(instance) = self.selected_resource(&self.ec2_instances).cloned() {
                    let region = self.action_region_for_resource(&instance);
                    if let Some(url) = instance.console_url(&region) {
                        if let Err(err) = open_in_browser(&url) {
                            self.notify_error(err);
                        }
                    }
                }
            }

            ActiveView::Lambda => {
                if let Some(func) = self.selected_resource(&self.lambda_functions).cloned() {
                    let region = self.action_region_for_resource(&func);
                    if let Some(url) = func.console_url(&region) {
                        if let Err(err) = open_in_browser(&url) {
                            self.notify_error(err);
                        }
                    }
                }
            }

            ActiveView::CloudWatch => {
                if let Some(item) = self.selected_resource(&self.cloudwatch_alarms).cloned() {
                    let region = self.action_region_for_resource(&item);
                    if let Some(url) = item.console_url(&region) {
                        if let Err(err) = open_in_browser(&url) {
                            self.notify_error(err);
                        }
                    }
                }
            }

            ActiveView::Secrets => {
                if let Some(item) = self.selected_resource(&self.secrets).cloned() {
                    let region = self.action_region_for_resource(&item);
                    if let Some(url) = item.console_url(&region) {
                        if let Err(err) = open_in_browser(&url) {
                            self.notify_error(err);
                        }
                    }
                }
            }

            ActiveView::Vpc => {
                if let Some(item) = self.selected_resource(&self.vpcs).cloned() {
                    let region = self.action_region_for_resource(&item);
                    if let Some(url) = item.console_url(&region) {
                        if let Err(err) = open_in_browser(&url) {
                            self.notify_error(err);
                        }
                    }
                }
            }

            ActiveView::Ecs => {
                if let Some(item) = self.selected_resource(&self.ecs_clusters).cloned() {
                    let region = self.action_region_for_resource(&item);
                    if let Some(url) = item.console_url(&region) {
                        if let Err(err) = open_in_browser(&url) {
                            self.notify_error(err);
                        }
                    }
                }
            }

            ActiveView::Rds => {
                if let Some(item) = self.selected_resource(&self.rds_instances).cloned() {
                    let region = self.action_region_for_resource(&item);
                    if let Some(url) = item.console_url(&region) {
                        if let Err(err) = open_in_browser(&url) {
                            self.notify_error(err);
                        }
                    }
                }
            }

            ActiveView::Apigateway => {
                if let Some(item) = self.selected_resource(&self.apigateway_apis).cloned() {
                    let region = self.action_region_for_resource(&item);
                    if let Some(url) = item.console_url(&region) {
                        if let Err(err) = open_in_browser(&url) {
                            self.notify_error(err);
                        }
                    }
                }
            }

            ActiveView::LoadBalancers => {
                if let Some(lb) = self.selected_resource(&self.load_balancers).cloned() {
                    let region = self.action_region_for_resource(&lb);
                    if let Some(url) = lb.console_url(&region) {
                        if let Err(err) = open_in_browser(&url) {
                            self.notify_error(err);
                        }
                    }
                }
            }

            ActiveView::TargetGroups => {
                if let Some(tg) = self.selected_resource(&self.target_groups).cloned() {
                    let region = self.action_region_for_resource(&tg);
                    if let Some(url) = tg.console_url(&region) {
                        if let Err(err) = open_in_browser(&url) {
                            self.notify_error(err);
                        }
                    }
                }
            }

            ActiveView::SecurityGroups => {
                if let Some(sg) = self.selected_resource(&self.security_groups).cloned() {
                    let region = self.action_region_for_resource(&sg);
                    if let Some(url) = sg.console_url(&region) {
                        if let Err(err) = open_in_browser(&url) {
                            self.notify_error(err);
                        }
                    }
                }
            }

            _ => {}
        }
    }

    fn trigger_cli_for_resource<T: DescribableResource + ?Sized>(&mut self, resource: &T) {
        let region = self.action_region_for_resource(resource);

        if let Some(command) = resource.cli_command(&region) {
            self.overlay = Some(OverlayState::ConfirmCommand(ConfirmCommandState {
                title: format!("AWS CLI for {}", resource.resource_name()),
                command,
                scroll: 0,
            }));
            self.footer_mode = FooterMode::Overlay;
        }
    }

    pub fn trigger_cli(&mut self) {
        self.overlay = None;

        match self.active_view {
            ActiveView::Ec2 => {
                if let Some(instance) = self.selected_resource(&self.ec2_instances).cloned() {
                    self.trigger_cli_for_resource(&instance);
                }
            }
            ActiveView::Lambda => {
                if let Some(func) = self.selected_resource(&self.lambda_functions).cloned() {
                    self.trigger_cli_for_resource(&func);
                }
            }
            ActiveView::CloudWatch => {
                if let Some(item) = self.selected_resource(&self.cloudwatch_alarms).cloned() {
                    self.trigger_cli_for_resource(&item);
                }
            }
            ActiveView::Secrets => {
                if let Some(item) = self.selected_resource(&self.secrets).cloned() {
                    self.trigger_cli_for_resource(&item);
                }
            }
            ActiveView::Vpc => {
                if let Some(item) = self.selected_resource(&self.vpcs).cloned() {
                    self.trigger_cli_for_resource(&item);
                }
            }
            ActiveView::Ecs => {
                if let Some(item) = self.selected_resource(&self.ecs_clusters).cloned() {
                    self.trigger_cli_for_resource(&item);
                }
            }
            ActiveView::Rds => {
                if let Some(item) = self.selected_resource(&self.rds_instances).cloned() {
                    self.trigger_cli_for_resource(&item);
                }
            }
            ActiveView::Apigateway => {
                if let Some(item) = self.selected_resource(&self.apigateway_apis).cloned() {
                    self.trigger_cli_for_resource(&item);
                }
            }
            ActiveView::Sqs => {
                if let Some(item) = self.selected_resource(&self.sqs_queues_data).cloned() {
                    self.trigger_cli_for_resource(&item);
                }
            }
            ActiveView::LoadBalancers => {
                if let Some(item) = self.selected_resource(&self.load_balancers).cloned() {
                    self.trigger_cli_for_resource(&item);
                }
            }
            ActiveView::TargetGroups => {
                if let Some(item) = self.selected_resource(&self.target_groups).cloned() {
                    self.trigger_cli_for_resource(&item);
                }
            }
            ActiveView::SecurityGroups => {
                if let Some(item) = self.selected_resource(&self.security_groups).cloned() {
                    self.trigger_cli_for_resource(&item);
                }
            }
            _ => {}
        }
    }

    pub async fn open_selected_finding(&mut self) {
        let Some(finding) = self.selected_finding().cloned() else {
            return;
        };

        self.active_view = match finding.route {
            FindingRoute::Ec2 => ActiveView::Ec2,
            FindingRoute::CloudWatch => ActiveView::CloudWatch,
            FindingRoute::Lambda => ActiveView::Lambda,
            FindingRoute::Rds => ActiveView::Rds,
            FindingRoute::Apigateway => ActiveView::Apigateway,
            FindingRoute::Secrets => ActiveView::Secrets,
            FindingRoute::Sqs => ActiveView::Sqs,
            FindingRoute::TargetGroups => ActiveView::TargetGroups,
            FindingRoute::LoadBalancers => ActiveView::LoadBalancers,
            FindingRoute::SecurityGroups => ActiveView::SecurityGroups,
            FindingRoute::Vpc => ActiveView::Vpc,
        };

        self.selected_row = 0;
        self.scroll_offset = 0;
        self.on_view_enter().await;
    }

    pub fn trigger_ssh(&mut self) {
        if self.active_view != ActiveView::Ec2 {
            return;
        }

        let Some(instance) = self.selected_resource(&self.ec2_instances).cloned() else {
            return;
        };

        if instance.state != "running" {
            self.overlay = Some(OverlayState::Describe(DescribeOverlayState::new(
                "SSH unavailable".into(),
                "Instance is not running.".into(),
            )));
            return;
        }

        let Some(ctx) = ssh::ssh_command(&instance) else {
            self.overlay = Some(OverlayState::Describe(DescribeOverlayState::new(
                "Private instance".into(),
                "This instance has no public IP.\nSSH requires a bastion or SSM Session Manager."
                    .into(),
            )));
            return;
        };

        // Key-aware branching
        if let Some(key_name) = &ctx.key_name {
            self.overlay = Some(OverlayState::SelectSshKey(SelectSshKeyState {
                title: format!("SSH into {} ({})", ctx.instance_name, key_name),
                context: ctx,
            }));
        } else {
            // No key pair, assume agent
            let cmd = format!("ssh {}@{}", ctx.user, ctx.host);

            self.overlay = Some(OverlayState::ConfirmCommand(ConfirmCommandState {
                title: format!("SSH into {}", ctx.instance_name),
                command: cmd,
                scroll: 0,
            }));
        }
    }
}
