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
    let mut lines = vec![
        "Seamless Glance — Help".into(),
        "".into(),
        "Global Navigation".into(),
        "  f                    Findings home".into(),
        "  /                    Open command palette".into(),
        "  Tab / Shift+Tab      Cycle through major views".into(),
        "  t                    Cycle through Seamless themes".into(),
        "  ?                    Open help".into(),
        "  r                    Refresh active view".into(),
        "  q                    Quit".into(),
        "".into(),
        "Movement And Regions".into(),
        "  ↑ / ↓                Move selection or scroll overlays".into(),
        "  PgUp / PgDn          Jump-scroll lists, overlays, and help".into(),
        "  Home / End           Jump to top or bottom".into(),
        "  ← / →                Change AWS region".into(),
        "  g                    Jump to the synthetic global slot".into(),
        "".into(),
        "Resource Actions".into(),
        "  Enter                Open related service from Findings".into(),
        "  d                    Describe selected resource".into(),
        "  v                    Toggle Describe between structured and JSON".into(),
        "  c                    Show AWS CLI command for selected resource".into(),
        "  o                    Open selected resource in the AWS console".into(),
        "  s                    Prepare an SSH command for the selected EC2 instance".into(),
        "  Esc                  Close overlays or exit help / command palette".into(),
        "".into(),
        "Command Palette".into(),
        "  findings             Jump to findings".into(),
        "  theme <name>         Switch to a named Seamless theme".into(),
        "  region <name>        Jump to a specific AWS region".into(),
        "  region global        Jump to the synthetic global slot".into(),
        "  rg <name>            Short alias for region".into(),
        "  themes               autumn, winter, summer, spring, developer".into(),
    ];

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
