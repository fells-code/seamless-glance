use ratatui::{
    layout::Constraint,
    style::Style,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

use crate::app::App;

pub fn render_apigatway(frame: &mut Frame, area: ratatui::layout::Rect, app: &mut App) {
    let total_rows = app.rds_instances.len();
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
        .apigateway_apis
        .iter()
        .enumerate()
        .skip(app.scroll_offset as usize)
        .take(visible_height)
        .map(|(i, api)| {
            let style = if i == app.selected_row {
                Style::default().fg(app.theme.highlight)
            } else {
                Style::default().fg(app.theme.text)
            };

            Row::new(vec![
                Cell::from(api.name.clone()),
                Cell::from(api.api_type.clone()),
                Cell::from(api.id.clone()),
                Cell::from(api.created_at.clone()),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        &[
            Constraint::Percentage(30),
            Constraint::Percentage(10),
            Constraint::Percentage(20),
            Constraint::Percentage(40),
        ],
    )
    .header(
        Row::new(vec!["Name", "Type", "ID", "Created"])
            .style(Style::default().fg(app.theme.accent)),
    )
    .block(
        Block::default()
            .title("API Gateway")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.primary)),
    );

    frame.render_widget(table, area);
}
