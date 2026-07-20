use crate::app::App;
use crate::ui::views::list_table::{
    filter_query, render_list_table, visible_rows, ListSelection, ListTable, RowCells,
};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    Frame,
};

pub fn render_tg(frame: &mut Frame, area: Rect, app: &mut App) {
    if crate::ui::views::status::render_unavailable(
        frame,
        area,
        "Target Groups",
        &app.target_groups_status,
        &app.theme,
    ) {
        return;
    }

    let theme = app.theme;

    let wrapped = app.wrap_mode_active();
    let filter = filter_query(&app.row_filter);
    let visible = app.visible_indices();
    let rows = visible_rows(&visible, &app.target_groups);

    render_list_table(
        frame,
        area,
        ListSelection {
            selected_row: &mut app.selected_row,
            scroll_offset: &mut app.scroll_offset,
        },
        &theme,
        ListTable {
            title: "Target Groups",
            headers: &[
                "NAME",
                "PROTO",
                "TYPE",
                "PORT",
                "LBS",
                "TARGETS",
                "HEALTHY",
                "UNHEALTHY",
                "SIGNALS",
            ],
            widths: &[
                Constraint::Percentage(24),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(6),
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Length(10),
                Constraint::Length(16),
            ],
            empty_message: "No target groups found in this region.\n\
                            This is normal if no load balancers or services are deployed.",
            filter,
            wrapped,
        },
        &rows,
        |tg| {
            let style = if tg.has_zero_healthy_targets() {
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD)
            } else if tg.is_orphan_candidate() {
                Style::default().fg(theme.accent)
            } else if tg.unhealthy_targets > 0 {
                Style::default().fg(theme.primary)
            } else {
                Style::default().fg(theme.text)
            };

            RowCells {
                cells: vec![
                    tg.name.clone(),
                    tg.protocol.clone(),
                    tg.target_type.clone(),
                    tg.port.to_string(),
                    tg.attached_load_balancer_count().to_string(),
                    tg.total_targets.to_string(),
                    tg.healthy_targets().to_string(),
                    tg.unhealthy_targets.to_string(),
                    {
                        let signals = tg.review_signals();
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
