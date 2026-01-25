use crate::app::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

pub fn render_lbs(frame: &mut Frame, area: Rect, app: &mut App) {
    let total_rows = app.load_balancers.len();
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
        .load_balancers
        .iter()
        .enumerate()
        .skip(app.scroll_offset as usize)
        .take(visible_height)
        .map(|(i, lb)| {
            let style = if i == app.selected_row {
                Style::default().fg(app.theme.highlight)
            } else {
                Style::default().fg(app.theme.text)
            };
            Row::new(vec![
                Cell::from(lb.name.clone()),
                Cell::from(lb.lb_type.clone()),
                Cell::from(lb.scheme.clone()),
                Cell::from(lb.state.clone()),
                Cell::from(lb.az_count.to_string()),
            ])
            .style(style)
        })
        .collect();

    let header = Row::new(vec![
        Cell::from("Name"),
        Cell::from("Type"),
        Cell::from("Scheme"),
        Cell::from("State"),
        Cell::from("AZs"),
    ])
    .style(Style::default().fg(app.theme.accent));

    let table = Table::new(
        rows,
        &[
            Constraint::Percentage(30),
            Constraint::Length(20),
            Constraint::Length(20),
            Constraint::Length(10),
            Constraint::Length(10),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title("Load Balancers")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.primary)),
    );

    frame.render_widget(table, area);
}
