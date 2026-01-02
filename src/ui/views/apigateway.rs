use ratatui::{
    layout::Constraint,
    style::Style,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

use crate::app::App;

pub fn render_apigatway(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let rows: Vec<Row> = app
        .apigateway_apis
        .iter()
        .map(|api| {
            Row::new(vec![
                Cell::from(api.name.clone()),
                Cell::from(api.api_type.clone()),
                Cell::from(api.id.clone()),
                Cell::from(api.created_at.clone()),
            ])
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
