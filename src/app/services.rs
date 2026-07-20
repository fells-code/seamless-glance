//! Single source of truth for per-view resource access.
//!
//! Every view is registered exactly once here with the two things the generic
//! handlers need: how many rows it has, and how to borrow the selected row as a
//! `DescribableResource`. Describe, open, and CLI then share one code path, so a
//! view cannot support one of them and silently miss the others.

use crate::app::{ActiveView, App};
use crate::models::describable::DescribableResource;

/// How a view exposes its selected row to the describe/open/CLI handlers.
///
/// `Resources` views back their rows with a `DescribableResource`; `Summary`
/// views (findings, cost, account overview) render aggregates that have no
/// single underlying resource to act on.
pub enum ViewRows {
    Resources(fn(&App) -> Option<Box<dyn DescribableResource>>),
    Summary,
}

pub struct ServiceEntry {
    pub view: ActiveView,
    pub item_count: fn(&App) -> usize,
    pub rows: ViewRows,
}

/// Take the selected row of `field` as an owned trait object.
///
/// The row is cloned out because every caller goes on to take `&mut App`, so it
/// cannot keep borrowing the collection it came from.
///
/// Written as a macro rather than a generic helper because each entry names a
/// different `App` field, and `fn` pointers cannot capture.
macro_rules! resources {
    ($field:ident) => {
        ViewRows::Resources(|app| {
            app.$field
                .get(app.selected_row)
                .map(|item| Box::new(item.clone()) as Box<dyn DescribableResource>)
        })
    };
}

macro_rules! count {
    ($field:ident) => {
        |app| app.$field.len()
    };
}

/// Rows rendered by the account overview view. The view paints a fixed layout
/// rather than a list, so the count is not derived from a collection.
const ACCOUNT_OVERVIEW_ROWS: usize = 10;

pub const SERVICES: &[ServiceEntry] = &[
    ServiceEntry {
        view: ActiveView::Findings,
        item_count: count!(findings),
        rows: ViewRows::Summary,
    },
    ServiceEntry {
        view: ActiveView::AccountOverview,
        item_count: |_| ACCOUNT_OVERVIEW_ROWS,
        rows: ViewRows::Summary,
    },
    ServiceEntry {
        view: ActiveView::CostOverview,
        item_count: count!(service_cost_insights),
        rows: ViewRows::Summary,
    },
    ServiceEntry {
        view: ActiveView::CostSavings,
        item_count: count!(cost_savings_opportunities),
        rows: ViewRows::Summary,
    },
    ServiceEntry {
        view: ActiveView::Ecs,
        item_count: count!(ecs_clusters),
        rows: resources!(ecs_clusters),
    },
    ServiceEntry {
        view: ActiveView::Ec2,
        item_count: count!(ec2_instances),
        rows: resources!(ec2_instances),
    },
    ServiceEntry {
        view: ActiveView::Rds,
        item_count: count!(rds_instances),
        rows: resources!(rds_instances),
    },
    ServiceEntry {
        view: ActiveView::Lambda,
        item_count: count!(lambda_functions),
        rows: resources!(lambda_functions),
    },
    ServiceEntry {
        view: ActiveView::Apigateway,
        item_count: count!(apigateway_apis),
        rows: resources!(apigateway_apis),
    },
    ServiceEntry {
        view: ActiveView::Sqs,
        item_count: count!(sqs_queues_data),
        rows: resources!(sqs_queues_data),
    },
    ServiceEntry {
        view: ActiveView::Vpc,
        item_count: count!(vpcs),
        rows: resources!(vpcs),
    },
    ServiceEntry {
        view: ActiveView::Secrets,
        item_count: count!(secrets),
        rows: resources!(secrets),
    },
    ServiceEntry {
        view: ActiveView::CloudWatch,
        item_count: count!(cloudwatch_alarms),
        rows: resources!(cloudwatch_alarms),
    },
    ServiceEntry {
        view: ActiveView::LoadBalancers,
        item_count: count!(load_balancers),
        rows: resources!(load_balancers),
    },
    ServiceEntry {
        view: ActiveView::TargetGroups,
        item_count: count!(target_groups),
        rows: resources!(target_groups),
    },
    ServiceEntry {
        view: ActiveView::SecurityGroups,
        item_count: count!(security_groups),
        rows: resources!(security_groups),
    },
];

pub fn entry_for(view: ActiveView) -> &'static ServiceEntry {
    SERVICES
        .iter()
        .find(|entry| entry.view == view)
        .expect("every ActiveView variant is registered in SERVICES")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::clients::AwsClients;
    use crate::models::sqs::SqsQueueInfo;
    use crate::models::tags::Tags;

    /// Mirrors `ActiveView` so a new variant fails here until it is registered,
    /// which is what keeps describe/open/CLI from drifting apart per service.
    const ALL_VIEWS: &[ActiveView] = &[
        ActiveView::Findings,
        ActiveView::AccountOverview,
        ActiveView::CostOverview,
        ActiveView::CostSavings,
        ActiveView::Ecs,
        ActiveView::Ec2,
        ActiveView::Rds,
        ActiveView::Lambda,
        ActiveView::Apigateway,
        ActiveView::Sqs,
        ActiveView::Vpc,
        ActiveView::Secrets,
        ActiveView::CloudWatch,
        ActiveView::LoadBalancers,
        ActiveView::TargetGroups,
        ActiveView::SecurityGroups,
    ];

    fn test_app() -> App {
        let config = aws_config::SdkConfig::builder()
            .region(aws_config::Region::new("us-east-1"))
            .behavior_version(aws_config::BehaviorVersion::latest())
            .build();
        App::new(AwsClients::new(&config))
    }

    /// The SQS view used to have a CLI arm but no describe or open arm, so the
    /// footer advertised `d`/`o` and both silently did nothing.
    #[test]
    fn the_sqs_view_resolves_a_selected_resource() {
        let mut app = test_app();
        app.active_view = ActiveView::Sqs;
        app.sqs_queues_data = vec![SqsQueueInfo {
            name: "orders".into(),
            queue_url: "https://sqs.us-east-1.amazonaws.com/1/orders".into(),
            is_fifo: false,
            messages_available: 0,
            messages_in_flight: 0,
            has_dlq: false,
            tags: Tags::empty(),
        }];
        app.selected_row = 0;

        let selected = app.selected_describable().expect("sqs row is describable");
        assert_eq!(selected.resource_name(), "orders");
        assert!(selected.console_url("us-east-1").is_some());
        assert!(selected.cli_command("us-east-1").is_some());
    }

    #[test]
    fn every_resource_view_reports_no_selection_when_empty() {
        let mut app = test_app();
        for view in ALL_VIEWS {
            app.active_view = *view;
            assert!(
                app.selected_describable().is_none(),
                "{view:?} yielded a resource from empty state"
            );
        }
    }

    #[test]
    fn every_view_is_registered_exactly_once() {
        for view in ALL_VIEWS {
            let matches = SERVICES.iter().filter(|e| e.view == *view).count();
            assert_eq!(matches, 1, "{view:?} must be registered exactly once");
        }
        assert_eq!(SERVICES.len(), ALL_VIEWS.len());
    }

    #[test]
    fn resource_views_expose_describe_open_and_cli() {
        for view in ALL_VIEWS {
            let entry = entry_for(*view);
            let is_summary = matches!(entry.rows, ViewRows::Summary);
            let expected_summary = matches!(
                view,
                ActiveView::Findings
                    | ActiveView::AccountOverview
                    | ActiveView::CostOverview
                    | ActiveView::CostSavings
            );
            assert_eq!(
                is_summary, expected_summary,
                "{view:?} row access does not match its view kind"
            );
        }
    }
}
