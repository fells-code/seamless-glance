use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    Frame,
};

use crate::app::App;
use crate::ui::views::list_table::{
    filter_query, render_list_table, visible_rows, ListSelection, ListTable, RowCells,
};

pub fn render_cw(frame: &mut Frame, area: Rect, app: &mut App) {
    if crate::ui::views::status::render_unavailable(
        frame,
        area,
        "CloudWatch Alarms",
        &app.cloudwatch_summary.status,
        &app.theme,
    ) {
        return;
    }

    let theme = app.theme;

    let wrapped = app.wrap_mode_active();
    let filter = filter_query(&app.row_filter);
    let visible = app.visible_indices();
    let rows = visible_rows(&visible, &app.cloudwatch_alarms);

    render_list_table(
        frame,
        area,
        ListSelection {
            selected_row: &mut app.selected_row,
            scroll_offset: &mut app.scroll_offset,
        },
        &theme,
        ListTable {
            title: "CloudWatch Alarms",
            headers: &["Alarm", "State", "Namespace", "Metric"],
            widths: &[
                Constraint::Percentage(40),
                Constraint::Percentage(15),
                Constraint::Percentage(25),
                Constraint::Percentage(20),
            ],
            empty_message: "No CloudWatch alarms found in this region.",
            filter,
            wrapped,
        },
        &rows,
        |a| {
            let style = if a.state == "ALARM" {
                Style::default().fg(theme.primary)
            } else {
                Style::default().fg(theme.text)
            };

            RowCells {
                cells: vec![
                    a.name.clone(),
                    a.state.clone(),
                    a.namespace.clone(),
                    a.metric.clone(),
                ],
                style,
            }
        },
    );
}
