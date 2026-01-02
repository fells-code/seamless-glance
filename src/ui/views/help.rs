use ratatui::{
    layout::Alignment,
    style::Style,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::{app::App, ui::centered_rect};

pub fn render(frame: &mut Frame, app: &App) {
    let area = centered_rect(70, 70, frame.size());

    frame.render_widget(Clear, area); // clear beneath

    let text = r#"
Seamless Glance — Help

Navigation
  1            Account Overview
  2            Cost Overview
  3            ECS
  4            Lambda
  5            ApiGateway
  6            SQS

Regions
  ← / →        Switch region

Commands
  /            Open command palette
  ecs          Go to ECS
  ec2          Go to EC2
  lambda       Go to Lambda
  apigw        Go to ApiGateway
  rds          Go to RDS
  sqs          Go to SQS
  cost         Go to Cost

General
  q            Quit
  Esc          Close overlays
"#;

    let block = Paragraph::new(text)
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
