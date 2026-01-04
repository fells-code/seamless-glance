use crate::app::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

pub fn render_sm(frame: &mut Frame, area: Rect, app: &App) {
    if !matches!(
        app.secrets_summary.status,
        crate::models::service_status::ServiceStatus::Ok
    ) {
        let msg = "Secrets Manager access unavailable";
        let p = ratatui::widgets::Paragraph::new(msg)
            .style(Style::default().fg(app.theme.accent))
            .block(Block::default().title("Secrets").borders(Borders::ALL));

        frame.render_widget(p, area);
        return;
    }

    let rows: Vec<Row> = app
        .secrets
        .iter()
        .map(|s| {
            let style = if s.rotation_enabled {
                Style::default().fg(app.theme.text)
            } else {
                Style::default().fg(app.theme.primary)
            };

            Row::new(vec![
                Cell::from(s.name.clone()),
                Cell::from(if s.rotation_enabled { "Yes" } else { "No" }).style(style),
                Cell::from(s.last_rotated.clone().unwrap_or("—".into())),
            ])
        })
        .collect();

    let widths = [
        Constraint::Percentage(50),
        Constraint::Percentage(20),
        Constraint::Percentage(30),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["Name", "Rotation", "Last Rotated"])
                .style(Style::default().fg(app.theme.accent)),
        )
        .block(
            Block::default()
                .title("Secrets Manager")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.primary)),
        );

    frame.render_widget(table, area);
}
