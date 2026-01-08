use crate::app::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

pub fn render_sm(frame: &mut Frame, area: Rect, app: &mut App) {
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

    let total_rows = app.secrets.len();

    if total_rows == 0 {
        app.selected_row = 0;
        app.scroll_offset = 0;
    }

    // Clamp selection
    if total_rows > 0 {
        app.selected_row = app.selected_row.min(total_rows.saturating_sub(1));
    }

    // Compute visible rows
    let visible_height = area.height.saturating_sub(3) as usize; // header + borders

    // Keep cursor visible
    if app.selected_row < app.scroll_offset as usize {
        app.scroll_offset = app.selected_row as u16;
    } else if app.selected_row >= app.scroll_offset as usize + visible_height {
        app.scroll_offset = (app.selected_row + 1 - visible_height) as u16;
    }

    let rows: Vec<Row> = app
        .secrets
        .iter()
        .enumerate()
        .skip(app.scroll_offset as usize)
        .take(visible_height)
        .map(|(i, s)| {
            let is_selected = i == app.selected_row;

            let style = if is_selected {
                Style::default().fg(app.theme.highlight)
            } else {
                Style::default().fg(app.theme.text)
            };

            Row::new(vec![
                Cell::from(s.name.clone()),
                Cell::from(if s.rotation_enabled { "Yes" } else { "No" }),
                Cell::from(s.last_rotated.clone().unwrap_or("—".into())),
            ])
            .style(style)
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
