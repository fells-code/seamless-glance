use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
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
    frame.render_widget(ratatui::widgets::Clear, area);
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Black)),
        area,
    );

    let popup = crate::ui::centered_rect(88, 84, area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(6),
            Constraint::Length(2),
        ])
        .split(popup);

    let title = Paragraph::new(overlay.title.clone())
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.text).add_modifier(Modifier::BOLD))
        .block(
            Block::default()
                .title("Describe")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.primary))
                .style(Style::default().bg(theme.background)),
        );

    let mode_text = format!(
        "Mode: {}  |  [v] Toggle structured / JSON  |  ↑ / ↓ Scroll  |  PgUp / PgDn Jump",
        overlay.mode_label()
    );
    let mode_bar = Paragraph::new(mode_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.accent))
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(theme.primary))
                .style(Style::default().bg(theme.background)),
        );

    let active_content = overlay.active_content();
    let content_lines = active_content.lines().count();
    let body_height = chunks[2].height.saturating_sub(2) as usize;
    let max_scroll = content_lines.saturating_sub(body_height);
    let clamped_scroll = overlay.scroll.min(max_scroll as u16);

    let body = Paragraph::new(active_content.to_string())
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false })
        .scroll((clamped_scroll, 0))
        .style(Style::default().fg(theme.text))
        .block(
            Block::default()
                .title(format!("{} View", overlay.mode_label()))
                .borders(Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(theme.primary))
                .style(Style::default().bg(theme.background)),
        );

    let footer = Paragraph::new("Home / End top-bottom  |  Esc close")
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.text))
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                .border_style(Style::default().fg(theme.primary))
                .style(Style::default().bg(theme.background)),
        );

    frame.render_widget(title, chunks[0]);
    frame.render_widget(mode_bar, chunks[1]);
    frame.render_widget(body, chunks[2]);
    frame.render_widget(footer, chunks[3]);
}
