use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    Frame,
};

use crate::app::App;
use crate::ui::views::list_table::{
    filter_query, render_list_table, visible_rows, ListSelection, ListTable, RowCells,
};

pub fn render_vpc(frame: &mut Frame, area: Rect, app: &mut App) {
    if crate::ui::views::status::render_unavailable(frame, area, "VPC", &app.vpc_status, &app.theme)
    {
        return;
    }

    let theme = app.theme;

    let wrapped = app.wrap_mode_active();
    let filter = filter_query(&app.row_filter);
    let visible = app.visible_indices();
    let rows = visible_rows(&visible, &app.vpcs);

    render_list_table(
        frame,
        area,
        ListSelection {
            selected_row: &mut app.selected_row,
            scroll_offset: &mut app.scroll_offset,
        },
        &theme,
        ListTable {
            title: "VPCs",
            headers: &["VPC ID", "CIDR", "State", "Default", "Subnets"],
            widths: &[
                Constraint::Percentage(28),
                Constraint::Percentage(22),
                Constraint::Percentage(12),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
            ],
            empty_message: "No VPCs found in this region.",
            filter,
            wrapped,
        },
        &rows,
        |v| {
            let style = if v.is_default {
                Style::default().fg(theme.primary)
            } else {
                Style::default().fg(theme.text)
            };

            RowCells {
                cells: vec![
                    v.vpc_id.clone(),
                    v.cidr.clone(),
                    v.state.clone(),
                    if v.is_default { "Yes" } else { "No" }.to_string(),
                    v.subnet_count.to_string(),
                ],
                style,
            }
        },
    );
}
