use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Cell, Row},
    Frame,
};

use crate::app::App;
use crate::ui::views::list_table::{render_list_table, ListSelection, ListTable};

pub fn render_vpc(frame: &mut Frame, area: Rect, app: &mut App) {
    if crate::ui::views::status::render_unavailable(frame, area, "VPC", &app.vpc_status, &app.theme)
    {
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
        },
        &app.vpcs,
        |v| {
            let style = if v.is_default {
                Style::default().fg(theme.primary)
            } else {
                Style::default().fg(theme.text)
            };

            Row::new(vec![
                Cell::from(v.vpc_id.clone()),
                Cell::from(v.cidr.clone()),
                Cell::from(v.state.clone()),
                Cell::from(if v.is_default { "Yes" } else { "No" }),
                Cell::from(v.subnet_count.to_string()),
            ])
            .style(style)
        },
    );
}
