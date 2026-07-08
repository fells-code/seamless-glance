use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::{centered_rect, overlay::overlays::SelectSshKeyState, theme::Theme};

pub fn render_select_ssh_key_overlay(
    frame: &mut Frame,
    area: Rect,
    state: &SelectSshKeyState,
    theme: &Theme,
) {
    let popup = centered_rect(60, 30, area);

    let text = format!(
        "{}\n\n\
         Instance expects key: {}\n\n\
         [1] Use SSH agent\n\
         [2] Specify private key path\n\n\
         Esc to cancel",
        state.title,
        state.context.key_name.as_deref().unwrap_or("unknown"),
    );

    let block = Paragraph::new(text)
        .style(Style::default().fg(theme.text))
        .block(
            Block::default()
                .title("SSH Authentication")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.primary)),
        );

    frame.render_widget(block, popup);
}
