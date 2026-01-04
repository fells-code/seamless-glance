use crate::{app::App, models::service_status::ServiceStatus};
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

pub fn render_cw(frame: &mut Frame, area: Rect, app: &App) {
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

    let rows: Vec<Row> = app
        .cloudwatch_alarms
        .iter()
        .map(|a| {
            let state_style = if a.state == "ALARM" {
                Style::default().fg(app.theme.primary)
            } else {
                Style::default().fg(app.theme.text)
            };

            Row::new(vec![
                Cell::from(a.name.clone()),
                Cell::from(a.state.clone()).style(state_style),
                Cell::from(a.namespace.clone()),
                Cell::from(a.metric.clone()),
            ])
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
