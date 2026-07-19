use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

pub fn render_tg(frame: &mut Frame, area: Rect, app: &mut App) {
    if crate::ui::views::status::render_unavailable(
        frame,
        area,
        "Target Groups",
        &app.target_groups_status,
        &app.theme,
    ) {
        return;
    }

    let total_rows = app.target_groups.len();
    if total_rows == 0 {
        app.selected_row = 0;
        app.scroll_offset = 0;

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

    app.selected_row = app.selected_row.min(total_rows - 1);

    let visible_height = area.height.saturating_sub(3) as usize;

    if app.selected_row < app.scroll_offset as usize {
        app.scroll_offset = app.selected_row as u16;
    } else if app.selected_row >= app.scroll_offset as usize + visible_height {
        app.scroll_offset = (app.selected_row + 1 - visible_height) as u16;
    }

    let rows = app
        .target_groups
        .iter()
        .enumerate()
        .skip(app.scroll_offset as usize)
        .take(visible_height)
        .map(|(i, tg)| {
            let unhealthy = tg.unhealthy_targets > 0;
            let zero_healthy = tg.has_zero_healthy_targets();
            let orphan = tg.is_orphan_candidate();

            let style = if i == app.selected_row {
                Style::default()
                    .fg(app.theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else if zero_healthy {
                Style::default()
                    .fg(app.theme.primary)
                    .add_modifier(Modifier::BOLD)
            } else if orphan {
                Style::default().fg(app.theme.accent)
            } else if unhealthy {
                Style::default().fg(app.theme.primary)
            } else {
                Style::default().fg(app.theme.text)
            };

            Row::new(vec![
                Cell::from(tg.name.clone()),
                Cell::from(tg.protocol.clone()),
                Cell::from(tg.target_type.clone()),
                Cell::from(tg.port.to_string()),
                Cell::from(tg.attached_load_balancer_count().to_string()),
                Cell::from(tg.total_targets.to_string()),
                Cell::from(tg.healthy_targets().to_string()),
                Cell::from(tg.unhealthy_targets.to_string()),
                Cell::from({
                    let signals = tg.review_signals();
                    if signals.is_empty() {
                        "-".into()
                    } else {
                        signals.join(",")
                    }
                }),
            ])
            .style(style)
        })
        .collect::<Vec<_>>();

    let table = Table::new(
        rows,
        [
            ratatui::layout::Constraint::Percentage(24),
            ratatui::layout::Constraint::Length(10),
            ratatui::layout::Constraint::Length(10),
            ratatui::layout::Constraint::Length(6),
            ratatui::layout::Constraint::Length(8),
            ratatui::layout::Constraint::Length(8),
            ratatui::layout::Constraint::Length(8),
            ratatui::layout::Constraint::Length(10),
            ratatui::layout::Constraint::Length(16),
        ],
    )
    .header(
        Row::new([
            "NAME",
            "PROTO",
            "TYPE",
            "PORT",
            "LBS",
            "TARGETS",
            "HEALTHY",
            "UNHEALTHY",
            "SIGNALS",
        ])
        .style(
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .title("Target Groups")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.primary)),
    );

    frame.render_widget(table, area);
}
