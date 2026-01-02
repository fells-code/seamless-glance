use ratatui::{
    layout::Constraint,
    style::Style,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

use crate::app::App;

pub fn render_sqs(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let rows: Vec<Row> = app
        .sqs_queues_data
        .iter()
        .map(|q| {
            Row::new(vec![
                Cell::from(q.name.clone()),
                Cell::from(if q.is_fifo { "FIFO" } else { "Standard" }),
                Cell::from(q.messages_available.to_string()),
                Cell::from(q.messages_in_flight.to_string()),
                Cell::from(if q.has_dlq { "Yes" } else { "No" }),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        &[
            Constraint::Percentage(35),
            Constraint::Percentage(10),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(10),
        ],
    )
    .header(
        Row::new(vec!["Queue", "Type", "Available", "In Flight", "DLQ"])
            .style(Style::default().fg(app.theme.accent)),
    )
    .block(
        Block::default()
            .title("SQS Queues")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.primary)),
    );

    frame.render_widget(table, area);
}
