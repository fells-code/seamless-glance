//! Single source of truth for per-view resource access.
//!
//! Every view is registered exactly once here with what the generic handlers
//! need: the text each row is searchable by, and how to borrow the selected row
//! as a `DescribableResource`. Describe, open, and CLI share one code path, so a
//! view cannot support one of them and silently miss the others.
//!
//! Row access goes through the active filter. Selection indexes the rows the
//! operator can see, so resolving it against the unfiltered data would act on a
//! different resource than the highlighted one.

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
    /// What each row is matched against when a filter is active, in the same
    /// order as the view's backing data.
    pub row_text: fn(&App) -> Vec<String>,
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
            let index = *app.visible_indices().get(app.selected_row)?;

            app.$field
                .get(index)
                .map(|item| Box::new(item.clone()) as Box<dyn DescribableResource>)
        })
    };
}

/// Search text for a view, built from the fields the operator can see.
macro_rules! row_text {
    ($field:ident, |$item:ident| $text:expr) => {
        |app| {
            app.$field
                .iter()
                .map(|$item| $text)
                .collect::<Vec<String>>()
        }
    };
}

/// Rows rendered by the account overview view. The view paints a fixed layout
/// rather than a list, so the count is not derived from a collection.
const ACCOUNT_OVERVIEW_ROWS: usize = 10;

pub const SERVICES: &[ServiceEntry] = &[
    ServiceEntry {
        view: ActiveView::Findings,
        row_text: row_text!(findings, |f| format!(
            "{} {} {} {} {}",
            f.service,
            f.severity.as_str(),
            f.category.as_str(),
            f.resource_id.clone().unwrap_or_default(),
            f.summary
        )),
        rows: ViewRows::Summary,
    },
    ServiceEntry {
        view: ActiveView::AccountOverview,
        row_text: |_| vec![String::new(); ACCOUNT_OVERVIEW_ROWS],
        rows: ViewRows::Summary,
    },
    ServiceEntry {
        view: ActiveView::CostOverview,
        row_text: |app| {
            app.sorted_cost_insights()
                .iter()
                .map(|insight| insight.service.clone())
                .collect()
        },
        rows: ViewRows::Summary,
    },
    ServiceEntry {
        view: ActiveView::CostSavings,
        row_text: row_text!(cost_savings_opportunities, |o| format!(
            "{} {}",
            o.title, o.service
        )),
        rows: ViewRows::Summary,
    },
    ServiceEntry {
        view: ActiveView::Ecs,
        row_text: row_text!(ecs_clusters, |c| c.name.clone()),
        rows: resources!(ecs_clusters),
    },
    ServiceEntry {
        view: ActiveView::Ec2,
        row_text: row_text!(ec2_instances, |i| format!(
            "{} {} {} {} {}",
            i.label(),
            i.id,
            i.instance_type,
            i.state,
            i.region
        )),
        rows: resources!(ec2_instances),
    },
    ServiceEntry {
        view: ActiveView::Rds,
        row_text: row_text!(rds_instances, |d| format!(
            "{} {} {} {}",
            d.identifier, d.engine, d.status, d.region
        )),
        rows: resources!(rds_instances),
    },
    ServiceEntry {
        view: ActiveView::Lambda,
        row_text: row_text!(lambda_functions, |f| format!(
            "{} {} {}",
            f.name, f.runtime, f.region
        )),
        rows: resources!(lambda_functions),
    },
    ServiceEntry {
        view: ActiveView::Apigateway,
        row_text: row_text!(apigateway_apis, |a| format!(
            "{} {} {}",
            a.name, a.id, a.api_type
        )),
        rows: resources!(apigateway_apis),
    },
    ServiceEntry {
        view: ActiveView::Sqs,
        row_text: row_text!(sqs_queues_data, |q| q.name.clone()),
        rows: resources!(sqs_queues_data),
    },
    ServiceEntry {
        view: ActiveView::Vpc,
        row_text: row_text!(vpcs, |v| format!("{} {} {}", v.vpc_id, v.cidr, v.state)),
        rows: resources!(vpcs),
    },
    ServiceEntry {
        view: ActiveView::Secrets,
        row_text: row_text!(secrets, |s| s.name.clone()),
        rows: resources!(secrets),
    },
    ServiceEntry {
        view: ActiveView::CloudWatch,
        row_text: row_text!(cloudwatch_alarms, |a| format!(
            "{} {} {} {}",
            a.name, a.state, a.namespace, a.metric
        )),
        rows: resources!(cloudwatch_alarms),
    },
    ServiceEntry {
        view: ActiveView::LoadBalancers,
        row_text: row_text!(load_balancers, |l| format!(
            "{} {} {} {}",
            l.name, l.lb_type, l.scheme, l.state
        )),
        rows: resources!(load_balancers),
    },
    ServiceEntry {
        view: ActiveView::TargetGroups,
        row_text: row_text!(target_groups, |t| format!(
            "{} {} {}",
            t.name, t.protocol, t.target_type
        )),
        rows: resources!(target_groups),
    },
    ServiceEntry {
        view: ActiveView::SecurityGroups,
        row_text: row_text!(security_groups, |g| format!(
            "{} {} {}",
            g.name, g.id, g.vpc_id
        )),
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
            dead_letter_target_arn: None,
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

    fn app_with_queues(names: &[&str]) -> App {
        let mut app = test_app();
        app.active_view = ActiveView::Sqs;
        app.sqs_queues_data = names
            .iter()
            .map(|name| SqsQueueInfo {
                name: (*name).into(),
                queue_url: format!("https://sqs.us-east-1.amazonaws.com/1/{name}"),
                is_fifo: false,
                messages_available: 0,
                messages_in_flight: 0,
                has_dlq: false,
                dead_letter_target_arn: None,
                tags: Tags::empty(),
            })
            .collect();
        app
    }

    /// The bug a view-level filter would introduce: selection indexes the rows
    /// on screen, so resolving it against the unfiltered data would describe a
    /// different resource than the highlighted one.
    #[test]
    fn a_selection_resolves_to_the_row_the_filter_left_visible() {
        let mut app = app_with_queues(&["alpha", "beta", "orders", "gamma"]);
        app.row_filter = "orders".into();
        app.selected_row = 0;

        assert_eq!(app.visible_indices(), vec![2]);
        let selected = app.selected_describable().expect("a row is selected");
        assert_eq!(selected.resource_name(), "orders");
    }

    #[test]
    fn an_unfiltered_view_sees_every_row_in_order() {
        let app = app_with_queues(&["alpha", "beta", "orders"]);

        assert_eq!(app.visible_indices(), vec![0, 1, 2]);
    }

    #[test]
    fn filtering_is_case_insensitive_and_matches_a_substring() {
        let mut app = app_with_queues(&["Orders-Prod", "billing"]);
        app.row_filter = "ORD".into();

        assert_eq!(app.visible_indices(), vec![0]);
    }

    #[test]
    fn a_filter_matching_nothing_selects_nothing() {
        let mut app = app_with_queues(&["alpha", "beta"]);
        app.row_filter = "nonexistent".into();

        assert!(app.visible_indices().is_empty());
        assert!(app.selected_describable().is_none());
    }

    /// Whitespace alone is not a filter, so it must not hide every row.
    #[test]
    fn a_blank_query_is_not_a_filter() {
        let mut app = app_with_queues(&["alpha", "beta"]);
        app.row_filter = "   ".into();

        assert!(!app.filter_is_active());
        assert_eq!(app.visible_indices(), vec![0, 1]);
    }

    #[test]
    fn typing_a_filter_returns_the_cursor_to_the_top() {
        let mut app = app_with_queues(&["alpha", "beta", "orders"]);
        app.selected_row = 2;
        app.scroll_offset = 2;

        app.push_filter_char('o');

        assert_eq!(app.selected_row, 0);
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn clearing_a_filter_restores_every_row() {
        let mut app = app_with_queues(&["alpha", "beta", "orders"]);
        app.row_filter = "orders".into();
        app.filter_mode = true;
        assert_eq!(app.visible_indices().len(), 1);

        app.clear_filter();

        assert!(!app.filter_mode);
        assert!(!app.filter_is_active());
        assert_eq!(app.visible_indices(), vec![0, 1, 2]);
    }

    /// Keeping a filter after leaving the prompt is the point: the operator
    /// narrows the list, then works it with the normal keys.
    #[test]
    fn committing_a_filter_keeps_the_rows_it_selected() {
        let mut app = app_with_queues(&["alpha", "orders"]);
        app.row_filter = "orders".into();
        app.filter_mode = true;

        app.commit_filter();

        assert!(!app.filter_mode);
        assert!(app.filter_is_active());
        assert_eq!(app.visible_indices(), vec![1]);
    }

    #[test]
    fn the_reported_total_is_the_unfiltered_row_count() {
        let mut app = app_with_queues(&["alpha", "beta", "orders"]);
        app.row_filter = "orders".into();

        assert_eq!(app.visible_indices().len(), 1);
        assert_eq!(app.total_row_count(), 3);
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
