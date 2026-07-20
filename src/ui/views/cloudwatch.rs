use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Cell, Row},
    Frame,
};

use crate::app::App;
use crate::ui::views::list_table::{render_list_table, visible_rows, ListSelection, ListTable};

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
        },
        &rows,
        |a| {
            let style = if a.state == "ALARM" {
                Style::default().fg(theme.primary)
            } else {
                Style::default().fg(theme.text)
            };

            Row::new(vec![
                Cell::from(a.name.clone()),
                Cell::from(a.state.clone()),
                Cell::from(a.namespace.clone()),
                Cell::from(a.metric.clone()),
            ])
            .style(style)
        },
    );
}
