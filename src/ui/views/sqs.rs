use ratatui::{
    layout::Constraint,
    style::Style,
    widgets::{Cell, Row},
    Frame,
};

use crate::app::App;
use crate::ui::views::list_table::{render_list_table, visible_rows, ListSelection, ListTable};

pub fn render_sqs(frame: &mut Frame, area: ratatui::layout::Rect, app: &mut App) {
    if crate::ui::views::status::render_unavailable(frame, area, "SQS", &app.sqs_status, &app.theme)
    {
        return;
    }

    let theme = app.theme;

    let visible = app.visible_indices();
    let rows = visible_rows(&visible, &app.sqs_queues_data);

    render_list_table(
        frame,
        area,
        ListSelection {
            selected_row: &mut app.selected_row,
            scroll_offset: &mut app.scroll_offset,
        },
        &theme,
        ListTable {
            title: "SQS Queues",
            headers: &["Queue", "Type", "Available", "In Flight", "DLQ", "Signals"],
            widths: &[
                Constraint::Percentage(35),
                Constraint::Percentage(10),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
                Constraint::Percentage(10),
                Constraint::Percentage(15),
            ],
            empty_message: "No SQS queues found in this region.",
        },
        &rows,
        |q| {
            let style = if q.has_backlog_incident() {
                Style::default().fg(theme.accent)
            } else if !q.has_dlq {
                Style::default().fg(theme.primary)
            } else {
                Style::default().fg(theme.text)
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
        },
    );
}
