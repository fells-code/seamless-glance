use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::overlay::overlays::DescribeOverlayState;
use crate::ui::theme::Theme;

pub fn render_describe_overlay(
    frame: &mut Frame,
    area: Rect,
    overlay: &DescribeOverlayState,
    theme: &Theme,
) {
    // Clear + backdrop
    frame.render_widget(ratatui::widgets::Clear, area);
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Black)),
        area,
    );

    // Centered modal (80% width / height)
    let popup = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ])
        .split(area)[1];

    let popup = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ])
        .split(popup)[1];

    // Main panel
    let block = Block::default()
        .title(format!(" Describe — {} ", overlay.title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.primary))
        .style(Style::default().bg(theme.background).fg(theme.text));

    let paragraph = Paragraph::new(overlay.content.clone())
        .block(block)
        .alignment(Alignment::Left)
        .scroll((overlay.scroll, 0))
        .style(Style::default().fg(theme.text));

    frame.render_widget(paragraph, popup);
}
