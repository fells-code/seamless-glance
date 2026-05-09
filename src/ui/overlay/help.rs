use ratatui::{
    layout::Alignment,
    style::Style,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::{app::App, ui::centered_rect};

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = centered_rect(70, 70, frame.size());

    frame.render_widget(Clear, area); // clear beneath

    let text = r#"
Seamless Glance — Help

Navigation
  1            Account Overview
  2            Cost Overview
  3            VPC
  4            EC2
  5            CloudWatch
  6            Lambda
  7            Secrets Manager
  8            ECS
  9            ApiGateway

Regions
  ← / →        Switch region
  ↑ / ↓        Move up and down

Commands
  /                   Open command palette
  region <region>     Change to region
  rg <region>         Change to region
  ecs                 Go to ECS
  ec2                 Go to EC2
  lambda              Go to Lambda
  apigw               Go to ApiGateway
  rds                 Go to RDS
  sqs                 Go to SQS
  cost                Go to Cost
  sm                  Go to Secrets Manager
  vpc                 Go to VPC
  cw                  Go to CloudWatch
  sm                  Go to Secrets Manager
  tg                  Go to Target Groups
  sg                  Go to Security Groups
  

General
  q            Quit
  r            Refresh current view
  d            Describe resource
  c            Show AWS CLI command
  g            Switch to a Global view of resource
  o            Open in console
  s            Shell into instance
  Esc          Close overlays
"#;

    let help_lines = text.lines().count();
    let visible_height = area.height.saturating_sub(2) as usize; // borders

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
