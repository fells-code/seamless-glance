use crate::app::App;
use crate::models::service_status::ServiceStatus;
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

pub fn render_rds(frame: &mut Frame, area: Rect, app: &mut App) {
    let total_rows = app.rds_instances.len();
    if total_rows == 0 {
        app.selected_row = 0;
        app.scroll_offset = 0;
    }

    if total_rows > 0 {
        app.selected_row = app.selected_row.min(total_rows - 1);
    }

    let visible_height = area.height.saturating_sub(3) as usize;

    if app.selected_row < app.scroll_offset as usize {
        app.scroll_offset = app.selected_row as u16;
    } else if app.selected_row >= app.scroll_offset as usize + visible_height {
        app.scroll_offset = (app.selected_row + 1 - visible_height) as u16;
    }

    if !matches!(app.rds_summary.status, ServiceStatus::Ok) {
        let msg = match &app.rds_summary.status {
            ServiceStatus::AccessDenied => "Access denied to RDS".to_string(),
            ServiceStatus::Unavailable(msg) => {
                if msg.is_empty() {
                    "RDS unavailable".to_string()
                } else {
                    format!("RDS unavailable: {}", msg)
                }
            }
            _ => String::new(),
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
        .enumerate()
        .skip(app.scroll_offset as usize)
        .take(visible_height)
        .map(|(i, db)| {
            let style = if i == app.selected_row {
                Style::default().fg(app.theme.highlight)
            } else {
                Style::default().fg(app.theme.text)
            };

            Row::new(vec![
                Cell::from(db.identifier.clone()),
                Cell::from(db.region.clone()),
                Cell::from(db.engine.clone()),
                Cell::from(db.instance_class.clone()),
                Cell::from(db.status.clone()),
                Cell::from(db.az.clone()),
                Cell::from(if db.multi_az { "Yes" } else { "No" }),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Percentage(22),
        Constraint::Percentage(14),
        Constraint::Percentage(12),
        Constraint::Percentage(18),
        Constraint::Percentage(10),
        Constraint::Percentage(14),
        Constraint::Percentage(10),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec![
                "Identifier",
                "Region",
                "Engine",
                "Class",
                "Status",
                "AZ",
                "Multi-AZ",
            ])
            .style(Style::default().fg(app.theme.accent)),
        )
        .block(
            Block::default()
                .title("RDS Instances")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.primary)),
        );

    frame.render_widget(table, area);
}
