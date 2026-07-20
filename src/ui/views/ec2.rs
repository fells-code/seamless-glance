use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    Frame,
};

use crate::app::App;
use crate::ui::views::list_table::{
    filter_query, render_list_table, visible_rows, ListSelection, ListTable, RowCells,
};

pub fn render_ec2(frame: &mut Frame, area: Rect, app: &mut App) {
    if crate::ui::views::status::render_unavailable(frame, area, "EC2", &app.ec2_status, &app.theme)
    {
        return;
    }

    let theme = app.theme;

    let wrapped = app.wrap_mode_active();
    let filter = filter_query(&app.row_filter);
    let visible = app.visible_indices();
    let rows = visible_rows(&visible, &app.ec2_instances);

    render_list_table(
        frame,
        area,
        ListSelection {
            selected_row: &mut app.selected_row,
            scroll_offset: &mut app.scroll_offset,
        },
        &theme,
        ListTable {
            title: "EC2 Instances",
            headers: &[
                "Instance ID",
                "Name",
                "Region",
                "State",
                "Type",
                "Avg CPU",
                "Owner",
                "Env",
                "Public IP",
                "Private IP",
                "AZ",
                "Signals",
            ],
            widths: &[
                Constraint::Percentage(30),
                Constraint::Percentage(16),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(9),
                Constraint::Percentage(10),
                Constraint::Percentage(9),
                Constraint::Percentage(11),
                Constraint::Percentage(11),
                Constraint::Percentage(8),
                Constraint::Percentage(12),
            ],
            empty_message: "No EC2 instances found in this region.",
            filter,
            wrapped,
        },
        &rows,
        |inst| {
            let style = if inst.has_tag_coverage_gap() {
                Style::default().fg(theme.accent)
            } else if inst.has_sustained_low_cpu() || inst.needs_stopped_review() {
                Style::default().fg(theme.primary)
            } else {
                Style::default().fg(theme.text)
            };

            RowCells {
                cells: vec![
                    inst.id.clone(),
                    inst.name().unwrap_or("-").to_string(),
                    inst.region.clone(),
                    inst.state.clone(),
                    inst.instance_type.clone(),
                    inst.formatted_avg_cpu(),
                    inst.owner().unwrap_or("-").to_string(),
                    inst.environment().unwrap_or("-").to_string(),
                    inst.public_ip.clone().unwrap_or("-".into()),
                    inst.private_ip.clone().unwrap_or("-".into()),
                    inst.az.clone(),
                    {
                        let signals = inst.review_signals();
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
