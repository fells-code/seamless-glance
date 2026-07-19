use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Cell, Row},
    Frame,
};

use crate::app::App;
use crate::ui::views::list_table::{render_list_table, ListSelection, ListTable};

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    if crate::ui::views::status::render_unavailable(
        frame,
        area,
        "Lambda",
        &app.lambda_status,
        &app.theme,
    ) {
        return;
    }

    let theme = app.theme;

    render_list_table(
        frame,
        area,
        ListSelection {
            selected_row: &mut app.selected_row,
            scroll_offset: &mut app.scroll_offset,
        },
        &theme,
        ListTable {
            title: "Lambda Functions",
            headers: &[
                "Name",
                "Region",
                "Runtime",
                "Memory (MB)",
                "Timeout (s)",
                "Last Modified",
            ],
            widths: &[
                Constraint::Percentage(24),
                Constraint::Percentage(14),
                Constraint::Percentage(14),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(28),
            ],
            empty_message: "No Lambda functions found in this region.",
        },
        &app.lambda_functions,
        |f| {
            let style = if f.has_high_memory() || f.is_stale() {
                Style::default().fg(theme.primary)
            } else {
                Style::default().fg(theme.text)
            };

            Row::new(vec![
                Cell::from(f.name.clone()),
                Cell::from(f.region.clone()),
                Cell::from(f.runtime.clone()),
                Cell::from(f.memory_mb.to_string()),
                Cell::from(f.timeout_sec.to_string()),
                Cell::from(f.last_modified.clone()),
            ])
            .style(style)
        },
    );
}
