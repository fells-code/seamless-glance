use crate::app::App;
use crate::ui::views::list_table::{
    filter_query, render_list_table, visible_rows, ListSelection, ListTable, RowCells,
};
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
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

    let wrapped = app.wrap_mode_active();
    let filter = filter_query(&app.row_filter);
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
            filter,
            wrapped,
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

            RowCells {
                cells: vec![
                    sg.id.clone(),
                    sg.name.clone(),
                    sg.inbound_rules.to_string(),
                    sg.outbound_rules.to_string(),
                    if sg.open_to_world { "yes" } else { "no" }.to_string(),
                    sg.sensitive_ports_label(),
                    sg.vpc_id.clone(),
                ],
                style,
            }
        },
    );
}
