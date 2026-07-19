use crate::models::service_status::ServiceStatus;
use crate::ui::theme::Theme;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// When `status` is not `Ok`, render a titled message describing the denied or
/// unavailable state into `area` and return `true` so the caller returns early
/// instead of drawing an empty table that looks like "no resources". Returns
/// `false` when the service is reachable and the caller should render normally.
pub fn render_unavailable(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    status: &ServiceStatus,
    theme: &Theme,
) -> bool {
    let message = match status {
        ServiceStatus::Ok => return false,
        ServiceStatus::AccessDenied => format!("Access denied to {title}"),
        ServiceStatus::Unavailable(msg) if msg.is_empty() => format!("{title} unavailable"),
        ServiceStatus::Unavailable(msg) => format!("{title} unavailable: {msg}"),
    };

    let paragraph = Paragraph::new(message)
        .style(Style::default().fg(theme.accent))
        .block(
            Block::default()
                .title(title.to_string())
                .borders(Borders::ALL),
        );
    frame.render_widget(paragraph, area);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn render_to_string(status: &ServiceStatus, title: &str) -> (bool, String) {
        let mut terminal = Terminal::new(TestBackend::new(60, 6)).unwrap();
        let theme = Theme::autumn();
        let mut rendered = false;

        terminal
            .draw(|frame| {
                rendered = render_unavailable(frame, frame.size(), title, status, &theme);
            })
            .unwrap();

        let text = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();

        (rendered, text)
    }

    #[test]
    fn ok_status_does_not_render_and_returns_false() {
        let (rendered, text) = render_to_string(&ServiceStatus::Ok, "Lambda");
        assert!(!rendered);
        assert!(!text.contains("Lambda"));
    }

    #[test]
    fn access_denied_renders_message_and_returns_true() {
        let (rendered, text) = render_to_string(&ServiceStatus::AccessDenied, "Lambda");
        assert!(rendered);
        assert!(text.contains("Access denied to Lambda"));
    }

    #[test]
    fn unavailable_surfaces_the_inner_message() {
        let status = ServiceStatus::Unavailable("throttled".into());
        let (rendered, text) = render_to_string(&status, "SQS");
        assert!(rendered);
        assert!(text.contains("SQS unavailable: throttled"));
    }
}
