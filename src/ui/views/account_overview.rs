use crate::{app::App, models::service_status::ServiceStatus};
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

const LABEL_WIDTH: usize = 14;

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

    let mut issues = Vec::new();

    if overview.alarms.alarms_in_alarm > 0 {
        issues.push(format!(
            "CloudWatch: {} alarms in ALARM",
            overview.alarms.alarms_in_alarm
        ));
    }

    if overview.secrets.rotation_disabled > 0 {
        issues.push(format!(
            "Secrets: {} without rotation",
            overview.secrets.rotation_disabled
        ));
    }

    if overview.ec2_stopped > 0 {
        issues.push(format!("EC2: {} stopped instances", overview.ec2_stopped));
    }

    let health_label = if issues.is_empty() {
        "Healthy"
    } else {
        "Attention Needed"
    };

    let tg_value = if overview.target_groups_unhealthy > 0 {
        format!(
            "{} target groups ({} unhealthy)",
            overview.target_groups_total, overview.target_groups_unhealthy
        )
    } else {
        format!(
            "{} target groups (all healthy)",
            overview.target_groups_total
        )
    };

    if overview.target_groups_unhealthy > 0 {
        issues.push(format!(
            "{} target groups ({} unhealthy)",
            overview.target_groups_total, overview.target_groups_unhealthy
        ));
    }

    let health_text = if issues.is_empty() {
        "No issues detected.\nYour account appears healthy.".to_string()
    } else {
        format!(
            "Issues detected:\n{}",
            issues
                .iter()
                .map(|i| format!("• {}", i))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    let health = Paragraph::new(format!(
        "Account Health: {}\n\n{}",
        health_label, health_text
    ))
    .style(Style::default().fg(app.theme.text))
    .block(
        Block::default()
            .title("Status")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.primary)),
    );

    let running = overview.ec2_running;
    let stopped = overview.ec2_stopped;

    let vpc_value = match &overview.vpc_status {
        ServiceStatus::Ok => format!(
            "{} VPCs / {} subnets",
            overview.vpc_count, overview.subnet_count
        ),
        ServiceStatus::AccessDenied => "⚠️ Access denied".into(),
        ServiceStatus::Unavailable(_) => "⚠️ Unavailable".into(),
    };

    let cloudwatch_value = match &overview.alarms.status {
        ServiceStatus::Ok => {
            if overview.alarms.alarms_in_alarm > 0 {
                format!(
                    "{} alarms ({} in ALARM)",
                    overview.alarms.total_alarms, overview.alarms.alarms_in_alarm
                )
            } else {
                format!("{} alarms (all OK)", overview.alarms.total_alarms)
            }
        }
        ServiceStatus::AccessDenied => "⚠️ Access denied".into(),
        ServiceStatus::Unavailable(_) => "⚠️ Unavailable".into(),
    };

    let ec2_value = format!("{} running / {} stopped", running, stopped);

    let ecs_value = format!(
        "{} clusters / {} services",
        overview.ecs_clusters, overview.ecs_services
    );

    let secrets_value = match &overview.secrets.status {
        ServiceStatus::Ok => format!(
            "{} total ({} without rotation)",
            overview.secrets.total, overview.secrets.rotation_disabled
        ),
        ServiceStatus::AccessDenied => "⚠️ Access denied".into(),
        ServiceStatus::Unavailable(_) => "⚠️ Unavailable".into(),
    };

    let lambda_value = match &overview.lambda_status {
        ServiceStatus::Ok => format!("{} functions", overview.lambda_functions),
        ServiceStatus::AccessDenied => "⚠️ Access denied".into(),
        ServiceStatus::Unavailable(_) => "⚠️ Unavailable".into(),
    };

    let apigw_value = match &overview.apigw_status {
        ServiceStatus::Ok => format!(
            "{} REST / {} HTTP",
            overview.apigw_rest_apis, overview.apigw_http_apis
        ),
        ServiceStatus::AccessDenied => "⚠️ Access denied".into(),
        ServiceStatus::Unavailable(_) => "⚠️ Unavailable".into(),
    };

    let sqs_value = match &overview.sqs_status {
        ServiceStatus::Ok => format!(
            "{} queues ({} DLQs)",
            overview.sqs_queues, overview.sqs_dlqs
        ),
        ServiceStatus::AccessDenied => "⚠️ Access denied".into(),
        ServiceStatus::Unavailable(_) => "⚠️ Unavailable".into(),
    };

    let rds_value = match &overview.rds_status.status {
        ServiceStatus::Ok => format!(
            "{} instances ({} available)",
            overview.rds_status.total, overview.rds_status.available
        ),
        ServiceStatus::AccessDenied => "⚠️ Access denied".into(),
        ServiceStatus::Unavailable(_) => "⚠️ Unavailable".into(),
    };

    // let elb_value = match &overview.elb_status {
    //     ServiceStatus::Ok => format!("{}", overview.load_balancers),
    //     ServiceStatus::AccessDenied => "⚠️ Access denied".into(),
    //     ServiceStatus::Unavailable(_) => "⚠️ Unavailable".into(),
    // };

    // ---- STATS ----
    let stats_text = format!(
        "{:<LABEL_WIDTH$} {}\n\
     {:<LABEL_WIDTH$} {}\n\
     {:<LABEL_WIDTH$} {}\n\
     {:<LABEL_WIDTH$} {}\n\
     {:<LABEL_WIDTH$} {}\n\
     {:<LABEL_WIDTH$} {}\n\
     {:<LABEL_WIDTH$} {}\n\
     {:<LABEL_WIDTH$} {}\n\
     {:<LABEL_WIDTH$} {}\n\
     {:<LABEL_WIDTH$} {}",
        "VPC",
        vpc_value,
        "CloudWatch",
        cloudwatch_value,
        "EC2",
        ec2_value,
        "ECS",
        ecs_value,
        "Target Groups",
        tg_value,
        "Secrets",
        secrets_value,
        "Lambda",
        lambda_value,
        "API Gateway",
        apigw_value,
        "SQS",
        sqs_value,
        "RDS",
        rds_value,
        // "Load Balancers",
        // elb_value,
        LABEL_WIDTH = LABEL_WIDTH
    );

    let stats = Paragraph::new(stats_text)
        .style(Style::default().fg(app.theme.text))
        .block(
            Block::default()
                .title("Overview")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.primary)),
        );

    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(6), // health summary
            Constraint::Min(0),    // inventory
        ])
        .split(area);

    frame.render_widget(health, chunks[0]);
    frame.render_widget(stats, chunks[1]);
}
