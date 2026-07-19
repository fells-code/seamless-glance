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

fn relative_age(seconds: i64) -> String {
    if seconds < 5 {
        "just now".into()
    } else if seconds < 60 {
        format!("{seconds}s ago")
    } else if seconds < 3600 {
        format!("{}m ago", seconds / 60)
    } else {
        format!("{}h ago", seconds / 3600)
    }
}

fn refresh_text(app: &App) -> String {
    let refresh = app
        .last_refresh
        .map(|t| {
            let age = chrono::Utc::now()
                .signed_duration_since(t)
                .num_seconds()
                .max(0);
            format!("{} ({})", t.format("%H:%M:%S UTC"), relative_age(age))
        })
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
    let profile_label = app.current_profile.as_deref().unwrap_or("default");

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
        "View: {}  |  Region: {} ({})  |  Profile: {}\n\
        Theme: {}  |  {}\n\
        Account: {}\n\
        {}\n\
        Account Health: {}\n\
        {}",
        view_name,
        region_label,
        region_mode,
        profile_label,
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

#[cfg(test)]
mod tests {
    use super::relative_age;

    #[test]
    fn relative_age_reads_naturally() {
        assert_eq!(relative_age(0), "just now");
        assert_eq!(relative_age(4), "just now");
        assert_eq!(relative_age(30), "30s ago");
        assert_eq!(relative_age(90), "1m ago");
        assert_eq!(relative_age(3600), "1h ago");
    }
}
