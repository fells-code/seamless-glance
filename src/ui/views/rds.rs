use crate::app::App;
use crate::models::service_status::ServiceStatus;
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

pub fn render_rds(frame: &mut Frame, area: Rect, app: &App) {
    if !matches!(app.rds_summary.status, ServiceStatus::Ok) {
        let msg = match app.rds_summary.status {
            ServiceStatus::AccessDenied => "Access denied to RDS",
            ServiceStatus::Unavailable(_) => "RDS unavailable",
            _ => "",
        };

        let p = Paragraph::new(msg)
            .style(Style::default().fg(app.theme.accent))
            .block(Block::default().title("RDS").borders(Borders::ALL));

        frame.render_widget(p, area);
        return;
    }

    let rows: Vec<Row> = app
        .rds_instances
        .iter()
        .map(|db| {
            Row::new(vec![
                Cell::from(db.identifier.clone()),
                Cell::from(db.engine.clone()),
                Cell::from(db.instance_class.clone()),
                Cell::from(db.status.clone()),
                Cell::from(db.az.clone()),
                Cell::from(if db.multi_az { "Yes" } else { "No" }),
            ])
        })
        .collect();

    let widths = [
        Constraint::Percentage(30),
        Constraint::Percentage(15),
        Constraint::Percentage(20),
        Constraint::Percentage(10),
        Constraint::Percentage(15),
        Constraint::Percentage(10),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec![
                "Identifier",
                "Engine",
                "Class",
                "Status",
                "AZ",
                "Multi-AZ",
            ])
            .style(Style::default().fg(app.theme.primary)),
        )
        .block(
            Block::default()
                .title("RDS Instances")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.primary)),
        );

    frame.render_widget(table, area);
}
