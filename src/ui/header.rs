use crate::{
    app::{App, RefreshPhase},
    config::VERSION,
};
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

fn account_health_label(app: &App) -> (&'static str, ratatui::style::Style) {
    if let Some(overview) = &app.account_overview {
        let mut issues = 0;

        if overview.alarms.alarms_in_alarm > 0 {
            issues += 1;
        }
        if overview.secrets.rotation_disabled > 0 {
            issues += 1;
        }
        if overview.ec2_stopped > 0 {
            issues += 1;
        }

        if issues == 0 {
            ("Healthy", Style::default().fg(app.theme.accent))
        } else {
            ("Attention Needed", Style::default().fg(app.theme.primary))
        }
    } else {
        ("Loading…", Style::default().fg(app.theme.text))
    }
}

fn refresh_text(app: &App) -> String {
    let refresh = app
        .last_refresh
        .map(|t| t.format("%H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "—".into());

    match &app.refresh_phase {
        RefreshPhase::Idle => format!("Last updated {}", refresh),
        RefreshPhase::Overview => "Refreshing overview…".into(),
        RefreshPhase::Services(services) => {
            if services.is_empty() {
                "Refreshing services…".into()
            } else {
                format!("Refreshing {}…", services.join(", "))
            }
        }
    }
}

pub fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let health = account_health_label(app);
    let status = refresh_text(app);

    let account_label = match &app.license {
        Some(license) if license.is_paid() => "Pro Account".to_string(),
        Some(license) => license
            .trial_days_remaining()
            .map(|d| format!("Free Trial · {} days left", d))
            .unwrap_or_else(|| "Free Trial".to_string()),
        None => "Free Trial".to_string(),
    };

    let (account, region, _role) = if let Some(overview) = &app.account_overview {
        (
            overview.account_id.as_str(),
            overview.region.as_str(),
            overview.role_name.as_deref().unwrap_or("unknown-role"),
        )
    } else {
        ("—", app.current_region().as_ref(), "—")
    };

    let identity_line = if let Some(o) = &app.account_overview {
        format!("Identity: {} ({})", o.identity_kind, o.identity_name)
    } else {
        "Identity: —".into()
    };

    let header_text = format!(
        "Account {}  |  {}\n\
        {}\n\
         Account Health: {}\n\
         {}",
        account, region, identity_line, health.0, status
    );

    let header = Paragraph::new(header_text)
        .style(Style::default().fg(app.theme.text))
        .block(
            Block::default()
                .title(format!("Seamless Glance v{} · {}", VERSION, account_label))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.primary)),
        );

    frame.render_widget(header, area);
}
