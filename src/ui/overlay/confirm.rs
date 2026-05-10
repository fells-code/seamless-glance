use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::ui::{centered_rect, overlay::overlays::ConfirmCommandState, theme::Theme};

pub fn render_confirm_command_overlay(
    frame: &mut Frame,
    area: Rect,
    state: &ConfirmCommandState,
    theme: &Theme,
) {
    frame.render_widget(ratatui::widgets::Clear, area);

    let popup_area = centered_rect(70, 40, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(popup_area);

    let title = Paragraph::new(state.title.clone())
        .style(Style::default().fg(theme.text).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.primary)),
        );

    let content_lines = state.command.lines().count();
    let body_height = chunks[1].height.saturating_sub(2) as usize;
    let max_scroll = content_lines.saturating_sub(body_height);
    let clamped_scroll = state.scroll.min(max_scroll as u16);

    let command = Paragraph::new(state.command.clone())
        .style(Style::default().fg(theme.accent))
        .wrap(Wrap { trim: false })
        .scroll((clamped_scroll, 0))
        .block(Block::default().title("Command").borders(Borders::ALL));

    let footer = Paragraph::new("Enter to run • Esc to cancel • ↑ / ↓ or PgUp / PgDn scroll")
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.text));

    frame.render_widget(title, chunks[0]);
    frame.render_widget(command, chunks[1]);
    frame.render_widget(footer, chunks[2]);
}
