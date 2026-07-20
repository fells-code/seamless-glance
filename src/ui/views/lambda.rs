use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    Frame,
};

use crate::app::App;
use crate::ui::views::list_table::{
    filter_query, render_list_table, visible_rows, ListSelection, ListTable, RowCells,
};

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

    let wrapped = app.wrap_mode_active();
    let filter = filter_query(&app.row_filter);
    let visible = app.visible_indices();
    let rows = visible_rows(&visible, &app.lambda_functions);

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
            filter,
            wrapped,
        },
        &rows,
        |f| {
            let style = if f.has_high_memory() || f.is_stale() {
                Style::default().fg(theme.primary)
            } else {
                Style::default().fg(theme.text)
            };

            RowCells {
                cells: vec![
                    f.name.clone(),
                    f.region.clone(),
                    f.runtime.clone(),
                    f.memory_mb.to_string(),
                    f.timeout_sec.to_string(),
                    f.last_modified.clone(),
                ],
                style,
            }
        },
    );
}
