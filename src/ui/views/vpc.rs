use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

use crate::app::App;

pub fn render_vpc(frame: &mut Frame, area: Rect, app: &App) {
    let rows: Vec<Row> = app
        .vpcs
        .iter()
        .map(|v| {
            Row::new(vec![
                Cell::from(v.vpc_id.clone()),
                Cell::from(v.cidr.clone()),
                Cell::from(v.state.clone()),
                Cell::from(if v.is_default { "Yes" } else { "No" }),
                Cell::from(v.subnet_count.to_string()),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        &[
            Constraint::Percentage(28),
            Constraint::Percentage(22),
            Constraint::Percentage(12),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
        ],
    )
    .header(
        Row::new(vec!["VPC ID", "CIDR", "State", "Default", "Subnets"])
            .style(Style::default().fg(app.theme.accent)),
    )
    .block(
        Block::default()
            .title("VPCs")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.primary)),
    );

    frame.render_widget(table, area);
}
