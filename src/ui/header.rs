use crate::{
    app::{App, RefreshPhase},
    config::VERSION,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

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
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70), // left
            Constraint::Percentage(30), // right
        ])
        .split(area);

    let status = refresh_text(app);

    let (account, region, role) = if let Some(overview) = &app.account_overview {
        (
            overview.account_id.as_str(),
            overview.region.as_str(),
            overview.role_name.as_deref().unwrap_or("unknown-role"),
        )
    } else {
        ("—", app.current_region().as_ref(), "—")
    };

    // LEFT: context
    let left_text = format!(
        "Account {}\nRegion {}\nRole {}\n{}",
        account, region, role, status
    );

    let left = Paragraph::new(left_text)
        .style(Style::default().fg(app.theme.text))
        .block(
            Block::default()
                .title(format!("Seamless Glance v{}", VERSION))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.primary)),
        );

    frame.render_widget(left, header_chunks[0]);

    // RIGHT: MTD cost
    let cost_text = app
        .account_overview
        .as_ref()
        .map(|o| format!("${:.2}", o.month_to_date_cost))
        .unwrap_or_else(|| "—".into());

    let right = Paragraph::new(cost_text)
        .alignment(Alignment::Right)
        .style(Style::default().fg(app.theme.accent))
        .block(
            Block::default()
                .title("MTD Cost (updated daily)")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.primary)),
        );

    frame.render_widget(right, header_chunks[1]);
}
