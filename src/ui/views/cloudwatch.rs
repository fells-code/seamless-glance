use crate::{app::App, models::service_status::ServiceStatus};
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

pub fn render_cw(frame: &mut Frame, area: Rect, app: &mut App) {
    if !matches!(app.cloudwatch_summary.status, ServiceStatus::Ok) {
        let msg = match app.cloudwatch_summary.status {
            ServiceStatus::AccessDenied => "Access denied to CloudWatch",
            ServiceStatus::Unavailable(_) => "CloudWatch unavailable",
            _ => "",
        };

        let p = Paragraph::new(msg)
            .style(Style::default().fg(app.theme.accent))
            .block(Block::default().title("CloudWatch").borders(Borders::ALL));

        frame.render_widget(p, area);
        return;
    }

    let total_rows = app.cloudwatch_alarms.len();
    if total_rows == 0 {
        app.selected_row = 0;
        app.scroll_offset = 0;
    }

    if total_rows > 0 {
        app.selected_row = app.selected_row.min(total_rows - 1);
    }

    let visible_height = area.height.saturating_sub(3) as usize; // header + borders

    // Keep selected row in view
    if app.selected_row < app.scroll_offset as usize {
        app.scroll_offset = app.selected_row as u16;
    } else if app.selected_row >= app.scroll_offset as usize + visible_height {
        app.scroll_offset = (app.selected_row + 1 - visible_height) as u16;
    }

    let rows: Vec<Row> = app
        .cloudwatch_alarms
        .iter()
        .enumerate()
        .skip(app.scroll_offset as usize)
        .take(visible_height)
        .map(|(i, a)| {
            let style = if i == app.selected_row {
                Style::default().fg(app.theme.highlight)
            } else if a.state == "ALARM" {
                Style::default().fg(app.theme.primary)
            } else {
                Style::default().fg(app.theme.text)
            };

            Row::new(vec![
                Cell::from(a.name.clone()),
                Cell::from(a.state.clone()),
                Cell::from(a.namespace.clone()),
                Cell::from(a.metric.clone()),
            ])
            .style(style)
        })
        .collect();

    let header = Row::new(vec!["Alarm", "State", "Namespace", "Metric"])
        .style(Style::default().fg(app.theme.accent));

    let widths = [
        Constraint::Percentage(40),
        Constraint::Percentage(15),
        Constraint::Percentage(25),
        Constraint::Percentage(20),
    ];

    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .title("CloudWatch Alarms")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.primary)),
    );

    frame.render_widget(table, area);
}
