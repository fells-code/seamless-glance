use crate::{app::App, models::service_status::ServiceStatus};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let Some(overview) = &app.account_overview else {
        let loading_text = format!("Fetching AWS data for {}…", app.current_region().as_ref());

        let loading = Paragraph::new(loading_text)
            .style(Style::default().fg(app.theme.accent))
            .block(
                Block::default()
                    .title("Seamless Glance")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.primary)),
            );

        frame.render_widget(loading, frame.size());
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(10)])
        .split(area);

    // ---- HEADER ----
    let role = overview.role_name.as_deref().unwrap_or("unknown-role");

    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70), // left
            Constraint::Percentage(30), // right
        ])
        .split(chunks[0]);

    // LEFT: identity + context
    let header_text = format!(
        "Account {}\nRegion {}   Role {}",
        overview.account_id, overview.region, role
    );

    let header_left = Paragraph::new(header_text)
        .style(Style::default().fg(app.theme.text))
        .block(
            Block::default()
                .title("Seamless Glance")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.primary)),
        );

    frame.render_widget(header_left, header_chunks[0]);

    // RIGHT: cost (right-aligned)
    let cost_text = format!("${:.2}", overview.month_to_date_cost);

    let header_right = Paragraph::new(cost_text)
        .alignment(ratatui::layout::Alignment::Right)
        .style(Style::default().fg(app.theme.accent))
        .block(
            Block::default()
                .title("MTD Cost")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.primary)),
        );

    frame.render_widget(header_right, header_chunks[1]);

    let rds_line = match &overview.rds_status {
        ServiceStatus::Ok => format!("RDS: {} instances", overview.rds_instances),
        ServiceStatus::AccessDenied => "RDS: ⚠️ Access denied".into(),
        ServiceStatus::Unavailable(_) => "RDS: ⚠️ Unavailable".into(),
    };

    let elb_line = match &overview.elb_status {
        ServiceStatus::Ok => format!("Load Balancers: {}", overview.load_balancers),
        ServiceStatus::AccessDenied => "Load Balancers: ⚠️ Access denied".into(),
        ServiceStatus::Unavailable(_) => "Load Balancers: ⚠️ Unavailable".into(),
    };

    let lambda_line = match &overview.lambda_status {
        ServiceStatus::Ok => format!("Lambda: {} functions", overview.lambda_functions),
        ServiceStatus::AccessDenied => "Lambda: ⚠️ Access denied".into(),
        ServiceStatus::Unavailable(_) => "Lambda: ⚠️ Unavailable".into(),
    };

    let apigw_line = match &overview.apigw_status {
        ServiceStatus::Ok => format!(
            "API Gateway: {} REST / {} HTTP",
            overview.apigw_rest_apis, overview.apigw_http_apis
        ),
        ServiceStatus::AccessDenied => "API Gateway: ⚠️ Access denied".into(),
        ServiceStatus::Unavailable(_) => "API Gateway: ⚠️ Unavailable".into(),
    };

    let sqs_line = match &overview.sqs_status {
        ServiceStatus::Ok => format!(
            "SQS: {} queues ({} DLQs)",
            overview.sqs_queues, overview.sqs_dlqs
        ),
        ServiceStatus::AccessDenied => "SQS: ⚠️ Access denied".into(),
        ServiceStatus::Unavailable(_) => "SQS: ⚠️ Unavailable".into(),
    };

    let vpc_line = match &overview.vpc_status {
        ServiceStatus::Ok => format!(
            "VPC: {} VPCs / {} subnets",
            overview.vpc_count, overview.subnet_count
        ),
        ServiceStatus::AccessDenied => "VPC: ⚠️ Access denied".into(),
        ServiceStatus::Unavailable(_) => "VPC: ⚠️ Unavailable".into(),
    };

    // ---- STATS ----
    let stats = Paragraph::new(format!(
        "{}\n\
         EC2: {} running / {} stopped\n\
         ECS: {} clusters / {} services\n\
         {}\n\
         {}\n\
         {}\n\
         {}\n\
         {}",
        vpc_line,
        overview.ec2_running,
        overview.ec2_stopped,
        overview.ecs_clusters,
        overview.ecs_services,
        lambda_line,
        apigw_line,
        sqs_line,
        rds_line,
        elb_line
    ))
    .style(Style::default().fg(app.theme.text))
    .block(
        Block::default()
            .title("Key Stats")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.accent)),
    );

    frame.render_widget(stats, chunks[1]);
}
