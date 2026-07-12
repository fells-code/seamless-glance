use crate::{
    app::{App, RefreshPhase},
    config::VERSION,
    ui::views::command::command_for_view,
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
    let view_name = command_for_view(app.active_view)
        .map(|command| command.description)
        .unwrap_or("Unknown view");
    let region_label = app.current_region_label();
    let region_mode = if app.is_global_region_selected() {
        "Global slot"
    } else {
        "Regional"
    };

    let account = app
        .account_overview
        .as_ref()
        .map(|overview| overview.account_id.as_str())
        .unwrap_or("—");

    let identity_line = if let Some(o) = &app.account_overview {
        match &o.role_name {
            Some(role_name) => format!(
                "Identity: {} ({})  |  Role: {}",
                o.identity_kind, o.identity_name, role_name
            ),
            None => format!("Identity: {} ({})", o.identity_kind, o.identity_name),
        }
    } else {
        "Identity: —".into()
    };

    let header_text = format!(
        "View: {}  |  Region: {} ({})\n\
        Theme: {}  |  {}\n\
        Account: {}\n\
        {}\n\
        Account Health: {}\n\
        {}",
        view_name,
        region_label,
        region_mode,
        app.theme_name.label(),
        app.theme_name.description(),
        account,
        identity_line,
        health.0,
        status
    );

    let header = Paragraph::new(header_text)
        .style(Style::default().fg(app.theme.text))
        .block(
            Block::default()
                .title(format!("Seamless Glance v{}", VERSION))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.primary)),
        );

    frame.render_widget(header, area);
}
