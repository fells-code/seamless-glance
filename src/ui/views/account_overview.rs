use crate::{app::App, models::service_status::ServiceStatus};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let Some(overview) = &app.account_overview else {
        let loading_text = format!("Fetching AWS inventory for {}…", app.current_region_label());

        let loading = Paragraph::new(loading_text)
            .style(Style::default().fg(app.theme.accent))
            .block(
                Block::default()
                    .title("Account Overview")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.primary)),
            );

        frame.render_widget(loading, area);
        return;
    };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(0)])
        .split(area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(layout[0]);

    let profile_text = format!(
        "Account {}\nIdentity: {} ({})\nScope: {}  |  Enabled regions: {} + global",
        overview.account_id,
        overview.identity_kind,
        overview.identity_name,
        app.current_region_label(),
        app.regions.len()
    );

    let compute_total = overview.ec2_running
        + overview.ec2_stopped
        + overview.lambda_functions
        + overview.ecs_services;
    let network_total = overview.vpc_count
        + overview.subnet_count
        + overview.target_groups_total as u32
        + overview.apigw_rest_apis
        + overview.apigw_http_apis;
    let data_total = overview.rds_status.total as u32 + overview.secrets.total as u32;
    let ops_total = overview.sqs_queues + overview.alarms.total_alarms as u32;

    let rollup_text = format!(
        "Compute: {} tracked resources  |  Data: {}\n\
         Network: {} tracked resources  |  Messaging + Ops: {}\n\
         This screen is a neutral inventory snapshot. Use Findings for prioritized issues.",
        compute_total, data_total, network_total, ops_total
    );

    render_panel(frame, top[0], app, "AWS Profile", &profile_text);
    render_panel(frame, top[1], app, "Inventory Snapshot", &rollup_text);

    let rows = vec![
        inventory_row(
            "CloudWatch",
            format!("{} alarms", overview.alarms.total_alarms),
            format!("{} in ALARM", overview.alarms.alarms_in_alarm),
            access_label(&overview.alarms.status),
        ),
        inventory_row(
            "EC2",
            format!("{} instances", overview.ec2_running + overview.ec2_stopped),
            format!(
                "{} running / {} stopped",
                overview.ec2_running, overview.ec2_stopped
            ),
            "Accessible".into(),
        ),
        inventory_row(
            "ECS",
            format!("{} clusters", overview.ecs_clusters),
            format!("{} services", overview.ecs_services),
            "Accessible".into(),
        ),
        inventory_row(
            "Lambda",
            format!("{} functions", overview.lambda_functions),
            "Function inventory".into(),
            access_label(&overview.lambda_status),
        ),
        inventory_row(
            "API Gateway",
            format!("{} REST APIs", overview.apigw_rest_apis),
            format!("{} HTTP APIs", overview.apigw_http_apis),
            access_label(&overview.apigw_status),
        ),
        inventory_row(
            "SQS",
            format!("{} queues", overview.sqs_queues),
            format!("{} queues with DLQs", overview.sqs_dlqs),
            access_label(&overview.sqs_status),
        ),
        inventory_row(
            "VPC",
            format!("{} VPCs", overview.vpc_count),
            format!("{} subnets", overview.subnet_count),
            access_label(&overview.vpc_status),
        ),
        inventory_row(
            "Target Groups",
            format!("{} groups", overview.target_groups_total),
            format!("{} unhealthy tracked", overview.target_groups_unhealthy),
            "Accessible".into(),
        ),
        inventory_row(
            "Secrets",
            format!("{} secrets", overview.secrets.total),
            format!(
                "{} with rotation disabled",
                overview.secrets.rotation_disabled
            ),
            access_label(&overview.secrets.status),
        ),
        inventory_row(
            "RDS",
            format!("{} instances", overview.rds_status.total),
            format!("{} available", overview.rds_status.available),
            access_label(&overview.rds_status.status),
        ),
    ];

    let table = Table::new(
        rows,
        [
            Constraint::Length(16),
            Constraint::Length(22),
            Constraint::Percentage(50),
            Constraint::Length(14),
        ],
    )
    .header(
        Row::new(["SERVICE", "INVENTORY", "DETAIL", "ACCESS"]).style(
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .title("Service Inventory")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.primary)),
    );

    frame.render_widget(table, layout[1]);
}

fn render_panel(frame: &mut Frame, area: Rect, app: &App, title: &str, text: &str) {
    let panel = Paragraph::new(text)
        .style(Style::default().fg(app.theme.text))
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.primary)),
        );

    frame.render_widget(panel, area);
}

fn inventory_row(service: &str, inventory: String, detail: String, access: String) -> Row<'static> {
    Row::new(vec![
        Cell::from(service.to_string()),
        Cell::from(inventory),
        Cell::from(detail),
        Cell::from(access),
    ])
}

fn access_label(status: &ServiceStatus) -> String {
    match status {
        ServiceStatus::Ok => "Accessible".into(),
        ServiceStatus::AccessDenied => "Access denied".into(),
        ServiceStatus::Unavailable(_) => "Unavailable".into(),
    }
}
