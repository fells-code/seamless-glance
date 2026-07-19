use crate::app::App;
use crate::ui::views::list_table::{render_list_table, ListSelection, ListTable};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Cell, Row},
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
        },
        &app.target_groups,
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

            Row::new(vec![
                Cell::from(tg.name.clone()),
                Cell::from(tg.protocol.clone()),
                Cell::from(tg.target_type.clone()),
                Cell::from(tg.port.to_string()),
                Cell::from(tg.attached_load_balancer_count().to_string()),
                Cell::from(tg.total_targets.to_string()),
                Cell::from(tg.healthy_targets().to_string()),
                Cell::from(tg.unhealthy_targets.to_string()),
                Cell::from({
                    let signals = tg.review_signals();
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
