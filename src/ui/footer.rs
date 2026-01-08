use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{app::App, ui::views::command::draw_command_palette};

pub enum FooterMode {
    Normal,
    Command,
    Help,
    Overlay,
}

pub fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    if app.command_mode {
        draw_command_palette(frame, area, app);
        return;
    }

    let footer_text = if app.command_mode {
        "Command mode — type a view name and press Enter (Esc to cancel)"
    } else if app.describe_overlay.is_some() {
        "Esc Close   ↑ / ↓ Scroll"
    } else if app.show_help {
        "Help — Esc Close   ↑ / ↓ Scroll"
    } else {
        "[d] Describe   [o] Open   [r] Refresh   [/] Navigate to service   [?] Help   [q] Quit"
    };

    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(app.theme.text))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(app.theme.primary)),
        );

    frame.render_widget(footer, area);
}
