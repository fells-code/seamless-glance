use aws_config::Region;
use chrono::Utc;

use crate::aws::clients::AwsClients;
use crate::cache::cost::{load_if_fresh, save, CostCache};
use crate::license::License;
use crate::models::apigatway::ApiGatewayInfo;
use crate::models::cloudwatch::{CloudWatchAlarm, CloudWatchSummary};
use crate::models::describable::DescribableResource;
use crate::models::ec2::Ec2InstanceInfo;
use crate::models::lambda::LambdaFunctionInfo;
use crate::models::rds::{RdsInstanceInfo, RdsSummary};
use crate::models::secrets::{SecretInfo, SecretsSummary};
use crate::models::service_status::ServiceStatus;
use crate::models::sqs::SqsQueueInfo;
use crate::models::vpc::VpcInfo;
use crate::models::{AccountOverview, BudgetInfo, EcsClusterInfo, EcsServiceInfo, EcsTaskInfo};
use crate::ui::footer::FooterMode;
use crate::ui::open::open_in_browser;
use crate::ui::overlay::describe::DescribeOverlayState;
use crate::ui::theme::Theme;
use crate::{aws, config};

pub enum EcsViewMode {
    Clusters,
    Services(String),
    Tasks {
        cluster_arn: String,
        service_arn: String,
    },
}

#[derive(Debug, Clone)]
pub enum RefreshPhase {
    Idle,
    Overview,
    Services(Vec<&'static str>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveView {
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
}
pub struct App {
    pub aws: AwsClients,
    pub license: Option<License>,
    pub describe_overlay: Option<DescribeOverlayState>,

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
    // Account overview
    pub account_overview: Option<AccountOverview>,
    pub budget: BudgetInfo,
    pub monthly_costs: Vec<f64>,
    pub service_costs: Vec<(String, f64)>,

    // ECS
    pub ecs_view: EcsViewMode,
    pub ecs_clusters: Vec<EcsClusterInfo>,
    pub ecs_services: Vec<EcsServiceInfo>,
    pub ecs_tasks: Vec<EcsTaskInfo>,
    pub ecs_selected_index: usize,

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
}

impl App {
    pub fn new(aws: AwsClients) -> Self {
        Self {
            aws,
            license: None,
            describe_overlay: None,
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
            ecs_view: EcsViewMode::Clusters,
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
            ecs_services: vec![],
            ecs_tasks: vec![],
            ecs_selected_index: 0,

            secrets_summary: SecretsSummary {
                status: ServiceStatus::Unavailable("Not loaded".into()),
                total: 0,
                rotation_disabled: 0,
            },
            secrets: vec![],

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
        }
    }

    // pub async fn load_services_for_cluster(&mut self, cluster_arn: &str) {
    //     self.ecs_services = aws::ecs::fetch_cluster_services(cluster_arn).await;
    //     self.ecs_view = EcsViewMode::ServiceList(cluster_arn.to_string());
    //     self.ecs_selected_index = 0;
    // }

    // pub async fn load_tasks_for_service(&mut self, cluster_arn: &str, service_arn: &str) {
    //     self.ecs_tasks = aws::ecs::fetch_service_tasks(cluster_arn, service_arn).await;
    //     self.ecs_view = EcsViewMode::TaskList {
    //         cluster_arn: cluster_arn.to_string(),
    //         service_arn: service_arn.to_string(),
    //     };
    //     self.ecs_selected_index = 0;
    // }

    // pub fn back_to_clusters(&mut self) {
    //     self.ecs_view = EcsViewMode::ClusterList;
    //     self.ecs_selected_index = 0;
    // }

    // pub async fn back_to_services(&mut self, cluster_arn: &str) {
    //     self.ecs_services = aws::ecs::fetch_cluster_services(cluster_arn).await;
    //     self.ecs_view = EcsViewMode::ServiceList(cluster_arn.to_string());
    //     self.ecs_selected_index = 0;
    // }

    pub fn current_region(&self) -> &Region {
        &self.regions[self.current_region_index]
    }

    pub async fn on_view_enter(&mut self) {
        match self.active_view {
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
            _ => {}
        }
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

        self.current_region_index = (self.current_region_index + 1) % self.regions.len();

        config::save_config(&config::GlanceConfig {
            region: Some(self.current_region().as_ref().to_string()),
            profile: None, // future
        });

        let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::v2025_08_07())
            .region(self.current_region().clone())
            .load()
            .await;

        self.aws = AwsClients::new(&sdk_config);

        self.trigger_refresh();
    }

    pub async fn previous_region(&mut self) {
        if self.regions.is_empty() {
            return;
        }

        if self.current_region_index == 0 {
            self.current_region_index = self.regions.len() - 1;
        } else {
            self.current_region_index -= 1;
        }

        config::save_config(&config::GlanceConfig {
            region: Some(self.current_region().as_ref().to_string()),
            profile: None, // future
        });

        let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::v2025_08_07())
            .region(self.current_region().clone())
            .load()
            .await;

        self.aws = AwsClients::new(&sdk_config);

        self.trigger_refresh();
    }

    pub async fn refresh_active(&mut self) {
        self.refresh_phase = RefreshPhase::Overview;
        self.account_overview = None;

        // Always refresh overview (header correctness)
        self.account_overview = Some(aws::account::fetch_account_overview(self).await);

        // Now refresh ONLY the active view
        match self.active_view {
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

            // Views with no region-scoped data
            ActiveView::AccountOverview | ActiveView::CostOverview => {}
        }

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

    async fn describe_from_resource<T: DescribableResource + ?Sized>(&mut self, resource: &T) {
        match resource.describe(&self.aws).await {
            Ok(text) => {
                self.describe_overlay = Some(DescribeOverlayState {
                    title: resource.resource_name(),
                    content: text,
                    scroll: 0,
                });
            }
            Err(err) => {
                self.describe_overlay = Some(DescribeOverlayState {
                    title: "Error".into(),
                    content: err.to_string(),
                    scroll: 0,
                });
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
            _ => self.footer_mode = FooterMode::Normal,
        }
    }

    pub fn trigger_open(&mut self) {
        // Always close overlay first
        self.describe_overlay = None;

        let region = self.current_region().as_ref();

        match self.active_view {
            ActiveView::Ec2 => {
                if let Some(instance) = self.selected_resource(&self.ec2_instances).cloned() {
                    if let Some(url) = instance.console_url(region) {
                        let _ = open_in_browser(&url);
                    }
                }
            }

            ActiveView::Lambda => {
                if let Some(func) = self.selected_resource(&self.lambda_functions).cloned() {
                    if let Some(url) = func.console_url(region) {
                        let _ = open_in_browser(&url);
                    }
                }
            }

            ActiveView::CloudWatch => {
                if let Some(item) = self.selected_resource(&self.cloudwatch_alarms).cloned() {
                    if let Some(url) = item.console_url(region) {
                        let _ = open_in_browser(&url);
                    }
                }
            }

            ActiveView::Secrets => {
                if let Some(item) = self.selected_resource(&self.secrets).cloned() {
                    if let Some(url) = item.console_url(region) {
                        let _ = open_in_browser(&url);
                    }
                }
            }

            ActiveView::Vpc => {
                if let Some(item) = self.selected_resource(&self.vpcs).cloned() {
                    if let Some(url) = item.console_url(region) {
                        let _ = open_in_browser(&url);
                    }
                }
            }

            ActiveView::Ecs => {
                if let Some(item) = self.selected_resource(&self.ecs_clusters).cloned() {
                    if let Some(url) = item.console_url(region) {
                        let _ = open_in_browser(&url);
                    }
                }
            }

            ActiveView::Rds => {
                if let Some(item) = self.selected_resource(&self.rds_instances).cloned() {
                    if let Some(url) = item.console_url(region) {
                        let _ = open_in_browser(&url);
                    }
                }
            }

            ActiveView::Apigateway => {
                if let Some(item) = self.selected_resource(&self.apigateway_apis).cloned() {
                    if let Some(url) = item.console_url(region) {
                        let _ = open_in_browser(&url);
                    }
                }
            }

            _ => {}
        }
    }
}
