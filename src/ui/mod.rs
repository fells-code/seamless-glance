pub mod footer;
pub mod header;
pub mod open;
pub mod overlay;
pub mod terminal;
pub mod theme;
pub mod views;

use crate::app::ActiveView;
use crate::app::App;
use crate::ui::footer::draw_footer;
use crate::ui::header::render_header;
use crate::ui::overlay::confirm::render_confirm_command_overlay;
use crate::ui::overlay::help;
use crate::ui::overlay::overlays::OverlayState;
use crate::ui::overlay::render::render_describe_overlay;
use crate::ui::overlay::select_profile::render_select_profile_overlay;
use crate::ui::overlay::select_ssh_key::render_select_ssh_key_overlay;
use crate::ui::views::account_overview;
use crate::ui::views::apigateway::render_apigatway;
use crate::ui::views::cloudwatch::render_cw;
use crate::ui::views::command::command_for_view;
use crate::ui::views::cost_overview::render_cost_overview;
use crate::ui::views::cost_savings::render as render_cost_savings;
use crate::ui::views::ec2::render_ec2;
use crate::ui::views::ecs::render_ecs_clusters;
use crate::ui::views::findings::render as render_findings;
use crate::ui::views::lambda::render;
use crate::ui::views::load_balancers::render_lbs;
use crate::ui::views::rds::render_rds;
use crate::ui::views::secrets::render_sm;
use crate::ui::views::security_groups::render_sg;
use crate::ui::views::sqs::render_sqs;
use crate::ui::views::target_groups::render_tg;
use crate::ui::views::vpc::render_vpc;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Min(1),
            Constraint::Length(4),
        ])
        .split(frame.size());

    let header_area = chunks[0];
    let main_area = chunks[1];
    let footer_area = chunks[2];

    render_header(frame, header_area, app);

    match app.active_view {
        ActiveView::Findings => {
            render_findings(frame, main_area, app);
        }
        ActiveView::AccountOverview => {
            account_overview::render(frame, main_area, app);
        }
        ActiveView::Ecs => {
            render_ecs_clusters(frame, main_area, app);
        }
        ActiveView::CostOverview => {
            render_cost_overview(frame, main_area, app);
        }
        ActiveView::CostSavings => {
            render_cost_savings(frame, main_area, app);
        }
        ActiveView::Lambda => {
            render(frame, main_area, app);
        }
        ActiveView::Apigateway => {
            render_apigatway(frame, main_area, app);
        }
        ActiveView::Sqs => {
            render_sqs(frame, main_area, app);
        }
        ActiveView::Vpc => {
            render_vpc(frame, main_area, app);
        }
        ActiveView::Ec2 => {
            render_ec2(frame, main_area, app);
        }
        ActiveView::CloudWatch => {
            render_cw(frame, main_area, app);
        }
        ActiveView::Secrets => {
            render_sm(frame, main_area, app);
        }
        ActiveView::Rds => {
            render_rds(frame, main_area, app);
        }
        ActiveView::LoadBalancers => {
            render_lbs(frame, main_area, app);
        }
        ActiveView::TargetGroups => {
            render_tg(frame, main_area, app);
        }
        ActiveView::SecurityGroups => {
            render_sg(frame, main_area, app);
        }
    }

    if app.is_refreshing {
        render_loading_overlay(frame, main_area, app);
    }

    if app.show_help {
        help::render(frame, app);
    }

    if let Some(overlay) = &app.overlay {
        match overlay {
            OverlayState::Describe(state) => {
                render_describe_overlay(frame, frame.size(), state, &app.theme);
            }
            OverlayState::ConfirmCommand(state) => {
                render_confirm_command_overlay(frame, frame.size(), state, &app.theme);
            }
            OverlayState::SelectSshKey(state) => {
                render_select_ssh_key_overlay(frame, frame.size(), state, &app.theme)
            }
            OverlayState::SelectProfile(state) => {
                render_select_profile_overlay(frame, frame.size(), state, &app.theme)
            }
        }
    }

    draw_footer(frame, footer_area, app);
}

fn render_loading_overlay(frame: &mut Frame, area: Rect, app: &App) {
    let popup = centered_rect(46, 22, area);
    let view_label = command_for_view(app.active_view)
        .map(|command| command.description)
        .unwrap_or("Current view");

    let detail = match &app.refresh_phase {
        crate::app::RefreshPhase::Idle | crate::app::RefreshPhase::Overview => {
            format!("Refreshing account overview and {view_label}…")
        }
        crate::app::RefreshPhase::Services(services) => {
            if services.is_empty() {
                format!("Refreshing {view_label}…")
            } else {
                format!("Loading {}…", services.join(", "))
            }
        }
    };

    let text = format!("Loading {view_label}\n\n{detail}");
    let block = Paragraph::new(text)
        .style(Style::default().fg(app.theme.text))
        .block(
            Block::default()
                .title("Loading")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.primary)),
        );

    frame.render_widget(Clear, popup);
    frame.render_widget(block, popup);
}
