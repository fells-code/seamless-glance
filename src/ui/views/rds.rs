use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Cell, Row},
    Frame,
};

use crate::app::App;
use crate::ui::views::list_table::{render_list_table, ListSelection, ListTable};

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
        },
        &app.rds_instances,
        |db| {
            let style = if db.status != "available" {
                Style::default().fg(theme.primary)
            } else if db.needs_single_az_review() {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.text)
            };

            Row::new(vec![
                Cell::from(db.identifier.clone()),
                Cell::from(db.region.clone()),
                Cell::from(db.engine.clone()),
                Cell::from(db.instance_class.clone()),
                Cell::from(db.status.clone()),
                Cell::from(db.az.clone()),
                Cell::from(if db.multi_az { "Yes" } else { "No" }),
                Cell::from({
                    let signals = db.review_signals();
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
