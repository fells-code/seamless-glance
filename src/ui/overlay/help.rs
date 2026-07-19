use ratatui::{
    layout::Alignment,
    style::Style,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::{
    app::App,
    ui::{
        centered_rect,
        keys::{KeyGroup, CONTEXTUAL_KEYS, KEY_BINDINGS},
        views::command::{CommandGroup, COMMANDS},
    },
};

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = centered_rect(76, 76, frame.size());

    frame.render_widget(Clear, area);

    let text = build_help_text();
    let help_lines = text.lines().count();
    let visible_height = area.height.saturating_sub(2) as usize;

    let max_scroll = help_lines.saturating_sub(visible_height);
    app.scroll_offset = app.scroll_offset.min(max_scroll as u16);

    let block = Paragraph::new(text)
        .scroll((app.scroll_offset, 0))
        .alignment(Alignment::Left)
        .style(Style::default().fg(app.theme.text))
        .block(
            Block::default()
                .title("Help")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.primary)),
        );

    frame.render_widget(block, area);
}

fn build_help_text() -> String {
    let mut lines = vec!["Seamless Glance — Help".to_string(), String::new()];

    // Both of these read the key registry, so help can never advertise a
    // binding the app does not actually run.
    lines.push("Global Navigation".into());
    for binding in KEY_BINDINGS
        .iter()
        .filter(|binding| binding.group == KeyGroup::Navigation)
    {
        lines.push(format!("  {:<20} {}", binding.key, binding.help));
    }

    lines.push(String::new());
    lines.push("Resource Actions".into());
    for binding in KEY_BINDINGS
        .iter()
        .filter(|binding| binding.group == KeyGroup::ResourceAction)
    {
        lines.push(format!("  {:<20} {}", binding.key, binding.help));
    }

    lines.push(String::new());
    lines.push("Movement And Overlays".into());
    for (key, description) in CONTEXTUAL_KEYS {
        lines.push(format!("  {key:<20} {description}"));
    }

    lines.push(String::new());
    lines.push("Command Palette".into());
    for (command, description) in [
        ("theme <name>", "Switch to a named Seamless theme"),
        ("region <name>", "Jump to a specific AWS region"),
        ("region global", "Jump to the synthetic global slot"),
        ("rg <name>", "Short alias for region"),
        ("profile <name>", "Switch to a named AWS profile"),
        ("profile", "Open the AWS profile picker"),
        ("themes", "autumn, winter, summer, spring, developer"),
    ] {
        lines.push(format!("  {command:<20} {description}"));
    }

    for group in [
        CommandGroup::Triage,
        CommandGroup::Overview,
        CommandGroup::Compute,
        CommandGroup::Data,
        CommandGroup::Messaging,
        CommandGroup::Networking,
        CommandGroup::Security,
        CommandGroup::Observability,
    ] {
        let commands = COMMANDS
            .iter()
            .filter(|command| command.group == group)
            .collect::<Vec<_>>();

        if commands.is_empty() {
            continue;
        }

        lines.push("".into());
        lines.push(group.label().to_string());

        for command in commands {
            let shortcut = command
                .shortcut
                .map(|shortcut| shortcut.to_string())
                .unwrap_or_else(|| "—".into());

            lines.push(format!(
                "  {:<18} [{}] {}",
                command.name, shortcut, command.description
            ));
        }
    }

    lines.join("\n")
}
