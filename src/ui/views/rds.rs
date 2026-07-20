use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    Frame,
};

use crate::app::App;
use crate::ui::views::list_table::{
    filter_query, render_list_table, visible_rows, ListSelection, ListTable, RowCells,
};

pub fn render_rds(frame: &mut Frame, area: Rect, app: &mut App) {
    if crate::ui::views::status::render_unavailable(
        frame,
        area,
        "RDS",
        &app.rds_summary.status,
        &app.theme,
    ) {
        return;
    }

    let theme = app.theme;

    let wrapped = app.wrap_mode_active();
    let filter = filter_query(&app.row_filter);
    let visible = app.visible_indices();
    let rows = visible_rows(&visible, &app.rds_instances);

    render_list_table(
        frame,
        area,
        ListSelection {
            selected_row: &mut app.selected_row,
            scroll_offset: &mut app.scroll_offset,
        },
        &theme,
        ListTable {
            title: "RDS Instances",
            headers: &[
                "Identifier",
                "Region",
                "Engine",
                "Class",
                "Status",
                "AZ",
                "Multi-AZ",
                "Signals",
            ],
            widths: &[
                Constraint::Percentage(22),
                Constraint::Percentage(14),
                Constraint::Percentage(12),
                Constraint::Percentage(18),
                Constraint::Percentage(10),
                Constraint::Percentage(14),
                Constraint::Percentage(10),
                Constraint::Percentage(14),
            ],
            empty_message: "No RDS instances found in this region.",
            filter,
            wrapped,
        },
        &rows,
        |db| {
            let style = if db.status != "available" {
                Style::default().fg(theme.primary)
            } else if db.needs_single_az_review() {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.text)
            };

            RowCells {
                cells: vec![
                    db.identifier.clone(),
                    db.region.clone(),
                    db.engine.clone(),
                    db.instance_class.clone(),
                    db.status.clone(),
                    db.az.clone(),
                    if db.multi_az { "Yes" } else { "No" }.to_string(),
                    {
                        let signals = db.review_signals();
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
