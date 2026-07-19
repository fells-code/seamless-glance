use ratatui::{
    layout::Constraint,
    style::Style,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

use crate::app::App;

pub fn render_sqs(frame: &mut Frame, area: ratatui::layout::Rect, app: &mut App) {
    if crate::ui::views::status::render_unavailable(frame, area, "SQS", &app.sqs_status, &app.theme)
    {
        return;
    }

    let total_rows = app.sqs_queues_data.len();
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
        .sqs_queues_data
        .iter()
        .enumerate()
        .skip(app.scroll_offset as usize)
        .take(visible_height)
        .map(|(i, q)| {
            let style = if i == app.selected_row {
                Style::default().fg(app.theme.highlight)
            } else if q.has_backlog_incident() {
                Style::default().fg(app.theme.accent)
            } else if !q.has_dlq {
                Style::default().fg(app.theme.primary)
            } else {
                Style::default().fg(app.theme.text)
            };

            Row::new(vec![
                Cell::from(q.name.clone()),
                Cell::from(if q.is_fifo { "FIFO" } else { "Standard" }),
                Cell::from(q.messages_available.to_string()),
                Cell::from(q.messages_in_flight.to_string()),
                Cell::from(if q.has_dlq { "Yes" } else { "No" }),
                Cell::from({
                    let signals = q.backlog_signals();
                    if signals.is_empty() {
                        "-".into()
                    } else {
                        signals.join(",")
                    }
                }),
            ])
            .style(style)
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
            Constraint::Percentage(15),
        ],
    )
    .header(
        Row::new(vec![
            "Queue",
            "Type",
            "Available",
            "In Flight",
            "DLQ",
            "Signals",
        ])
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
