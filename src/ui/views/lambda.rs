use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let rows: Vec<Row> = app
        .lambda_functions
        .iter()
        .map(|f| {
            Row::new(vec![
                Cell::from(f.name.clone()),
                Cell::from(f.runtime.clone()),
                Cell::from(f.memory_mb.to_string()),
                Cell::from(f.timeout_sec.to_string()),
                Cell::from(f.last_modified.clone()),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        &[
            Constraint::Percentage(30),
            Constraint::Percentage(15),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(35),
        ],
    )
    .header(
        Row::new(vec![
            "Name",
            "Runtime",
            "Memory (MB)",
            "Timeout (s)",
            "Last Modified",
        ])
        .style(Style::default().fg(app.theme.accent)),
    )
    .block(
        Block::default()
            .title("Lambda Functions")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.primary)),
    );

    frame.render_widget(table, area);
}
