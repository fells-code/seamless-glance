use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{
    app::{ActiveView, App},
    ui::views::command::{command_for_view, draw_command_palette},
};

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

    let view_hint = command_for_view(app.active_view)
        .map(|command| format!("View: {}", command.name))
        .unwrap_or_else(|| "View: unknown".into());

    let footer_text = if app.command_mode {
        "Command palette — type a view name or alias and press Enter (Esc to cancel)".to_string()
    } else if app.overlay.is_some() {
        "Esc Close   ↑ / ↓ Scroll".to_string()
    } else if app.show_help {
        "Help — Esc Close   ↑ / ↓ Scroll".to_string()
    } else if app.active_view == ActiveView::Findings {
        "[Enter] Open related view   [Tab] Next view   [/] Jump   [r] Refresh   [?] Help   [q] Quit"
            .to_string()
    } else if app.active_view == ActiveView::Ec2 {
        format!(
            "{}   [Tab/Shift+Tab] Cycle views   [/] Jump   [d] Describe   [c] CLI   [o] Console   [s] SSH   [g] Global   [r] Refresh   [?] Help   [q] Quit",
            view_hint
        )
    } else {
        format!(
            "{}   [Tab/Shift+Tab] Cycle views   [/] Jump   [d] Describe   [c] CLI   [o] Console   [g] Global   [r] Refresh   [?] Help   [q] Quit",
            view_hint
        )
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
