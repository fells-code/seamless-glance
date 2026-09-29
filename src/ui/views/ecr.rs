use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    Frame,
};

use chrono::{DateTime, Utc};

use crate::app::App;
use crate::ui::views::list_table::{
    filter_query, render_list_table, visible_rows, ListSelection, ListTable, RowCells,
};

fn date_label(date: Option<DateTime<Utc>>) -> String {
    date.map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "-".into())
}

pub fn render_ecr(frame: &mut Frame, area: Rect, app: &mut App) {
    if crate::ui::views::status::render_unavailable(frame, area, "ECR", &app.ecr_status, &app.theme)
    {
        return;
    }

    let theme = app.theme;

    let wrapped = app.wrap_mode_active();
    let filter = filter_query(&app.row_filter);
    let visible = app.visible_indices();
    let rows = visible_rows(&visible, &app.ecr_repositories);

    render_list_table(
        frame,
        area,
        ListSelection {
            selected_row: &mut app.selected_row,
            scroll_offset: &mut app.scroll_offset,
        },
        &theme,
        ListTable {
            title: "ECR Repositories",
            headers: &[
                "Name",
                "Images",
                "Untagged",
                "Size",
                "Last Push",
                "Last Pull",
                "Lifecycle",
                "Signals",
            ],
            widths: &[
                Constraint::Percentage(26),
                Constraint::Percentage(8),
                Constraint::Percentage(9),
                Constraint::Percentage(10),
                Constraint::Percentage(11),
                Constraint::Percentage(11),
                Constraint::Percentage(9),
                Constraint::Percentage(16),
            ],
            empty_message: "No ECR repositories found in this region.",
            filter,
            wrapped,
        },
        &rows,
        |r| {
            let style = if r.has_untagged_buildup() || r.is_stale() {
                Style::default().fg(theme.primary)
            } else if r.lacks_lifecycle_policy() {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.text)
            };

            let (count, untagged, last_pushed, last_pulled) = match &r.images {
                Some(images) => (
                    images.count.to_string(),
                    images.untagged.to_string(),
                    date_label(images.last_pushed),
                    date_label(images.last_pulled),
                ),
                None => ("?".into(), "?".into(), "-".into(), "-".into()),
            };

            let signals = r.review_signals();

            RowCells {
                cells: vec![
                    r.name.clone(),
                    count,
                    untagged,
                    r.size_label(),
                    last_pushed,
                    last_pulled,
                    match r.has_lifecycle_policy {
                        Some(true) => "Yes",
                        Some(false) => "No",
                        None => "?",
                    }
                    .to_string(),
                    if signals.is_empty() {
                        "-".into()
                    } else {
                        signals.join(",")
                    },
                ],
                style,
            }
        },
    );
}
