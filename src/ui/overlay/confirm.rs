use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph},
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

    let command = Paragraph::new(state.command.clone())
        .style(Style::default().fg(theme.accent))
        .block(Block::default().title("Command").borders(Borders::ALL));

    let footer = Paragraph::new("Enter to run • Esc to cancel")
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.text));

    frame.render_widget(title, chunks[0]);
    frame.render_widget(command, chunks[1]);
    frame.render_widget(footer, chunks[2]);
}
