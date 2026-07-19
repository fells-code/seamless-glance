//! Non-blocking refresh: AWS fetches run on a spawned task and stream their
//! results back to the UI over a channel, so the event loop keeps drawing and
//! accepting input (and the per-service progress phases actually render).

use std::time::{Duration, Instant};

use tokio::sync::mpsc::UnboundedSender;

use crate::app::{ActiveView, App, RefreshPhase};
use crate::aws;
use crate::models::apigatway::ApiGatewayInfo;
use crate::models::cloudwatch::{CloudWatchAlarm, CloudWatchSummary};
use crate::models::ec2::Ec2InstanceInfo;
use crate::models::elb::LoadBalancerInfo;
use crate::models::lambda::LambdaFunctionInfo;
use crate::models::rds::{RdsInstanceInfo, RdsSummary};
use crate::models::secrets::{SecretInfo, SecretsSummary};
use crate::models::security_group::SecurityGroupInfo;
use crate::models::service_status::ServiceStatus;
use crate::models::sqs::SqsQueueInfo;
use crate::models::target_group::TargetGroupInfo;
use crate::models::vpc::VpcInfo;
use crate::models::{AccountOverview, BudgetInfo, EcsClusterInfo, ServiceCostInsight};

/// A single result streamed from the refresh worker back to the app. The worker
/// sends `Phase` updates as it advances and one payload update per service it
/// fetched, then `Done`.
pub enum RefreshUpdate {
    Phase(RefreshPhase),
    AccountOverview(Box<AccountOverview>),
    Ec2(Vec<Ec2InstanceInfo>, ServiceStatus),
    Lambda(Vec<LambdaFunctionInfo>, ServiceStatus),
    Apigateway(Vec<ApiGatewayInfo>, ServiceStatus),
    Sqs(Vec<SqsQueueInfo>, ServiceStatus),
    Vpc(Vec<VpcInfo>, ServiceStatus),
    LoadBalancers(Vec<LoadBalancerInfo>, ServiceStatus),
    TargetGroups(Vec<TargetGroupInfo>, ServiceStatus),
    SecurityGroups(Vec<SecurityGroupInfo>, ServiceStatus),
    Ecs(Vec<EcsClusterInfo>),
    Secrets(SecretsSummary, Vec<SecretInfo>),
    Rds(RdsSummary, Vec<RdsInstanceInfo>),
    CloudWatch(CloudWatchSummary, Vec<CloudWatchAlarm>),
    Cost {
        budget: BudgetInfo,
        monthly_costs: Vec<f64>,
        service_costs: Vec<(String, f64)>,
        service_cost_insights: Vec<ServiceCostInsight>,
        status: ServiceStatus,
    },
    Done,
}

/// How long a fetched inventory is served from memory before a view switch
/// refetches it. Navigation within this window is instant and never re-hits
/// AWS; a manual refresh (`r`) or a profile/region switch always refetches.
const INVENTORY_TTL: Duration = Duration::from_secs(60);

/// The per-service inventories whose freshness is tracked so navigation can
/// serve cached data instead of always refetching.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum InventoryKind {
    AccountOverview,
    Ec2,
    Lambda,
    Apigateway,
    Sqs,
    Vpc,
    LoadBalancers,
    TargetGroups,
    SecurityGroups,
    Ecs,
    Secrets,
    Rds,
    CloudWatch,
    Cost,
}

impl RefreshUpdate {
    /// The inventory a payload update refreshes, if any. `Phase` and `Done`
    /// carry no inventory.
    fn inventory_kind(&self) -> Option<InventoryKind> {
        Some(match self {
            RefreshUpdate::AccountOverview(_) => InventoryKind::AccountOverview,
            RefreshUpdate::Ec2(..) => InventoryKind::Ec2,
            RefreshUpdate::Lambda(..) => InventoryKind::Lambda,
            RefreshUpdate::Apigateway(..) => InventoryKind::Apigateway,
            RefreshUpdate::Sqs(..) => InventoryKind::Sqs,
            RefreshUpdate::Vpc(..) => InventoryKind::Vpc,
            RefreshUpdate::LoadBalancers(..) => InventoryKind::LoadBalancers,
            RefreshUpdate::TargetGroups(..) => InventoryKind::TargetGroups,
            RefreshUpdate::SecurityGroups(..) => InventoryKind::SecurityGroups,
            RefreshUpdate::Ecs(_) => InventoryKind::Ecs,
            RefreshUpdate::Secrets(..) => InventoryKind::Secrets,
            RefreshUpdate::Rds(..) => InventoryKind::Rds,
            RefreshUpdate::CloudWatch(..) => InventoryKind::CloudWatch,
            RefreshUpdate::Cost { .. } => InventoryKind::Cost,
            RefreshUpdate::Phase(_) | RefreshUpdate::Done => return None,
        })
    }
}

impl App {
    /// Run the refresh for `self.active_view` on a `worker` app, sending each
    /// result over `tx`. Consumes the worker (it is a throwaway clone of the
    /// account context spawned onto its own task). Errors from a closed channel
    /// are ignored: they only happen when a newer refresh has superseded this one.
    pub(crate) async fn stream_refresh(mut self, tx: UnboundedSender<RefreshUpdate>) {
        let _ = tx.send(RefreshUpdate::Phase(RefreshPhase::Overview));
        let overview = aws::account::fetch_account_overview(&self).await;
        let _ = tx.send(RefreshUpdate::AccountOverview(Box::new(overview)));

        match self.active_view {
            ActiveView::Findings => {
                let _ = tx.send(RefreshUpdate::Phase(RefreshPhase::Services(vec![
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
                ])));

                // These are all independent, so run them concurrently: refresh
                // latency becomes the slowest single fetch, not their sum.
                let (
                    (cw_summary, cw_alarms),
                    (ec2, ec2_status),
                    (apis, apigw_status),
                    (secrets_summary, secrets),
                    (sgs, sg_status),
                    (target_groups, tg_status, load_balancers, lb_status),
                    (queues, sqs_status),
                    (rds_summary, rds_instances),
                    (functions, lambda_status),
                    (vpcs, vpc_status),
                ) = tokio::join!(
                    aws::cloudwatch::fetch_cloudwatch(&self),
                    aws::ec2::fetch_instances(&self),
                    aws::apigateway::fetch_apigateway_apis(&self),
                    aws::secrets::fetch_secrets(&self),
                    aws::security_group::fetch_security_groups(&self),
                    self.fetch_target_groups_and_load_balancers(),
                    aws::sqs::fetch_sqs_queues(&self),
                    aws::rds::fetch_rds(&self),
                    aws::lambda::fetch_lambda_functions(&self),
                    aws::vpc::fetch_vpcs(&self),
                );

                let _ = tx.send(RefreshUpdate::CloudWatch(cw_summary, cw_alarms));
                let _ = tx.send(RefreshUpdate::Ec2(ec2, ec2_status));
                let _ = tx.send(RefreshUpdate::Apigateway(apis, apigw_status));
                let _ = tx.send(RefreshUpdate::Secrets(secrets_summary, secrets));
                let _ = tx.send(RefreshUpdate::SecurityGroups(sgs, sg_status));
                let _ = tx.send(RefreshUpdate::TargetGroups(target_groups, tg_status));
                let _ = tx.send(RefreshUpdate::LoadBalancers(load_balancers, lb_status));
                let _ = tx.send(RefreshUpdate::Sqs(queues, sqs_status));
                let _ = tx.send(RefreshUpdate::Rds(rds_summary, rds_instances));
                let _ = tx.send(RefreshUpdate::Lambda(functions, lambda_status));
                let _ = tx.send(RefreshUpdate::Vpc(vpcs, vpc_status));
            }
            ActiveView::CostSavings => {
                // Cost data mutates the worker (cache + fields), so fetch it
                // first; the resource inventories then run concurrently.
                phase(&tx, "Cost Explorer");
                self.send_cost(&tx).await;

                let _ = tx.send(RefreshUpdate::Phase(RefreshPhase::Services(vec![
                    "EC2",
                    "API Gateway",
                    "Lambda",
                    "Target Groups",
                    "Load Balancers",
                ])));

                let (
                    (ec2, ec2_status),
                    (apis, apigw_status),
                    (functions, lambda_status),
                    (target_groups, tg_status, load_balancers, lb_status),
                ) = tokio::join!(
                    aws::ec2::fetch_instances(&self),
                    aws::apigateway::fetch_apigateway_apis(&self),
                    aws::lambda::fetch_lambda_functions(&self),
                    self.fetch_target_groups_and_load_balancers(),
                );

                let _ = tx.send(RefreshUpdate::Ec2(ec2, ec2_status));
                let _ = tx.send(RefreshUpdate::Apigateway(apis, apigw_status));
                let _ = tx.send(RefreshUpdate::Lambda(functions, lambda_status));
                let _ = tx.send(RefreshUpdate::TargetGroups(target_groups, tg_status));
                let _ = tx.send(RefreshUpdate::LoadBalancers(load_balancers, lb_status));
            }
            ActiveView::Ec2 => {
                phase(&tx, "EC2");
                let (instances, status) = aws::ec2::fetch_instances(&self).await;
                let _ = tx.send(RefreshUpdate::Ec2(instances, status));
            }
            ActiveView::Lambda => {
                phase(&tx, "Lambda");
                let (functions, status) = aws::lambda::fetch_lambda_functions(&self).await;
                let _ = tx.send(RefreshUpdate::Lambda(functions, status));
            }
            ActiveView::CloudWatch => {
                phase(&tx, "CloudWatch");
                let (summary, alarms) = aws::cloudwatch::fetch_cloudwatch(&self).await;
                let _ = tx.send(RefreshUpdate::CloudWatch(summary, alarms));
            }
            ActiveView::Vpc => {
                phase(&tx, "VPC");
                let (vpcs, status) = aws::vpc::fetch_vpcs(&self).await;
                let _ = tx.send(RefreshUpdate::Vpc(vpcs, status));
            }
            ActiveView::Sqs => {
                phase(&tx, "SQS");
                let (queues, status) = aws::sqs::fetch_sqs_queues(&self).await;
                let _ = tx.send(RefreshUpdate::Sqs(queues, status));
            }
            ActiveView::Apigateway => {
                phase(&tx, "API Gateway");
                let (apis, status) = aws::apigateway::fetch_apigateway_apis(&self).await;
                let _ = tx.send(RefreshUpdate::Apigateway(apis, status));
            }
            ActiveView::Ecs => {
                phase(&tx, "ECS");
                let clusters = aws::ecs::fetch_ecs_clusters(&self).await;
                let _ = tx.send(RefreshUpdate::Ecs(clusters));
            }
            ActiveView::Secrets => {
                phase(&tx, "Secrets");
                let (summary, secrets) = aws::secrets::fetch_secrets(&self).await;
                let _ = tx.send(RefreshUpdate::Secrets(summary, secrets));
            }
            ActiveView::Rds => {
                phase(&tx, "RDS");
                let (summary, instances) = aws::rds::fetch_rds(&self).await;
                let _ = tx.send(RefreshUpdate::Rds(summary, instances));
            }
            ActiveView::LoadBalancers => {
                let _ = tx.send(RefreshUpdate::Phase(RefreshPhase::Services(vec![
                    "Target Groups",
                    "Load Balancers",
                ])));
                let (target_groups, tg_status, load_balancers, lb_status) =
                    self.fetch_target_groups_and_load_balancers().await;
                let _ = tx.send(RefreshUpdate::TargetGroups(target_groups, tg_status));
                let _ = tx.send(RefreshUpdate::LoadBalancers(load_balancers, lb_status));
            }
            ActiveView::TargetGroups => {
                phase(&tx, "Target Groups");
                let (groups, status) = aws::target_group::fetch_target_groups(&self).await;
                let _ = tx.send(RefreshUpdate::TargetGroups(groups, status));
            }
            ActiveView::SecurityGroups => {
                phase(&tx, "Security Groups");
                let (groups, status) = aws::security_group::fetch_security_groups(&self).await;
                let _ = tx.send(RefreshUpdate::SecurityGroups(groups, status));
            }
            ActiveView::AccountOverview => {}
            ActiveView::CostOverview => {
                phase(&tx, "Cost Explorer");
                self.send_cost(&tx).await;
            }
        }

        let _ = tx.send(RefreshUpdate::Done);
    }

    /// Target groups must be fetched before load balancers so target-group
    /// health can be folded in, so this pair is fetched together and returned as
    /// a unit. Borrows `&self` only, so it can join the concurrent fetch set.
    async fn fetch_target_groups_and_load_balancers(
        &self,
    ) -> (
        Vec<TargetGroupInfo>,
        ServiceStatus,
        Vec<LoadBalancerInfo>,
        ServiceStatus,
    ) {
        let (target_groups, tg_status) = aws::target_group::fetch_target_groups(self).await;
        let (mut load_balancers, lb_status) = aws::elb::fetch_load_balancers(self).await;
        aws::elb::apply_target_group_health(&mut load_balancers, &target_groups);
        (target_groups, tg_status, load_balancers, lb_status)
    }

    async fn send_cost(&mut self, tx: &UnboundedSender<RefreshUpdate>) {
        self.refresh_cost_data(true).await;
        let _ = tx.send(RefreshUpdate::Cost {
            budget: self.budget.clone(),
            monthly_costs: self.monthly_costs.clone(),
            service_costs: self.service_costs.clone(),
            service_cost_insights: self.service_cost_insights.clone(),
            status: self.cost_status.clone(),
        });
    }

    /// The inventories the given view reads, so navigation knows what must be
    /// fresh before it can skip a refetch. Every view depends on the account
    /// overview (the worker fetches it first for header correctness).
    fn required_inventories(view: ActiveView) -> &'static [InventoryKind] {
        use InventoryKind::*;
        match view {
            ActiveView::Findings => &[
                AccountOverview,
                CloudWatch,
                Ec2,
                Apigateway,
                Secrets,
                SecurityGroups,
                TargetGroups,
                LoadBalancers,
                Sqs,
                Rds,
                Lambda,
                Vpc,
            ],
            ActiveView::CostSavings => &[
                AccountOverview,
                Cost,
                Ec2,
                Apigateway,
                Lambda,
                TargetGroups,
                LoadBalancers,
            ],
            ActiveView::CostOverview => &[AccountOverview, Cost],
            ActiveView::Ec2 => &[AccountOverview, Ec2],
            ActiveView::Lambda => &[AccountOverview, Lambda],
            ActiveView::CloudWatch => &[AccountOverview, CloudWatch],
            ActiveView::Vpc => &[AccountOverview, Vpc],
            ActiveView::Sqs => &[AccountOverview, Sqs],
            ActiveView::Apigateway => &[AccountOverview, Apigateway],
            ActiveView::Ecs => &[AccountOverview, Ecs],
            ActiveView::Secrets => &[AccountOverview, Secrets],
            ActiveView::Rds => &[AccountOverview, Rds],
            ActiveView::LoadBalancers => &[AccountOverview, TargetGroups, LoadBalancers],
            ActiveView::TargetGroups => &[AccountOverview, TargetGroups],
            ActiveView::SecurityGroups => &[AccountOverview, SecurityGroups],
            ActiveView::AccountOverview => &[AccountOverview],
        }
    }

    /// True when every inventory the active view needs was fetched within the
    /// TTL under the current context, so a view switch can serve cached data
    /// instead of refetching. Freshness is cleared on a profile or region
    /// change, so this can never serve a prior context's data.
    pub(crate) fn active_view_is_fresh(&self) -> bool {
        let now = Instant::now();
        Self::required_inventories(self.active_view)
            .iter()
            .all(|kind| {
                self.inventory_fetched_at
                    .get(kind)
                    .is_some_and(|fetched| now.duration_since(*fetched) < INVENTORY_TTL)
            })
    }

    /// Apply one streamed update to the live app state. Returns `true` when the
    /// refresh finished (the caller then rebuilds derived state and clears the
    /// refreshing flag).
    pub(crate) fn apply_refresh_update(&mut self, update: RefreshUpdate) -> bool {
        if let Some(kind) = update.inventory_kind() {
            self.inventory_fetched_at.insert(kind, Instant::now());
        }

        match update {
            RefreshUpdate::Phase(phase) => self.refresh_phase = phase,
            RefreshUpdate::AccountOverview(overview) => self.account_overview = Some(*overview),
            RefreshUpdate::Ec2(instances, status) => {
                self.ec2_instances = instances;
                self.ec2_status = status;
            }
            RefreshUpdate::Lambda(functions, status) => {
                self.lambda_functions = functions;
                self.lambda_status = status;
            }
            RefreshUpdate::Apigateway(apis, status) => {
                self.apigateway_apis = apis;
                self.apigateway_status = status;
            }
            RefreshUpdate::Sqs(queues, status) => {
                self.sqs_queues_data = queues;
                self.sqs_status = status;
            }
            RefreshUpdate::Vpc(vpcs, status) => {
                self.vpcs = vpcs;
                self.vpc_status = status;
            }
            RefreshUpdate::LoadBalancers(load_balancers, status) => {
                self.load_balancers = load_balancers;
                self.load_balancers_status = status;
            }
            RefreshUpdate::TargetGroups(target_groups, status) => {
                self.target_groups = target_groups;
                self.target_groups_status = status;
            }
            RefreshUpdate::SecurityGroups(groups, status) => {
                self.security_groups = groups;
                self.security_groups_status = status;
            }
            RefreshUpdate::Ecs(clusters) => self.ecs_clusters = clusters,
            RefreshUpdate::Secrets(summary, secrets) => {
                self.secrets_summary = summary;
                self.secrets = secrets;
            }
            RefreshUpdate::Rds(summary, instances) => {
                self.rds_summary = summary;
                self.rds_instances = instances;
            }
            RefreshUpdate::CloudWatch(summary, alarms) => {
                self.cloudwatch_summary = summary;
                self.cloudwatch_alarms = alarms;
            }
            RefreshUpdate::Cost {
                budget,
                monthly_costs,
                service_costs,
                service_cost_insights,
                status,
            } => {
                self.budget = budget;
                self.monthly_costs = monthly_costs;
                self.service_costs = service_costs;
                self.service_cost_insights = service_cost_insights;
                self.cost_status = status;
                self.cost_loaded = true;
            }
            RefreshUpdate::Done => return true,
        }
        false
    }

    /// Drain any pending refresh updates, applying each. When the worker signals
    /// completion, rebuild derived state and clear the refreshing flag.
    pub fn drain_refresh_updates(&mut self) {
        let Some(mut rx) = self.refresh_rx.take() else {
            return;
        };

        let mut done = false;
        loop {
            match rx.try_recv() {
                Ok(update) => {
                    if self.apply_refresh_update(update) {
                        done = true;
                        break;
                    }
                }
                // Nothing more right now, but the worker is still running.
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                // Worker finished or died without a final Done; wrap up either way.
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    done = true;
                    break;
                }
            }
        }

        if done {
            self.finish_refresh();
        } else {
            // Not finished and the worker is still alive: keep the receiver.
            self.refresh_rx = Some(rx);
        }
    }

    fn finish_refresh(&mut self) {
        self.rebuild_findings();
        self.rebuild_cost_savings();
        self.refresh_phase = RefreshPhase::Idle;
        self.last_refresh = Some(chrono::Utc::now());
        self.is_refreshing = false;
        self.in_flight_refresh = None;
        self.selected_row = 0;
        self.scroll_offset = 0;
    }
}

fn phase(tx: &UnboundedSender<RefreshUpdate>, service: &'static str) {
    let _ = tx.send(RefreshUpdate::Phase(RefreshPhase::Services(vec![service])));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::clients::AwsClients;

    fn test_app() -> App {
        let config = aws_config::SdkConfig::builder()
            .region(aws_config::Region::new("us-east-1"))
            .behavior_version(aws_config::BehaviorVersion::latest())
            .build();
        App::new(AwsClients::new(&config))
    }

    #[test]
    fn payload_updates_land_on_the_matching_fields() {
        let mut app = test_app();

        assert!(!app.apply_refresh_update(RefreshUpdate::Ec2(vec![], ServiceStatus::Ok)));
        app.apply_refresh_update(RefreshUpdate::Lambda(vec![], ServiceStatus::AccessDenied));
        assert!(matches!(app.lambda_status, ServiceStatus::AccessDenied));

        app.apply_refresh_update(RefreshUpdate::Phase(RefreshPhase::Services(vec!["EC2"])));
        assert!(matches!(app.refresh_phase, RefreshPhase::Services(_)));
    }

    #[test]
    fn done_update_signals_completion() {
        let mut app = test_app();
        assert!(!app.apply_refresh_update(RefreshUpdate::Ec2(vec![], ServiceStatus::Ok)));
        assert!(app.apply_refresh_update(RefreshUpdate::Done));
    }

    #[test]
    fn cost_update_marks_cost_loaded() {
        let mut app = test_app();
        assert!(!app.cost_loaded);
        app.apply_refresh_update(RefreshUpdate::Cost {
            budget: app.budget.clone(),
            monthly_costs: vec![0.0; 6],
            service_costs: vec![],
            service_cost_insights: vec![],
            status: ServiceStatus::Ok,
        });
        assert!(app.cost_loaded);
        assert!(matches!(app.cost_status, ServiceStatus::Ok));
    }

    fn mark_fresh(app: &mut App, kind: InventoryKind) {
        app.inventory_fetched_at.insert(kind, Instant::now());
    }

    #[test]
    fn a_view_is_fresh_only_when_all_its_inventories_were_fetched() {
        let mut app = test_app();
        app.active_view = ActiveView::Ec2;

        // Nothing fetched yet: must refetch.
        assert!(!app.active_view_is_fresh());

        // The account overview alone is not enough for the EC2 view.
        mark_fresh(&mut app, InventoryKind::AccountOverview);
        assert!(!app.active_view_is_fresh());

        // With EC2 also fetched (through the real apply path, which stamps
        // freshness), the EC2 view is served from cache.
        app.apply_refresh_update(RefreshUpdate::Ec2(vec![], ServiceStatus::Ok));
        assert!(app.active_view_is_fresh());

        // A different view whose inventory was never fetched still refetches.
        app.active_view = ActiveView::Lambda;
        assert!(!app.active_view_is_fresh());
    }

    #[test]
    fn clearing_service_data_resets_freshness() {
        let mut app = test_app();
        app.active_view = ActiveView::Ec2;
        mark_fresh(&mut app, InventoryKind::AccountOverview);
        mark_fresh(&mut app, InventoryKind::Ec2);
        assert!(app.active_view_is_fresh());

        app.clear_service_data();
        assert!(!app.active_view_is_fresh());
    }
}
