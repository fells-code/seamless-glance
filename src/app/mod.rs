use aws_config::Region;
use chrono::Utc;
use std::collections::BTreeSet;

use crate::aws::clients::AwsClients;
use crate::cache::cost::{load_if_fresh, save, CostCache};
use crate::license::License;
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
use crate::models::{AccountOverview, BudgetInfo, EcsClusterInfo};
use crate::resources::ssh;
use crate::ui::footer::FooterMode;
use crate::ui::open::open_in_browser;
use crate::ui::overlay::overlays::{
    ConfirmCommandState, DescribeOverlayState, OverlayState, SelectSshKeyState,
};
use crate::ui::theme::Theme;
use crate::{aws, config};

#[derive(Debug, Clone)]
pub enum RefreshPhase {
    Idle,
    Overview,
    Services(Vec<&'static str>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveView {
    Findings,
    AccountOverview,
    CostOverview,
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
    pub license: Option<License>,
    pub overlay: Option<OverlayState>,

    // Global App properties
    pub cost_loaded: bool,
    pub regions: Vec<Region>,
    pub current_region_index: usize,
    pub active_view: ActiveView,
    pub should_quit: bool,
    pub command_mode: bool,
    pub command_input: String,
    pub show_help: bool,
    pub scroll_offset: u16,
    pub selected_row: usize,
    pub last_refresh: Option<chrono::DateTime<chrono::Utc>>,
    pub is_refreshing: bool,
    pub refresh_phase: RefreshPhase,

    pub footer_mode: FooterMode,

    pub theme: Theme,
    pub findings: Vec<Finding>,
    // Account overview
    pub account_overview: Option<AccountOverview>,
    pub budget: BudgetInfo,
    pub monthly_costs: Vec<f64>,
    pub service_costs: Vec<(String, f64)>,

    // ECS
    pub ecs_clusters: Vec<EcsClusterInfo>,

    // Lambda
    pub lambda_functions: Vec<LambdaFunctionInfo>,

    // APIGW
    pub apigateway_apis: Vec<ApiGatewayInfo>,

    // SQS
    pub sqs_queues_data: Vec<SqsQueueInfo>,

    // VPC
    pub vpcs: Vec<VpcInfo>,

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

    // Target Groups
    pub target_groups: Vec<TargetGroupInfo>,

    // Security Groups
    pub security_groups: Vec<SecurityGroupInfo>,
}

impl App {
    pub fn new(aws: AwsClients) -> Self {
        Self {
            aws,
            license: None,
            overlay: None,
            scroll_offset: 0,
            selected_row: 0,
            cost_loaded: false,
            account_overview: None,
            regions: vec![],
            current_region_index: 0,
            should_quit: false,
            command_mode: false,
            command_input: String::new(),
            active_view: ActiveView::AccountOverview,
            budget: BudgetInfo {
                monthly_budget: 0.0,
                month_to_date_cost: 0.0,
                forecast: 0.0,
            },
            monthly_costs: vec![0.0; 6],
            service_costs: vec![],
            theme: Theme::seamless(),
            findings: vec![],
            lambda_functions: vec![],
            apigateway_apis: vec![],
            sqs_queues_data: vec![],
            vpcs: vec![],
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
            show_help: false,

            target_groups: vec![],
            security_groups: vec![],
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
        config::save_config(&config::GlanceConfig {
            region: Some(self.current_region_label()),
            profile: None,
        });
    }

    pub async fn set_region_by_index(&mut self, index: usize) {
        if self.regions.is_empty() {
            return;
        }

        if index >= self.regions.len() {
            return;
        }

        self.current_region_index = index;

        let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::v2026_01_12())
            .region(self.current_region().clone())
            .load()
            .await;

        self.aws = AwsClients::new(&sdk_config);
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
        match self.active_view {
            ActiveView::Findings => {
                self.refresh_phase = RefreshPhase::Services(vec!["Security Groups"]);
                self.security_groups = aws::security_group::fetch_security_groups(self).await;
                self.rebuild_findings();
                self.refresh_phase = RefreshPhase::Idle;
            }
            ActiveView::Lambda => {
                self.lambda_functions = aws::lambda::fetch_lambda_functions(self).await;
            }

            ActiveView::Ecs => {
                if self.ecs_clusters.is_empty() {
                    self.ecs_clusters = aws::ecs::fetch_ecs_clusters(self).await;
                }
            }

            ActiveView::Apigateway => {
                self.apigateway_apis = aws::apigateway::fetch_apigateway_apis(self).await;
            }

            ActiveView::Sqs => {
                self.sqs_queues_data = aws::sqs::fetch_sqs_queues(self).await;
            }

            ActiveView::Vpc => {
                self.vpcs = aws::vpc::fetch_vpcs(self).await;
            }

            ActiveView::Ec2 => {
                self.ec2_instances = aws::ec2::fetch_instances(self).await;
            }
            ActiveView::CloudWatch => {
                let (summary, alarms) = aws::cloudwatch::fetch_cloudwatch(self).await;
                self.cloudwatch_summary = summary;
                self.cloudwatch_alarms = alarms;
            }
            ActiveView::Secrets => {
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
                self.load_balancers = aws::elb::fetch_load_balancers(self).await;
            }

            ActiveView::TargetGroups => {
                self.target_groups = aws::target_group::fetch_target_groups(self).await;
            }

            ActiveView::SecurityGroups => {
                self.security_groups = aws::security_group::fetch_security_groups(self).await;
            }

            _ => {}
        }
    }

    pub fn rebuild_findings(&mut self) {
        let mut findings = Vec::new();

        if let Some(overview) = &self.account_overview {
            if overview.alarms.alarms_in_alarm > 0 {
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

            if overview.target_groups_unhealthy > 0 {
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

            if overview.secrets.rotation_disabled > 0 {
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

            if overview.ec2_stopped > 0 {
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

        findings.sort_by(|a, b| {
            a.severity
                .rank()
                .cmp(&b.severity.rank())
                .then_with(|| a.category.as_str().cmp(b.category.as_str()))
                .then_with(|| a.service.cmp(&b.service))
        });

        self.findings = findings;
    }

    pub fn trigger_refresh(&mut self) {
        if self.is_refreshing {
            return;
        }

        self.is_refreshing = true;
        self.account_overview = None;
    }

    pub async fn next_region(&mut self) {
        if self.regions.is_empty() {
            return;
        }

        let total_slots = self.region_slot_count();
        self.current_region_index = (self.current_region_index + 1) % total_slots;

        self.persist_region_selection();

        if !self.is_global_region_selected() {
            let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::v2026_01_12())
                .region(self.current_region().clone())
                .load()
                .await;

            self.aws = AwsClients::new(&sdk_config);
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
            let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::v2026_01_12())
                .region(self.current_region().clone())
                .load()
                .await;

            self.aws = AwsClients::new(&sdk_config);
        }

        self.trigger_refresh();
    }

    pub async fn refresh_active(&mut self) {
        self.refresh_phase = RefreshPhase::Overview;
        self.account_overview = None;

        // Always refresh overview (header correctness)
        self.account_overview = Some(aws::account::fetch_account_overview(self).await);

        match self.active_view {
            ActiveView::Findings => {
                self.refresh_phase = RefreshPhase::Services(vec!["Security Groups", "Findings"]);
                self.security_groups = aws::security_group::fetch_security_groups(self).await;
            }
            ActiveView::Ec2 => {
                self.refresh_phase = RefreshPhase::Services(vec!["EC2"]);
                self.ec2_instances = aws::ec2::fetch_instances(self).await;
            }

            ActiveView::Lambda => {
                self.refresh_phase = RefreshPhase::Services(vec!["Lambda"]);
                self.lambda_functions = aws::lambda::fetch_lambda_functions(self).await;
            }

            ActiveView::CloudWatch => {
                self.refresh_phase = RefreshPhase::Services(vec!["CloudWatch"]);
                let (summary, alarms) = aws::cloudwatch::fetch_cloudwatch(self).await;
                self.cloudwatch_summary = summary;
                self.cloudwatch_alarms = alarms;
            }

            ActiveView::Vpc => {
                self.refresh_phase = RefreshPhase::Services(vec!["VPC"]);
                self.vpcs = aws::vpc::fetch_vpcs(self).await;
            }

            ActiveView::Sqs => {
                self.refresh_phase = RefreshPhase::Services(vec!["SQS"]);
                self.sqs_queues_data = aws::sqs::fetch_sqs_queues(self).await;
            }

            ActiveView::Apigateway => {
                self.refresh_phase = RefreshPhase::Services(vec!["API Gateway"]);
                self.apigateway_apis = aws::apigateway::fetch_apigateway_apis(self).await;
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
                self.refresh_phase = RefreshPhase::Services(vec!["Load Balancers"]);
                let lbs = aws::elb::fetch_load_balancers(self).await;
                self.load_balancers = lbs;
            }

            ActiveView::TargetGroups => {
                self.target_groups = aws::target_group::fetch_target_groups(self).await;
            }

            ActiveView::SecurityGroups => {
                self.security_groups = aws::security_group::fetch_security_groups(self).await;
            }

            // Views with no region-scoped data
            ActiveView::AccountOverview | ActiveView::CostOverview => {}
        }

        self.rebuild_findings();
        self.refresh_phase = RefreshPhase::Idle;
        self.last_refresh = Some(Utc::now());
        self.is_refreshing = false;
        self.selected_row = 0;
        self.scroll_offset = 0;
    }

    pub async fn load_cost_data(&mut self) {
        if let Some(cache) = load_if_fresh() {
            self.budget = cache.budget;
            self.monthly_costs = cache.monthly_costs;
            self.service_costs = cache.service_costs;
            self.cost_loaded = true;
            return;
        }

        // Fetch fresh data
        let budget = aws::cost::fetch_budget(self).await;
        let monthly_costs = aws::cost::fetch_last_6_month_costs(self).await;
        let service_costs = aws::cost::fetch_service_cost_breakdown(self).await;

        self.budget = budget.clone();
        self.monthly_costs = monthly_costs.clone();
        self.service_costs = service_costs.clone();
        self.cost_loaded = true;

        save(&CostCache {
            fetched_at: Utc::now(),
            budget,
            monthly_costs,
            service_costs,
        });
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
            let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::v2026_01_12())
                .region(Region::new(action_region.clone()))
                .load()
                .await;

            let aws = AwsClients::new(&sdk_config);
            resource.describe(&aws).await
        };

        match result {
            Ok(text) => {
                self.overlay = Some(OverlayState::Describe(DescribeOverlayState {
                    title: resource.resource_name(),
                    content: text,
                    scroll: 0,
                }));
            }
            Err(err) => {
                self.overlay = Some(OverlayState::Describe(DescribeOverlayState {
                    title: "Error".into(),
                    content: err.to_string(),
                    scroll: 0,
                }));
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
                        let _ = open_in_browser(&url);
                    }
                }
            }

            ActiveView::Lambda => {
                if let Some(func) = self.selected_resource(&self.lambda_functions).cloned() {
                    let region = self.action_region_for_resource(&func);
                    if let Some(url) = func.console_url(&region) {
                        let _ = open_in_browser(&url);
                    }
                }
            }

            ActiveView::CloudWatch => {
                if let Some(item) = self.selected_resource(&self.cloudwatch_alarms).cloned() {
                    let region = self.action_region_for_resource(&item);
                    if let Some(url) = item.console_url(&region) {
                        let _ = open_in_browser(&url);
                    }
                }
            }

            ActiveView::Secrets => {
                if let Some(item) = self.selected_resource(&self.secrets).cloned() {
                    let region = self.action_region_for_resource(&item);
                    if let Some(url) = item.console_url(&region) {
                        let _ = open_in_browser(&url);
                    }
                }
            }

            ActiveView::Vpc => {
                if let Some(item) = self.selected_resource(&self.vpcs).cloned() {
                    let region = self.action_region_for_resource(&item);
                    if let Some(url) = item.console_url(&region) {
                        let _ = open_in_browser(&url);
                    }
                }
            }

            ActiveView::Ecs => {
                if let Some(item) = self.selected_resource(&self.ecs_clusters).cloned() {
                    let region = self.action_region_for_resource(&item);
                    if let Some(url) = item.console_url(&region) {
                        let _ = open_in_browser(&url);
                    }
                }
            }

            ActiveView::Rds => {
                if let Some(item) = self.selected_resource(&self.rds_instances).cloned() {
                    let region = self.action_region_for_resource(&item);
                    if let Some(url) = item.console_url(&region) {
                        let _ = open_in_browser(&url);
                    }
                }
            }

            ActiveView::Apigateway => {
                if let Some(item) = self.selected_resource(&self.apigateway_apis).cloned() {
                    let region = self.action_region_for_resource(&item);
                    if let Some(url) = item.console_url(&region) {
                        let _ = open_in_browser(&url);
                    }
                }
            }

            ActiveView::LoadBalancers => {
                if let Some(lb) = self.selected_resource(&self.load_balancers).cloned() {
                    let region = self.action_region_for_resource(&lb);
                    if let Some(url) = lb.console_url(&region) {
                        let _ = open_in_browser(&url);
                    }
                }
            }

            ActiveView::TargetGroups => {
                if let Some(tg) = self.selected_resource(&self.target_groups).cloned() {
                    let region = self.action_region_for_resource(&tg);
                    if let Some(url) = tg.console_url(&region) {
                        let _ = open_in_browser(&url);
                    }
                }
            }

            ActiveView::SecurityGroups => {
                if let Some(sg) = self.selected_resource(&self.security_groups).cloned() {
                    let region = self.action_region_for_resource(&sg);
                    if let Some(url) = sg.console_url(&region) {
                        let _ = open_in_browser(&url);
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
            FindingRoute::Secrets => ActiveView::Secrets,
            FindingRoute::TargetGroups => ActiveView::TargetGroups,
            FindingRoute::SecurityGroups => ActiveView::SecurityGroups,
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
            self.overlay = Some(OverlayState::Describe(DescribeOverlayState {
                title: "SSH unavailable".into(),
                content: "Instance is not running.".into(),
                scroll: 0,
            }));
            return;
        }

        let Some(ctx) = ssh::ssh_command(&instance) else {
            self.overlay = Some(OverlayState::Describe(DescribeOverlayState {
            title: "Private instance".into(),
            content:
                "This instance has no public IP.\nSSH requires a bastion or SSM Session Manager."
                    .into(),
            scroll: 0,
        }));
            return;
        };

        // Key-aware branching
        if ctx.key_name.is_some() {
            self.overlay = Some(OverlayState::SelectSshKey(SelectSshKeyState {
                title: format!(
                    "SSH into {} ({})",
                    ctx.instance_name,
                    ctx.key_name.as_ref().unwrap()
                ),
                context: ctx,
                selected: 0,
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
