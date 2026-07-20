use crate::app::App;
use crate::ui::views::list_table::{render_list_table, visible_rows, ListSelection, ListTable};
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Cell, Row},
    Frame,
};

pub fn render_sg(frame: &mut Frame, area: Rect, app: &mut App) {
    if crate::ui::views::status::render_unavailable(
        frame,
        area,
        "Security Groups",
        &app.security_groups_status,
        &app.theme,
    ) {
        return;
    }

    let theme = app.theme;

    let visible = app.visible_indices();
    let rows = visible_rows(&visible, &app.security_groups);

    render_list_table(
        frame,
        area,
        ListSelection {
            selected_row: &mut app.selected_row,
            scroll_offset: &mut app.scroll_offset,
        },
        &theme,
        ListTable {
            title: "Security Groups",
            headers: &["ID", "NAME", "IN", "OUT", "WORLD", "SENSITIVE", "VPC"],
            widths: &[
                Constraint::Length(12),
                Constraint::Percentage(24),
                Constraint::Length(6),
                Constraint::Length(6),
                Constraint::Length(7),
                Constraint::Length(12),
                Constraint::Percentage(25),
            ],
            empty_message: "No security groups found in this region.\n\
                            This is uncommon and may indicate a highly restricted account.",
        },
        &rows,
        |sg| {
            // Sensitive public ports and world-open both read as a subtle
            // warning rather than an alarm.
            let style = if !sg.sensitive_public_ports.is_empty() || sg.open_to_world {
                Style::default().fg(theme.primary)
            } else {
                Style::default().fg(theme.text)
            };

            Row::new(vec![
                Cell::from(sg.id.clone()),
                Cell::from(sg.name.clone()),
                Cell::from(sg.inbound_rules.to_string()),
                Cell::from(sg.outbound_rules.to_string()),
                Cell::from(if sg.open_to_world { "yes" } else { "no" }),
                Cell::from(sg.sensitive_ports_label()),
                Cell::from(sg.vpc_id.clone()),
            ])
            .style(style)
        },
    );
}
