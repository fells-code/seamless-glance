use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Cell, Row},
    Frame,
};

use crate::app::App;
use crate::ui::views::list_table::{render_list_table, ListSelection, ListTable};

pub fn render_ec2(frame: &mut Frame, area: Rect, app: &mut App) {
    if crate::ui::views::status::render_unavailable(frame, area, "EC2", &app.ec2_status, &app.theme)
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
        },
        &app.ec2_instances,
        |inst| {
            let style = if inst.has_tag_coverage_gap() {
                Style::default().fg(theme.accent)
            } else if inst.has_sustained_low_cpu() || inst.needs_stopped_review() {
                Style::default().fg(theme.primary)
            } else {
                Style::default().fg(theme.text)
            };

            Row::new(vec![
                Cell::from(inst.id.clone()),
                Cell::from(inst.name().unwrap_or("-").to_string()),
                Cell::from(inst.region.clone()),
                Cell::from(inst.state.clone()),
                Cell::from(inst.instance_type.clone()),
                Cell::from(inst.formatted_avg_cpu()),
                Cell::from(inst.owner().unwrap_or("-").to_string()),
                Cell::from(inst.environment().unwrap_or("-").to_string()),
                Cell::from(inst.public_ip.clone().unwrap_or("-".into())),
                Cell::from(inst.private_ip.clone().unwrap_or("-".into())),
                Cell::from(inst.az.clone()),
                Cell::from({
                    let signals = inst.review_signals();
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
