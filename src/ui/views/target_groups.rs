use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Row, Table},
    Frame,
};

pub fn render_tg(frame: &mut Frame, area: Rect, app: &App) {
    if app.target_groups.is_empty() {
        let empty = ratatui::widgets::Paragraph::new(
            "No target groups found in this region.\n\
             This is normal if no load balancers or services are deployed.",
        )
        .block(
            Block::default()
                .title("Target Groups")
                .borders(Borders::ALL),
        );

        frame.render_widget(empty, area);
        return;
    }

    let rows = app.target_groups.iter().enumerate().map(|(i, tg)| {
        let unhealthy = tg.unhealthy_targets > 0;

        let style = if i == app.selected_row {
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD)
        } else if unhealthy {
            Style::default().fg(app.theme.primary)
        } else {
            Style::default()
        };

        Row::new(vec![
            tg.name.clone(),
            tg.target_type.clone(),
            tg.port.to_string(),
            tg.total_targets.to_string(),
            tg.unhealthy_targets.to_string(),
        ])
        .style(style)
    });

    let table = Table::new(
        rows,
        [
            ratatui::layout::Constraint::Percentage(35),
            ratatui::layout::Constraint::Length(10),
            ratatui::layout::Constraint::Length(6),
            ratatui::layout::Constraint::Length(8),
            ratatui::layout::Constraint::Length(10),
        ],
    )
    .header(
        Row::new(["NAME", "TYPE", "PORT", "TARGETS", "UNHEALTHY"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .title("Target Groups")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.primary)),
    );

    frame.render_widget(table, area);
}
