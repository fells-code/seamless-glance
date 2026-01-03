use crate::app::App;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

pub fn render_ec2(frame: &mut Frame, area: Rect, app: &App) {
    let rows: Vec<Row> = app
        .ec2_instances
        .iter()
        .map(|i| {
            Row::new(vec![
                Cell::from(i.id.clone()),
                Cell::from(i.name.clone().unwrap_or_else(|| "-".into())),
                Cell::from(i.state.clone()),
                Cell::from(i.instance_type.clone()),
                Cell::from(i.az.clone()),
            ])
        })
        .collect();

    let header = Row::new(vec!["Instance ID", "Name", "State", "Type", "AZ"])
        .style(Style::default().fg(app.theme.primary));

    let widths = [
        ratatui::layout::Constraint::Percentage(30),
        ratatui::layout::Constraint::Percentage(20),
        ratatui::layout::Constraint::Percentage(10),
        ratatui::layout::Constraint::Percentage(20),
        ratatui::layout::Constraint::Percentage(20),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title("EC2 Instances")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.primary)),
        )
        .widths(&[
            ratatui::layout::Constraint::Percentage(30),
            ratatui::layout::Constraint::Percentage(20),
            ratatui::layout::Constraint::Percentage(10),
            ratatui::layout::Constraint::Percentage(20),
            ratatui::layout::Constraint::Percentage(20),
        ]);

    frame.render_widget(table, area);
}
