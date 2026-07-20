use aws_config::Region;
use chrono::Utc;
use std::collections::HashMap;
use std::time::Instant;

use crate::app::findings::{build_findings, FindingContext};
use crate::aws::clients::AwsClients;
use crate::aws::pricing::PriceBook;
use crate::cache::cost::{load_if_fresh, save, CostCache};
use crate::models::apigateway::ApiGatewayInfo;
use crate::models::cloudwatch::{CloudWatchAlarm, CloudWatchSummary};
use crate::models::describable::DescribableResource;
use crate::models::ec2::Ec2InstanceInfo;
use crate::models::elb::LoadBalancerInfo;
use crate::models::finding::{Finding, FindingRoute};
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

mod findings;
mod refresh;
mod services;

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
    // Narrows the active view's rows to those matching the query. Selection
    // indexes the filtered rows, so everything that resolves a selection has to
    // go through `visible_indices`.
    pub filter_mode: bool,
    pub row_filter: String,
    pub show_help: bool,
    pub scroll_offset: u16,
    pub selected_row: usize,
    pub wrap_text: bool,
    pub detail_scroll_offset: u16,
    pub last_refresh: Option<chrono::DateTime<chrono::Utc>>,
    pub is_refreshing: bool,
    pub refresh_phase: RefreshPhase,
    // Receiver for results streamed from the in-flight refresh worker task.
    refresh_rx: Option<tokio::sync::mpsc::UnboundedReceiver<refresh::RefreshUpdate>>,
    // (profile, region, view) the in-flight refresh is fetching for, so an
    // identical re-trigger is deduped while a context or view change supersedes.
    in_flight_refresh: Option<(Option<String>, String, ActiveView)>,
    // When each service inventory was last fetched under the current context, so
    // a view switch can serve fresh-enough data instead of refetching. Cleared
    // on a profile or region change alongside the inventory itself.
    inventory_fetched_at: HashMap<refresh::InventoryKind, Instant>,
    // (profile, region label) the currently held per-service data was fetched
    // under. When it changes, stale data is cleared so findings are never built
    // from a prior region or profile and mislabeled with the new one.
    data_context: Option<(Option<String>, String)>,

    // Set when the config on disk is unreadable and could not be backed up.
    // Saving would overwrite it, so preference writes stop for the session.
    config_writes_blocked: bool,

    pub footer_mode: FooterMode,
    pub notification: Option<Notification>,

    pub theme: Theme,
    pub theme_name: ThemeName,
    pub findings: Vec<Finding>,
    // Account overview
    pub account_overview: Option<AccountOverview>,
    pub budget: BudgetInfo,
    pub cost_status: ServiceStatus,
    pub monthly_costs: Vec<f64>,
    pub service_costs: Vec<(String, f64)>,
    pub service_cost_insights: Vec<ServiceCostInsight>,
    pub cost_savings_opportunities: Vec<CostSavingsOpportunity>,
    // List prices for resource types the waste findings can cost. Loaded from
    // disk at startup and topped up as unseen resource types appear.
    pub prices: PriceBook,

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
    pub ec2_status: ServiceStatus,

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
            filter_mode: false,
            row_filter: String::new(),
            active_view: ActiveView::Findings,
            budget: BudgetInfo {
                monthly_budget: 0.0,
                month_to_date_cost: 0.0,
                forecast: 0.0,
                forecast_low: None,
                forecast_high: None,
            },
            cost_status: ServiceStatus::Unavailable("Not loaded".into()),
            monthly_costs: vec![0.0; 6],
            service_costs: vec![],
            service_cost_insights: vec![],
            cost_savings_opportunities: vec![],
            prices: crate::cache::pricing::load_if_fresh().unwrap_or_default(),
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
            ec2_status: ServiceStatus::Unavailable("Not loaded".into()),
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
            refresh_rx: None,
            in_flight_refresh: None,
            inventory_fetched_at: HashMap::new(),
            data_context: None,
            config_writes_blocked: false,
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

    /// True when a modal surface (command palette, help, or an overlay) owns
    /// input. Navigation and resource-action keys gate on this so they cannot
    /// mutate view or region state while a modal is open.
    pub fn modal_open(&self) -> bool {
        self.command_mode || self.show_help || self.overlay.is_some()
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

    pub fn persist_region_selection(&mut self) {
        self.persist_preferences();
    }

    pub fn persist_preferences(&mut self) {
        // The config on disk could not be read and could not be moved aside,
        // so writing would destroy preferences that are still recoverable by
        // hand. Better to lose this session's change than the whole file.
        if self.config_writes_blocked {
            return;
        }

        let loaded = config::load_config();
        if loaded.blocks_saving() {
            self.config_writes_blocked = true;
            if let Some(warning) = loaded.warning() {
                self.notify_error(warning);
            }
            return;
        }

        let mut cfg = loaded.config();
        cfg.region = Some(self.current_region_label());
        cfg.theme = Some(self.theme_name.as_str().to_string());
        cfg.profile = self.current_profile.clone();

        if let Err(err) = config::save_config(&cfg) {
            self.notify_error(format!("Could not save preferences: {err}"));
        }
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
        // The filter is scoped to the view it was typed in. Carrying it across
        // would silently hide rows in a view the operator never filtered.
        self.filter_mode = false;
        self.row_filter.clear();
        self.reset_row_selection();

        // Serve cached inventory when it is still fresh so navigation does not
        // re-hit AWS on every view switch. A manual refresh (`r`) and profile or
        // region switches call `trigger_refresh` directly and always refetch.
        if !self.active_view_is_fresh() {
            self.trigger_refresh();
        }
    }

    fn view_uses_free_scroll(&self) -> bool {
        matches!(self.active_view, ActiveView::AccountOverview)
    }

    /// Whether this view can show a selected row in full.
    ///
    /// Every list view can: the table shortens values to their column width, so
    /// wrap mode is how a value too long to fit is read. Account overview paints
    /// a fixed layout with no row to expand.
    pub fn active_view_supports_wrap(&self) -> bool {
        self.active_view != ActiveView::AccountOverview
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

    /// Indices into the active view's backing data for the rows currently
    /// shown, in display order.
    ///
    /// Every caller that turns a selection into a resource goes through this.
    /// Resolving `selected_row` against the unfiltered data would act on a
    /// different resource than the highlighted one.
    /// Cost insights ranked by spend.
    ///
    /// The cost overview displays this order, so the registry has to match it:
    /// a selection there indexes the ranking, not the fetch order.
    pub fn sorted_cost_insights(&self) -> Vec<ServiceCostInsight> {
        let mut sorted = self.service_cost_insights.clone();
        sorted.sort_by(|a, b| {
            b.monthly_cost
                .partial_cmp(&a.monthly_cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted
    }

    pub fn visible_indices(&self) -> Vec<usize> {
        let row_text = (services::entry_for(self.active_view).row_text)(self);
        let needle = self.row_filter.trim().to_lowercase();

        if needle.is_empty() {
            return (0..row_text.len()).collect();
        }

        row_text
            .iter()
            .enumerate()
            .filter(|(_, text)| text.to_lowercase().contains(&needle))
            .map(|(index, _)| index)
            .collect()
    }

    /// How many rows the active view has before filtering, for reporting how
    /// much a filter is hiding.
    pub fn total_row_count(&self) -> usize {
        (services::entry_for(self.active_view).row_text)(self).len()
    }

    pub fn filter_is_active(&self) -> bool {
        !self.row_filter.trim().is_empty()
    }

    fn active_view_item_count(&self) -> usize {
        self.visible_indices().len()
    }

    /// Put the cursor back at the top of the list.
    ///
    /// Called whenever the row set changes underneath the selection, since a
    /// row index carries no meaning across a different set of rows.
    pub fn reset_row_selection(&mut self) {
        self.selected_row = 0;
        self.scroll_offset = 0;
        self.detail_scroll_offset = 0;
    }

    pub fn open_filter(&mut self) {
        self.filter_mode = true;
        self.footer_mode = FooterMode::Filter;
    }

    /// Leave filter entry, keeping the query and the rows it selected.
    pub fn commit_filter(&mut self) {
        self.filter_mode = false;
        self.footer_mode = FooterMode::Normal;
    }

    /// Leave filter entry and restore the full row set.
    pub fn clear_filter(&mut self) {
        self.filter_mode = false;
        self.row_filter.clear();
        self.footer_mode = FooterMode::Normal;
        self.reset_row_selection();
    }

    pub fn push_filter_char(&mut self, c: char) {
        self.row_filter.push(c);
        self.reset_row_selection();
    }

    pub fn pop_filter_char(&mut self) {
        self.row_filter.pop();
        self.reset_row_selection();
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
        let region_label = self.current_region_label();
        let ctx = FindingContext {
            account_overview: self.account_overview.as_ref(),
            region_label: &region_label,
            cloudwatch_alarms: &self.cloudwatch_alarms,
            secrets: &self.secrets,
            ec2_instances: &self.ec2_instances,
            target_groups: &self.target_groups,
            load_balancers: &self.load_balancers,
            apigateway_apis: &self.apigateway_apis,
            sqs_queues_data: &self.sqs_queues_data,
            rds_instances: &self.rds_instances,
            security_groups: &self.security_groups,
            vpcs: &self.vpcs,
            lambda_functions: &self.lambda_functions,
            prices: &self.prices,
        };

        self.findings = build_findings(&ctx);
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
        let want = (
            self.current_profile.clone(),
            self.current_region_label(),
            self.active_view,
        );

        // A refresh for the exact same profile, region, and view is already in
        // flight; leave it alone. A context or view change supersedes it below.
        if self.is_refreshing && self.in_flight_refresh.as_ref() == Some(&want) {
            return;
        }

        // Clear stale inventory when the account context (profile + region)
        // changes so findings are never rebuilt from a prior context's data.
        let data_context = (want.0.clone(), want.1.clone());
        if self.data_context.as_ref() != Some(&data_context) {
            self.clear_service_data();
            self.data_context = Some(data_context);
        }

        self.is_refreshing = true;
        self.account_overview = None;
        self.refresh_phase = RefreshPhase::Overview;
        self.in_flight_refresh = Some(want);

        // Run the fetches on a throwaway clone of the account context so the
        // event loop keeps drawing and handling input while they run. Replacing
        // the receiver supersedes any older in-flight refresh (its sends drop).
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        self.refresh_rx = Some(rx);

        let mut worker = App::new(self.aws.clone());
        worker.regions = self.regions.clone();
        worker.current_region_index = self.current_region_index;
        worker.current_profile = self.current_profile.clone();
        worker.active_view = self.active_view;

        tokio::spawn(worker.stream_refresh(tx));
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

    /// Drop every per-service inventory so a region or profile switch cannot
    /// leave a prior context's resources to be rebuilt into findings and stamped
    /// with the new region. The refresh worker refetches the fresh data.
    fn clear_service_data(&mut self) {
        self.inventory_fetched_at.clear();
        self.ec2_instances.clear();
        self.lambda_functions.clear();
        self.apigateway_apis.clear();
        self.sqs_queues_data.clear();
        self.vpcs.clear();
        self.load_balancers.clear();
        self.target_groups.clear();
        self.security_groups.clear();
        self.ecs_clusters.clear();
        self.secrets.clear();
        self.rds_instances.clear();
        self.cloudwatch_alarms.clear();
        self.findings.clear();

        let not_loaded = ServiceStatus::Unavailable("Not loaded".into());
        self.ec2_status = not_loaded.clone();
        self.lambda_status = not_loaded.clone();
        self.apigateway_status = not_loaded.clone();
        self.sqs_status = not_loaded.clone();
        self.vpc_status = not_loaded.clone();
        self.load_balancers_status = not_loaded.clone();
        self.target_groups_status = not_loaded.clone();
        self.security_groups_status = not_loaded.clone();
        self.rds_summary.status = not_loaded.clone();
        self.secrets_summary.status = not_loaded.clone();
        self.cloudwatch_summary.status = not_loaded;
    }

    pub async fn load_cost_data(&mut self) {
        self.refresh_cost_data(false).await;
    }

    pub async fn refresh_cost_data(&mut self, force_refresh: bool) {
        let profile = self.current_profile.clone();
        let region = self.current_region_label().to_string();

        if !force_refresh {
            if let Some(cache) = load_if_fresh(profile.as_deref(), &region) {
                let cache_has_usage_insight =
                    !cache.service_cost_insights.is_empty() || cache.service_costs.is_empty();

                if cache_has_usage_insight {
                    self.budget = cache.budget;
                    self.monthly_costs = cache.monthly_costs;
                    self.service_costs = cache.service_costs;
                    self.service_cost_insights = cache.service_cost_insights;
                    // Only successful fetches are cached, so a cache hit is Ok data.
                    self.cost_status = ServiceStatus::Ok;
                    self.cost_loaded = true;
                    self.rebuild_cost_savings();
                    return;
                }
            }
        }

        let (budget, budget_status) = aws::cost::fetch_budget(self).await;
        let (monthly_costs, monthly_status) = aws::cost::fetch_last_6_month_costs(self).await;
        let (service_cost_insights, insights_status) =
            aws::cost::fetch_service_cost_insights(self).await;
        let cost_status = [budget_status, monthly_status, insights_status]
            .into_iter()
            .find(|status| !matches!(status, ServiceStatus::Ok))
            .unwrap_or(ServiceStatus::Ok);
        let service_costs = service_cost_insights
            .iter()
            .map(|insight| (insight.service.clone(), insight.monthly_cost))
            .collect::<Vec<_>>();

        self.budget = budget.clone();
        self.monthly_costs = monthly_costs.clone();
        self.service_costs = service_costs.clone();
        self.service_cost_insights = service_cost_insights.clone();
        self.cost_status = cost_status.clone();
        self.cost_loaded = true;
        self.rebuild_cost_savings();

        // Only persist a successful fetch. Caching a denied or throttled result
        // would reintroduce the misleading $0 overview on the next launch for the
        // cache TTL.
        if matches!(cost_status, ServiceStatus::Ok) {
            save(&CostCache {
                fetched_at: Utc::now(),
                profile,
                region,
                budget,
                monthly_costs,
                service_costs,
                service_cost_insights,
            });
        }
    }

    pub fn selected_cost_savings_opportunity(&self) -> Option<&CostSavingsOpportunity> {
        let index = *self.visible_indices().get(self.selected_row)?;
        self.cost_savings_opportunities.get(index)
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

    /// The selected row of `items`, resolved through the active filter so it is
    /// the row the operator can actually see.
    pub fn selected_resource<'a, T: DescribableResource>(
        &'a self,
        items: &'a [T],
    ) -> Option<&'a T> {
        let index = *self.visible_indices().get(self.selected_row)?;
        items.get(index)
    }

    /// The selected row of the active view, when that view is backed by
    /// resources that support describe/open/CLI.
    fn selected_describable(&self) -> Option<Box<dyn DescribableResource>> {
        match services::entry_for(self.active_view).rows {
            services::ViewRows::Resources(selected) => selected(self),
            services::ViewRows::Summary => None,
        }
    }

    pub fn selected_finding(&self) -> Option<&Finding> {
        let index = *self.visible_indices().get(self.selected_row)?;
        self.findings.get(index)
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

        match self.selected_describable() {
            Some(resource) => self.describe_from_resource(resource.as_ref()).await,
            None => self.footer_mode = FooterMode::Normal,
        }
    }

    pub fn trigger_open(&mut self) {
        // Always close overlay first
        self.overlay = None;

        let Some(resource) = self.selected_describable() else {
            return;
        };

        let region = self.action_region_for_resource(resource.as_ref());
        if let Some(url) = resource.console_url(&region) {
            if let Err(err) = open_in_browser(&url) {
                self.notify_error(err);
            }
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

        if let Some(resource) = self.selected_describable() {
            self.trigger_cli_for_resource(resource.as_ref());
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
