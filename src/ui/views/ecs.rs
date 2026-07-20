use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    Frame,
};

use crate::app::App;
use crate::ui::views::list_table::{
    filter_query, render_list_table, visible_rows, ListSelection, ListTable, RowCells,
};

pub fn render_ecs_clusters(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = app.theme;

    let wrapped = app.wrap_mode_active();
    let filter = filter_query(&app.row_filter);
    let visible = app.visible_indices();
    let rows = visible_rows(&visible, &app.ecs_clusters);

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
            filter,
            wrapped,
        },
        &rows,
        |c| {
            RowCells {
                cells: vec![
                    c.name.clone(),
                    c.active_services.to_string(),
                    format!("{} / {}", c.running_tasks, c.pending_tasks),
                    c.registered_container_instances.to_string(),
                    c.cpu.to_string(),
                    c.memory.to_string(),
                    // TODO(#43): cluster health is a placeholder, not yet computed.
                    "OK".to_string(),
                ],
                style: Style::default().fg(theme.text),
            }
        },
    );
}
