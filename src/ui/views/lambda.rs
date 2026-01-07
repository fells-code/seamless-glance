use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let total_rows = app.lambda_functions.len();
    if total_rows == 0 {
        app.selected_row = 0;
        app.scroll_offset = 0;
    }

    // Clamp selection to bounds
    if total_rows > 0 {
        app.selected_row = app.selected_row.min(total_rows - 1);
    }

    // How many rows can we show?
    let visible_height = area.height.saturating_sub(3) as usize; // header + borders

    // Keep selected row in view
    if app.selected_row < app.scroll_offset as usize {
        app.scroll_offset = app.selected_row as u16;
    } else if app.selected_row >= app.scroll_offset as usize + visible_height {
        app.scroll_offset = (app.selected_row + 1 - visible_height) as u16;
    }

    let rows: Vec<Row> = app
        .lambda_functions
        .iter()
        .enumerate()
        .skip(app.scroll_offset as usize)
        .take(visible_height)
        .map(|(i, f)| {
            let style = if i == app.selected_row {
                Style::default()
                    .fg(app.theme.background)
                    .bg(app.theme.primary)
            } else {
                Style::default().fg(app.theme.text)
            };
            Row::new(vec![
                Cell::from(f.name.clone()),
                Cell::from(f.runtime.clone()),
                Cell::from(f.memory_mb.to_string()),
                Cell::from(f.timeout_sec.to_string()),
                Cell::from(f.last_modified.clone()),
            ])
            .style(style)
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
