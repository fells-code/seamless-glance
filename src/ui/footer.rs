use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{
    app::{ActiveView, App},
    ui::{
        keys::{self, KeyAction},
        overlay::overlays::OverlayState,
        views::command::{command_for_view, draw_command_palette},
    },
};

pub enum FooterMode {
    Normal,
    Command,
    Help,
    Overlay,
}

/// Hints for a normal view, built from the key registry so the footer can only
/// advertise keys that are actually bound and meaningful here.
fn normal_view_hints(app: &App) -> String {
    let view_hint = command_for_view(app.active_view)
        .map(|command| format!("View: {}", command.name))
        .unwrap_or_else(|| "View: unknown".into());

    let opens_related = matches!(
        app.active_view,
        ActiveView::Findings | ActiveView::CostSavings
    );

    let mut hints = vec![view_hint];

    if opens_related {
        hints.push("[Enter] Open related view".into());
    }

    hints.push("[Tab/Shift+Tab] Cycle views".into());

    for binding in keys::bindings_for_view(app.active_view, app.active_view_supports_wrap()) {
        // The wrap key flips meaning once wrapped detail is on.
        let label = if binding.action == KeyAction::ToggleWrap && app.wrap_mode_active() {
            "Compact view"
        } else {
            binding.label
        };

        hints.push(format!("[{}] {}", binding.key, label));
    }

    hints.join("   ")
}

pub fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    if app.command_mode {
        draw_command_palette(frame, area, app);
        return;
    }

    let footer_text = if let Some(overlay) = &app.overlay {
        match overlay {
            OverlayState::Describe(_) => {
                "Esc Close   [v] Toggle structured / JSON   ↑ / ↓ Scroll   PgUp / PgDn Jump   Home / End Top-Bottom".to_string()
            }
            OverlayState::ConfirmCommand(_) => {
                "Esc Close   Enter Run   ↑ / ↓ Scroll   PgUp / PgDn Jump   Home / End Top-Bottom".to_string()
            }
            OverlayState::SelectSshKey(_) => {
                "Esc Close   [1] Agent   [2] Private key path".to_string()
            }
            OverlayState::SelectProfile(_) => {
                "Esc Cancel   Enter Switch profile   ↑ / ↓ Move".to_string()
            }
        }
    } else if app.show_help {
        "Help — Esc Close   ↑ / ↓ Scroll   PgUp / PgDn Jump   Home / End Top-Bottom".to_string()
    } else {
        normal_view_hints(app)
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
