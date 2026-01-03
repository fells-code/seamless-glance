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
];

pub fn draw_command_palette(frame: &mut Frame, area: Rect, app: &App) {
    if !app.command_mode {
        return;
    }

    let input = &app.command_input;

    let matches: Vec<_> = COMMANDS
        .iter()
        .filter(|c| c.name.starts_with(input))
        .collect();

    let mut lines = vec![format!(":{}", input)];

    for cmd in matches.iter().take(5) {
        lines.push(format!("  {:<6} — {}", cmd.name, cmd.description));
    }

    let cmd_ui = Paragraph::new(lines.join("\n"))
        .style(Style::default().fg(app.theme.accent))
        .block(Block::default().borders(Borders::TOP).title("Command"));

    frame.render_widget(cmd_ui, area);
}
