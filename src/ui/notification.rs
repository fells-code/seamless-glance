use crate::ui::theme::Theme;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub enum NotificationLevel {
    Warning,
    Error,
}

/// A transient, non-modal message shown to the operator. Errors that would
/// otherwise be written to a stderr hidden behind the alternate screen are
/// surfaced here instead.
pub struct Notification {
    pub message: String,
    pub level: NotificationLevel,
    expires_at: Instant,
}

impl Notification {
    const TTL: Duration = Duration::from_secs(6);

    pub fn new(message: impl Into<String>, level: NotificationLevel) -> Self {
        Self {
            message: message.into(),
            level,
            expires_at: Instant::now() + Self::TTL,
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

/// Render the notification as a banner anchored to the bottom of `area` (the
/// main body), just above the footer. Non-modal: it draws over the body but
/// does not capture input.
pub fn render_notification(
    frame: &mut Frame,
    notification: &Notification,
    area: Rect,
    theme: &Theme,
) {
    let (title, color) = match notification.level {
        NotificationLevel::Warning => ("Warning", Color::Yellow),
        NotificationLevel::Error => ("Error", Color::Red),
    };

    let height = 3;
    if area.height < height {
        return;
    }

    let banner = Rect {
        x: area.x,
        y: area.y + area.height - height,
        width: area.width,
        height,
    };

    let paragraph = Paragraph::new(notification.message.as_str())
        .style(Style::default().fg(theme.text))
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(color).add_modifier(Modifier::BOLD)),
        );

    frame.render_widget(Clear, banner);
    frame.render_widget(paragraph, banner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::Theme;
    use ratatui::{backend::TestBackend, Terminal};

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn renders_message_anchored_to_bottom_of_area() {
        let mut terminal = Terminal::new(TestBackend::new(40, 10)).unwrap();
        let notification = Notification::new("Unknown region: mars-1", NotificationLevel::Warning);
        let theme = Theme::autumn();

        terminal
            .draw(|frame| {
                let area = frame.size();
                render_notification(frame, &notification, area, &theme);
            })
            .unwrap();

        assert!(buffer_text(&terminal).contains("Unknown region: mars-1"));
    }

    #[test]
    fn skips_render_when_area_too_short() {
        let mut terminal = Terminal::new(TestBackend::new(40, 10)).unwrap();
        let notification = Notification::new("hidden", NotificationLevel::Error);
        let theme = Theme::autumn();

        terminal
            .draw(|frame| {
                let short = Rect {
                    x: 0,
                    y: 0,
                    width: 40,
                    height: 2,
                };
                render_notification(frame, &notification, short, &theme);
            })
            .unwrap();

        assert!(!buffer_text(&terminal).contains("hidden"));
    }
}
