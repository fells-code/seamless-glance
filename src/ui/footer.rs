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
}

pub fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    if app.command_mode {
        draw_command_palette(frame, area, app);
        return;
    }

    let status = if app.is_refreshing {
        "Refreshing…"
    } else {
        "Live"
    };

    let refresh = app
        .last_refresh
        .map(|t| t.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| "—".into());

    let footer_text = match app.footer_mode {
        FooterMode::Normal => {
            "[q] Quit   [1] Overview   [2] Cost   [3] ECS   [4] Lambda  [5] ApiGateway  [6] SQS   [← →] Region   [/] Command   [?] Help"
        }
        FooterMode::Command => "Command mode — type and press Enter (Esc to cancel)",
        FooterMode::Help => "Help — Esc to close",
    };

    // let footer_text = format!(
    //     "[q] Quit   [1] Overview   [2] Cost   [3] ECS   [← →] Region   [/] Command   [?] Help  Status: {}   Last refresh: {}",
    //     status, refresh
    // );

    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(app.theme.text))
        .block(Block::default().borders(Borders::TOP));

    frame.render_widget(footer, area);
}
