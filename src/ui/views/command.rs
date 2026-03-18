use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{ActiveView, App};

pub struct Command {
    pub name: &'static str,
    pub description: &'static str,
    pub view: ActiveView,
}

pub const COMMANDS: &[Command] = &[
    Command {
        name: "ecs",
        description: "ECS view",
        view: ActiveView::Ecs,
    },
    Command {
        name: "ec2",
        description: "EC2 view",
        view: ActiveView::Ec2,
    },
    Command {
        name: "rds",
        description: "RDS view",
        view: ActiveView::Rds,
    },
    Command {
        name: "cost",
        description: "Cost overview",
        view: ActiveView::CostOverview,
    },
    Command {
        name: "lambda",
        description: "Lambda view",
        view: ActiveView::Lambda,
    },
    Command {
        name: "apigw",
        description: "API Gateway view",
        view: ActiveView::Apigateway,
    },
    Command {
        name: "sqs",
        description: "SQS queues",
        view: ActiveView::Sqs,
    },
    Command {
        name: "vpc",
        description: "VPCs",
        view: ActiveView::Vpc,
    },
    Command {
        name: "cw",
        description: "Cloudwatch",
        view: ActiveView::CloudWatch,
    },
    Command {
        name: "sm",
        description: "Secrets Manager",
        view: ActiveView::Secrets,
    },
    Command {
        name: "lb",
        description: "Load Balancer",
        view: ActiveView::LoadBalancers,
    },
    Command {
        name: "tg",
        description: "Target Groups",
        view: ActiveView::TargetGroups,
    },
    Command {
        name: "sg",
        description: "Security Groups",
        view: ActiveView::SecurityGroups,
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

pub fn draw_command_palette(frame: &mut Frame, area: Rect, app: &App) {
    if !app.command_mode {
        return;
    }

    let input = app.command_input.trim();
    let (cmd, _args) = parse_command(input);

    let matches: Vec<_> = COMMANDS
        .iter()
        .filter(|c| c.name.starts_with(cmd))
        .collect();

    let mut lines = vec![format!(":{}", input)];

    if "region".starts_with(cmd) || "rg".starts_with(cmd) || cmd == "region" || cmd == "rg" {
        lines.push("  region <name> — jump to a region".into());
        lines.push("  region global — jump to global view".into());
        lines.push("  rg <name>     — short alias for region".into());
    }

    for cmd in matches.iter().take(5) {
        lines.push(format!("  {:<6} — {}", cmd.name, cmd.description));
    }

    let cmd_ui = Paragraph::new(lines.join("\n"))
        .style(Style::default().fg(app.theme.accent))
        .block(Block::default().borders(Borders::TOP).title("Command"));

    frame.render_widget(cmd_ui, area);
}
