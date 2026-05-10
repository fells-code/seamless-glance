use crate::app::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

pub fn render_sg(frame: &mut Frame, area: Rect, app: &App) {
    if app.security_groups.is_empty() {
        let empty = ratatui::widgets::Paragraph::new(
            "No security groups found in this region.\n\
             This is uncommon and may indicate a highly restricted account.",
        )
        .block(
            Block::default()
                .title("Security Groups")
                .borders(Borders::ALL),
        );

        frame.render_widget(empty, area);
        return;
    }

    let rows = app.security_groups.iter().enumerate().map(|(i, sg)| {
        let world = if sg.open_to_world { "yes" } else { "no" };
        let sensitive = sg.sensitive_ports_label();

        let style = if i == app.selected_row {
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD)
        } else if !sg.sensitive_public_ports.is_empty() {
            Style::default().fg(app.theme.primary)
        } else if sg.open_to_world {
            // Subtle warning, not alarmist
            Style::default().fg(app.theme.primary)
        } else {
            Style::default()
        };

        Row::new(vec![
            Cell::from(sg.id.clone()),
            Cell::from(sg.name.clone()),
            Cell::from(sg.inbound_rules.to_string()),
            Cell::from(sg.outbound_rules.to_string()),
            Cell::from(world),
            Cell::from(sensitive),
            Cell::from(sg.vpc_id.clone()),
        ])
        .style(style)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(12), // SG ID
            Constraint::Percentage(24),
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Length(7),
            Constraint::Length(12),
            Constraint::Percentage(25),
        ],
    )
    .header(
        Row::new(["ID", "NAME", "IN", "OUT", "WORLD", "SENSITIVE", "VPC"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .title("Security Groups")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.primary)),
    );

    frame.render_widget(table, area);
}
