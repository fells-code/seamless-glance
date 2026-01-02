use crate::app::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, List, ListItem, Row, Table},
    Frame,
};

pub fn render_ecs_clusters(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    // Build table rows
    let rows: Vec<Row> = app
        .ecs_clusters
        .iter()
        .map(|c| {
            Row::new(vec![
                Cell::from(c.name.clone()),
                Cell::from(c.active_services.to_string()),
                Cell::from(format!("{} / {}", c.running_tasks, c.pending_tasks)),
                Cell::from(c.registered_container_instances.to_string()),
                Cell::from(c.cpu.to_string()),
                Cell::from(c.memory.to_string()),
                Cell::from("OK"), // placeholder health
            ])
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
    .widths(&[
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

pub fn render_ecs_services(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .ecs_services
        .iter()
        .map(|s| {
            ListItem::new(format!(
                "{} | {}/{} running | deployments: {}",
                s.name,
                s.running_count,
                s.desired_count,
                s.deployments.len(),
            ))
        })
        .collect();

    let list = List::new(items).block(Block::default().title("ECS Services").borders(Borders::ALL));

    frame.render_widget(list, area);
}

pub fn render_ecs_tasks(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .ecs_tasks
        .iter()
        .map(|t| {
            ListItem::new(format!(
                "{} | {} -> {} | CPU: {:?} | Mem: {:?}",
                t.task_definition, t.desired_status, t.last_status, t.cpu, t.memory
            ))
        })
        .collect();

    let list = List::new(items).block(Block::default().title("ECS Tasks").borders(Borders::ALL));

    frame.render_widget(list, area);
}
