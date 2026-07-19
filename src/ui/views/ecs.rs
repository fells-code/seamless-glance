use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Cell, Row},
    Frame,
};

use crate::app::App;
use crate::ui::views::list_table::{render_list_table, ListSelection, ListTable};

pub fn render_ecs_clusters(frame: &mut Frame, area: Rect, app: &mut App) {
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
            title: "ECS Clusters",
            headers: &[
                "Cluster",
                "Services",
                "Tasks (R/P)",
                "EC2s",
                "CPU",
                "Memory",
                "Health",
            ],
            widths: &[
                Constraint::Percentage(30),
                Constraint::Length(10),
                Constraint::Length(12),
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Length(10),
                Constraint::Length(10),
            ],
            empty_message: "No ECS clusters found in this region.",
        },
        &app.ecs_clusters,
        |c| {
            Row::new(vec![
                Cell::from(c.name.clone()),
                Cell::from(c.active_services.to_string()),
                Cell::from(format!("{} / {}", c.running_tasks, c.pending_tasks)),
                Cell::from(c.registered_container_instances.to_string()),
                Cell::from(c.cpu.to_string()),
                Cell::from(c.memory.to_string()),
                // TODO(#43): cluster health is a placeholder, not yet computed.
                Cell::from("OK"),
            ])
            .style(Style::default().fg(theme.text))
        },
    );
}
