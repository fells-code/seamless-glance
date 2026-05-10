use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{ActiveView, App};
use crate::ui::theme::ThemeName;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommandGroup {
    Triage,
    Overview,
    Compute,
    Data,
    Messaging,
    Networking,
    Security,
    Observability,
}

impl CommandGroup {
    pub fn label(&self) -> &'static str {
        match self {
            CommandGroup::Triage => "Triage",
            CommandGroup::Overview => "Overview",
            CommandGroup::Compute => "Compute",
            CommandGroup::Data => "Data",
            CommandGroup::Messaging => "Messaging",
            CommandGroup::Networking => "Networking",
            CommandGroup::Security => "Security",
            CommandGroup::Observability => "Observability",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Command {
    pub name: &'static str,
    pub description: &'static str,
    pub view: ActiveView,
    pub group: CommandGroup,
    pub shortcut: Option<char>,
    pub aliases: &'static [&'static str],
}

pub const COMMANDS: &[Command] = &[
    Command {
        name: "findings",
        description: "Priority findings and triage inbox",
        view: ActiveView::Findings,
        group: CommandGroup::Triage,
        shortcut: Some('f'),
        aliases: &["finding", "triage", "home"],
    },
    Command {
        name: "account",
        description: "Account overview",
        view: ActiveView::AccountOverview,
        group: CommandGroup::Overview,
        shortcut: Some('1'),
        aliases: &["overview", "account-overview", "summary"],
    },
    Command {
        name: "cost",
        description: "Cost overview",
        view: ActiveView::CostOverview,
        group: CommandGroup::Overview,
        shortcut: Some('2'),
        aliases: &["billing", "spend"],
    },
    Command {
        name: "ec2",
        description: "EC2 instances",
        view: ActiveView::Ec2,
        group: CommandGroup::Compute,
        shortcut: Some('4'),
        aliases: &["instances", "compute"],
    },
    Command {
        name: "lambda",
        description: "Lambda functions",
        view: ActiveView::Lambda,
        group: CommandGroup::Compute,
        shortcut: Some('6'),
        aliases: &["functions"],
    },
    Command {
        name: "ecs",
        description: "ECS clusters",
        view: ActiveView::Ecs,
        group: CommandGroup::Compute,
        shortcut: Some('8'),
        aliases: &["containers", "cluster"],
    },
    Command {
        name: "rds",
        description: "RDS instances",
        view: ActiveView::Rds,
        group: CommandGroup::Data,
        shortcut: None,
        aliases: &["database", "databases"],
    },
    Command {
        name: "sqs",
        description: "SQS queues",
        view: ActiveView::Sqs,
        group: CommandGroup::Messaging,
        shortcut: None,
        aliases: &["queue", "queues", "messaging"],
    },
    Command {
        name: "vpc",
        description: "VPCs",
        view: ActiveView::Vpc,
        group: CommandGroup::Networking,
        shortcut: Some('3'),
        aliases: &["network", "networks"],
    },
    Command {
        name: "apigw",
        description: "API Gateway APIs",
        view: ActiveView::Apigateway,
        group: CommandGroup::Networking,
        shortcut: Some('9'),
        aliases: &["api", "apigateway", "gateway"],
    },
    Command {
        name: "lb",
        description: "Load balancers",
        view: ActiveView::LoadBalancers,
        group: CommandGroup::Networking,
        shortcut: None,
        aliases: &["elbv2", "load-balancer", "loadbalancer"],
    },
    Command {
        name: "tg",
        description: "Target groups",
        view: ActiveView::TargetGroups,
        group: CommandGroup::Networking,
        shortcut: None,
        aliases: &["target-group", "targetgroups"],
    },
    Command {
        name: "sg",
        description: "Security groups",
        view: ActiveView::SecurityGroups,
        group: CommandGroup::Security,
        shortcut: None,
        aliases: &["security-group", "securitygroups"],
    },
    Command {
        name: "sm",
        description: "Secrets Manager",
        view: ActiveView::Secrets,
        group: CommandGroup::Security,
        shortcut: Some('7'),
        aliases: &["secrets", "secret", "secretsmanager"],
    },
    Command {
        name: "cw",
        description: "CloudWatch alarms",
        view: ActiveView::CloudWatch,
        group: CommandGroup::Observability,
        shortcut: Some('5'),
        aliases: &["cloudwatch", "alarms", "monitoring"],
    },
];

pub fn parse_command(input: &str) -> (&str, &str) {
    let trimmed = input.trim().trim_start_matches('/');

    if let Some((cmd, rest)) = trimmed.split_once(char::is_whitespace) {
        (cmd.trim(), rest.trim())
    } else {
        (trimmed, "")
    }
}

pub fn command_for_view(view: ActiveView) -> Option<&'static Command> {
    COMMANDS.iter().find(|command| command.view == view)
}

pub fn next_command(current: ActiveView) -> &'static Command {
    let index = COMMANDS
        .iter()
        .position(|command| command.view == current)
        .unwrap_or(0);

    &COMMANDS[(index + 1) % COMMANDS.len()]
}

pub fn previous_command(current: ActiveView) -> &'static Command {
    let index = COMMANDS
        .iter()
        .position(|command| command.view == current)
        .unwrap_or(0);

    &COMMANDS[(index + COMMANDS.len() - 1) % COMMANDS.len()]
}

pub fn matching_commands(query: &str) -> Vec<&'static Command> {
    let normalized = query.trim().trim_start_matches('/').to_ascii_lowercase();

    let mut matches = COMMANDS
        .iter()
        .filter_map(|command| match_score(command, &normalized).map(|score| (score, command)))
        .collect::<Vec<_>>();

    matches.sort_by(|(left_score, left_command), (right_score, right_command)| {
        left_score
            .cmp(right_score)
            .then_with(|| left_command.group.cmp(&right_command.group))
            .then_with(|| left_command.name.cmp(right_command.name))
    });

    matches.into_iter().map(|(_, command)| command).collect()
}

fn match_score(command: &Command, query: &str) -> Option<u8> {
    if query.is_empty() {
        return Some(0);
    }

    let description = command.description.to_ascii_lowercase();

    if command.name == query {
        return Some(0);
    }

    if command.aliases.contains(&query) {
        return Some(1);
    }

    if command.name.starts_with(query) {
        return Some(2);
    }

    if command.aliases.iter().any(|alias| alias.starts_with(query)) {
        return Some(3);
    }

    if command.name.contains(query) {
        return Some(4);
    }

    if command.aliases.iter().any(|alias| alias.contains(query)) {
        return Some(5);
    }

    if description.contains(query) {
        return Some(6);
    }

    None
}

pub fn draw_command_palette(frame: &mut Frame, area: Rect, app: &App) {
    if !app.command_mode {
        return;
    }

    let input = app.command_input.trim();
    let (cmd, _args) = parse_command(input);
    let matches = matching_commands(cmd);

    let mut lines = vec![format!(":{}", input)];
    lines.push("  Jump to a view, or use `region <name>` / `rg <name>`.".into());

    if "region".starts_with(cmd) || "rg".starts_with(cmd) || cmd == "region" || cmd == "rg" {
        lines.push("".into());
        lines.push("  Regions".into());
        lines.push("  region <name>    Jump to a specific AWS region".into());
        lines.push("  region global    Jump to the synthetic global slot".into());
        lines.push("  rg <name>        Short alias for region".into());
    }

    if "theme".starts_with(cmd)
        || "themes".starts_with(cmd)
        || cmd == "theme"
        || cmd == "themes"
        || "th".starts_with(cmd)
    {
        lines.push("".into());
        lines.push("  Themes".into());
        lines.push("  theme <name>     Switch to a Seamless theme".into());

        for theme_name in ThemeName::ALL {
            lines.push(format!(
                "  theme {:<9} {}",
                theme_name.as_str(),
                theme_name.description()
            ));
        }
    }

    let mut last_group = None;
    for command in matches.iter().take(8) {
        if last_group != Some(command.group) {
            lines.push("".into());
            lines.push(format!("  {}", command.group.label()));
            last_group = Some(command.group);
        }

        let marker = if app.active_view == command.view {
            ">"
        } else {
            " "
        };
        let shortcut = command
            .shortcut
            .map(|shortcut| format!("[{}]", shortcut))
            .unwrap_or_else(|| "   ".into());

        lines.push(format!(
            "  {} {:<14} {:<4} {}",
            marker, command.name, shortcut, command.description
        ));
    }

    let is_special_query = cmd == "region"
        || cmd == "rg"
        || cmd == "theme"
        || cmd == "themes"
        || "th".starts_with(cmd);

    if matches.is_empty() && !cmd.is_empty() && !is_special_query {
        lines.push("".into());
        lines.push("  No matching view commands".into());
    }

    let cmd_ui = Paragraph::new(lines.join("\n"))
        .style(Style::default().fg(app.theme.accent))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .title("Command Palette"),
        );

    frame.render_widget(cmd_ui, area);
}
