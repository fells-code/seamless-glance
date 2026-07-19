use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::ui::{centered_rect, overlay::overlays::SelectProfileState, theme::Theme};

pub fn render_select_profile_overlay(
    frame: &mut Frame,
    area: Rect,
    state: &SelectProfileState,
    theme: &Theme,
) {
    let popup = centered_rect(50, 60, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title("Switch AWS Profile")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.primary));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    if state.profiles.is_empty() {
        let message = Paragraph::new(
            "No profiles found in ~/.aws/config or ~/.aws/credentials.\n\nAdd a profile there, then reopen this picker.",
        )
        .style(Style::default().fg(theme.text));
        frame.render_widget(message, chunks[0]);
    } else {
        let items: Vec<ListItem> = state
            .profiles
            .iter()
            .map(|name| ListItem::new(name.as_str()))
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(state.selected.min(state.profiles.len() - 1)));

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .fg(theme.background)
                    .bg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, chunks[0], &mut list_state);
    }

    let hint = Paragraph::new("↑ / ↓ Move   Enter Switch   Esc Cancel")
        .style(Style::default().fg(theme.accent));
    frame.render_widget(hint, chunks[1]);
}
