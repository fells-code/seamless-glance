use aws_config::Region;
use chrono::{DateTime, Utc};

use crate::models::{
    AccountOverview, AwsServiceItem, BudgetInfo, EcsClusterInfo, EcsServiceInfo, EcsTaskInfo,
};
use crate::ui::footer::FooterMode;
use crate::ui::theme::Theme;
use crate::{aws, config};

pub enum EcsViewMode {
    ClusterList,
    ServiceList(String),
    TaskList {
        cluster_arn: String,
        service_arn: String,
    },
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
}
pub struct App {
    // Global App properties
    pub regions: Vec<Region>,
    pub current_region_index: usize,
    pub active_view: ActiveView,
    pub should_quit: bool,
    pub command_mode: bool,
    pub command_input: String,
    pub show_help: bool,

    pub last_refresh: Option<chrono::DateTime<chrono::Utc>>,
    pub is_refreshing: bool,

    pub refresh_interval_secs: u64,
    pub last_refresh_attempt: Option<DateTime<Utc>>,
    pub footer_mode: FooterMode,

    pub theme: Theme,
    // Account overview
    pub account_overview: Option<AccountOverview>,
    pub services: Vec<AwsServiceItem>,
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
    pub lambda_functions: Vec<crate::aws::lambda::LambdaFunctionInfo>,

    // APIGW
    pub apigateway_apis: Vec<aws::apigateway::ApiGatewayInfo>,

    // SQS
    pub sqs_queues_data: Vec<aws::sqs::SqsQueueInfo>,

    // VPC
    pub vpcs: Vec<crate::aws::vpc::VpcInfo>,
}

impl App {
    pub fn new() -> Self {
        Self {
            account_overview: None,
            regions: vec![],
            current_region_index: 0,
            should_quit: false,
            command_mode: false,
            command_input: String::new(),
            active_view: ActiveView::AccountOverview,
            services: vec![],
            budget: BudgetInfo {
                monthly_budget: 0.0,
                month_to_date_cost: 0.0,
                forecast: 0.0,
            },
            monthly_costs: vec![0.0; 6],
            service_costs: vec![],
            theme: Theme::seamless(),
            ecs_view: EcsViewMode::ClusterList,
            lambda_functions: vec![],
            apigateway_apis: vec![],
            sqs_queues_data: vec![],
            vpcs: vec![],
            ecs_clusters: vec![],
            ecs_services: vec![],
            ecs_tasks: vec![],
            ecs_selected_index: 0,

            last_refresh: None,
            is_refreshing: false,

            refresh_interval_secs: 30,
            last_refresh_attempt: None,
            footer_mode: FooterMode::Normal,
            show_help: false,
        }
    }

    // pub async fn load_account_overview(&mut self, region: &aws_config::Region) {
    //     self.account_overview = Some(crate::aws::account::fetch_account_overview(region).await);
    // }

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
                self.lambda_functions = aws::lambda::fetch_lambda_functions().await;
            }

            ActiveView::Ecs => {
                // optional: preload ECS clusters if empty
                if self.ecs_clusters.is_empty() {
                    self.ecs_clusters = aws::ecs::fetch_ecs_clusters().await;
                }
            }

            ActiveView::Apigateway => {
                self.apigateway_apis = aws::apigateway::fetch_apigateway_apis().await;
            }

            ActiveView::Sqs => {
                self.sqs_queues_data = aws::sqs::fetch_sqs_queues().await;
            }

            ActiveView::Vpc => {
                self.vpcs = aws::vpc::fetch_vpcs().await;
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

        self.trigger_refresh();
    }

    pub fn should_auto_refresh(&self) -> bool {
        if self.is_refreshing {
            return false;
        }

        let Some(last) = self.last_refresh_attempt else {
            return true;
        };

        let elapsed = (Utc::now() - last).num_seconds();
        elapsed >= self.refresh_interval_secs as i64
    }

    pub fn trigger_auto_refresh(&mut self) {
        self.is_refreshing = true;
        self.last_refresh_attempt = Some(Utc::now());
        self.account_overview = None; // loading UX
    }
}
