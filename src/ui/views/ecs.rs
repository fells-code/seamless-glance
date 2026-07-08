use crate::app::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

pub fn render_ecs_clusters(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = &app.theme;

    let total_rows = app.ecs_clusters.len();
    if total_rows == 0 {
        app.selected_row = 0;
        app.scroll_offset = 0;
    }

    if total_rows > 0 {
        app.selected_row = app.selected_row.min(total_rows - 1);
    }

    let visible_height = area.height.saturating_sub(3) as usize; // header + borders

    // Keep selected row in view
    if app.selected_row < app.scroll_offset as usize {
        app.scroll_offset = app.selected_row as u16;
    } else if app.selected_row >= app.scroll_offset as usize + visible_height {
        app.scroll_offset = (app.selected_row + 1 - visible_height) as u16;
    }

    let rows: Vec<Row> = app
        .ecs_clusters
        .iter()
        .enumerate()
        .skip(app.scroll_offset as usize)
        .take(visible_height)
        .map(|(i, c)| {
            let style = if i == app.selected_row {
                Style::default().fg(app.theme.highlight)
            } else {
                Style::default().fg(app.theme.text)
            };
            Row::new(vec![
                Cell::from(c.name.clone()),
                Cell::from(c.active_services.to_string()),
                Cell::from(format!("{} / {}", c.running_tasks, c.pending_tasks)),
                Cell::from(c.registered_container_instances.to_string()),
                Cell::from(c.cpu.to_string()),
                Cell::from(c.memory.to_string()),
                Cell::from("OK"),
            ])
            .style(style)
        })
        .collect();

    let header = Row::new(vec![
        Cell::from("Cluster"),
        Cell::from("Services"),
        Cell::from("Tasks (R/P)"),
        Cell::from("EC2s"),
        Cell::from("CPU"),
        Cell::from("Memory"),
        Cell::from("Health"),
    ])
    .style(Style::default().fg(theme.accent));

    let table = Table::new(
        rows,
        &[
            Constraint::Percentage(30),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(10),
        ],
    )
    .header(header)
    .widths([
        Constraint::Percentage(30),
        Constraint::Length(10),
        Constraint::Length(12),
        Constraint::Length(8),
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Length(10),
    ])
    .block(
        Block::default()
            .title("ECS Clusters")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.primary)),
    )
    .column_spacing(1);

    frame.render_widget(table, area);
}
