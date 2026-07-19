use crate::app::App;
use crate::ui::views::list_table::{render_list_table, ListSelection, ListTable};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Cell, Row},
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
        },
        &app.load_balancers,
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

            Row::new(vec![
                Cell::from(lb.name.clone()),
                Cell::from(lb.lb_type.clone()),
                Cell::from(lb.scheme.clone()),
                Cell::from(lb.state.clone()),
                Cell::from(lb.az_count.to_string()),
                Cell::from(lb.attached_target_groups.to_string()),
                Cell::from(lb.total_targets.to_string()),
                Cell::from(lb.healthy_targets.to_string()),
                Cell::from({
                    let signals = lb.review_signals();
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
