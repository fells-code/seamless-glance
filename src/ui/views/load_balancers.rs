use crate::app::App;
use crate::ui::views::list_table::{
    filter_query, render_list_table, visible_rows, ListSelection, ListTable, RowCells,
};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    Frame,
};

pub fn render_lbs(frame: &mut Frame, area: Rect, app: &mut App) {
    if crate::ui::views::status::render_unavailable(
        frame,
        area,
        "Load Balancers",
        &app.load_balancers_status,
        &app.theme,
    ) {
        return;
    }

    let theme = app.theme;

    let wrapped = app.wrap_mode_active();
    let filter = filter_query(&app.row_filter);
    let visible = app.visible_indices();
    let rows = visible_rows(&visible, &app.load_balancers);

    render_list_table(
        frame,
        area,
        ListSelection {
            selected_row: &mut app.selected_row,
            scroll_offset: &mut app.scroll_offset,
        },
        &theme,
        ListTable {
            title: "Load Balancers",
            headers: &[
                "Name", "Type", "Scheme", "State", "AZs", "TGs", "Targets", "Healthy", "Signals",
            ],
            widths: &[
                Constraint::Percentage(24),
                Constraint::Length(18),
                Constraint::Length(18),
                Constraint::Length(10),
                Constraint::Length(6),
                Constraint::Length(6),
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Length(18),
            ],
            empty_message: "No load balancers found in this region.",
            filter,
            wrapped,
        },
        &rows,
        |lb| {
            let style = if lb.has_zero_healthy_targets() {
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD)
            } else if lb.has_no_active_targets() {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.text)
            };

            RowCells {
                cells: vec![
                    lb.name.clone(),
                    lb.lb_type.clone(),
                    lb.scheme.clone(),
                    lb.state.clone(),
                    lb.az_count.to_string(),
                    lb.attached_target_groups.to_string(),
                    lb.total_targets.to_string(),
                    lb.healthy_targets.to_string(),
                    {
                        let signals = lb.review_signals();
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
