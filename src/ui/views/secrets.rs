use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    Frame,
};

use crate::app::App;
use crate::ui::views::list_table::{
    filter_query, render_list_table, visible_rows, ListSelection, ListTable, RowCells,
};

pub fn render_sm(frame: &mut Frame, area: Rect, app: &mut App) {
    if crate::ui::views::status::render_unavailable(
        frame,
        area,
        "Secrets Manager",
        &app.secrets_summary.status,
        &app.theme,
    ) {
        return;
    }

    let theme = app.theme;

    let wrapped = app.wrap_mode_active();
    let filter = filter_query(&app.row_filter);
    let visible = app.visible_indices();
    let rows = visible_rows(&visible, &app.secrets);

    render_list_table(
        frame,
        area,
        ListSelection {
            selected_row: &mut app.selected_row,
            scroll_offset: &mut app.scroll_offset,
        },
        &theme,
        ListTable {
            title: "Secrets Manager",
            headers: &["Name", "Rotation", "Last Rotated", "Signals"],
            widths: &[
                Constraint::Percentage(40),
                Constraint::Percentage(20),
                Constraint::Percentage(30),
                Constraint::Percentage(10),
            ],
            empty_message: "No secrets found in this region.",
            filter,
            wrapped,
        },
        &rows,
        |s| {
            let style = if s.needs_rotation_review() {
                Style::default().fg(theme.primary)
            } else if s.has_stale_rotation() || s.rotation_disabled() {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.text)
            };

            RowCells {
                cells: vec![
                    s.name.clone(),
                    if s.rotation_enabled { "Yes" } else { "No" }.to_string(),
                    s.last_rotated.clone().unwrap_or("-".into()),
                    {
                        let signals = s.review_signals();
                        if signals.is_empty() {
                            "-".into()
                        } else {
                            signals.join(",")
                        }
                    },
                ],
                style,
            }
        },
    );
}
