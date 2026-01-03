use crate::{app::App, models::service_status::ServiceStatus};
use ratatui::{
    layout::Rect,
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

    let (running, stopped) =
        app.ec2_instances
            .iter()
            .fold((0, 0), |acc, i| match i.state.as_str() {
                "running" => (acc.0 + 1, acc.1),
                "stopped" => (acc.0, acc.1 + 1),
                _ => acc,
            });

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

    let cloudwatch_line = match &overview.alarms.status {
        ServiceStatus::Ok => {
            if overview.alarms.alarms_in_alarm > 0 {
                format!(
                    "CloudWatch: {} alarms ({} in ALARM)",
                    overview.alarms.total_alarms, overview.alarms.alarms_in_alarm
                )
            } else {
                format!(
                    "CloudWatch: {} alarms (all OK)",
                    overview.alarms.total_alarms
                )
            }
        }
        ServiceStatus::AccessDenied => "CloudWatch: ⚠️ Access denied".into(),
        ServiceStatus::Unavailable(_) => "CloudWatch: ⚠️ Unavailable".into(),
    };

    // ---- STATS ----
    let stats = Paragraph::new(format!(
        "{}\n\
         {}\n\
         EC2: {} running / {} stopped\n\
         ECS: {} clusters / {} services\n\
         {}\n\
         {}\n\
         {}\n\
         {}\n\
         {}",
        vpc_line,
        cloudwatch_line,
        running,
        stopped,
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

    frame.render_widget(stats, area);
}
