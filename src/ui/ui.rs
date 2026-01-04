use crate::app::ActiveView;
use crate::app::App;
use crate::ui::footer::draw_footer;
use crate::ui::header::render_header;
use crate::ui::views::apigateway::render_apigatway;
use crate::ui::views::cloudwatch::render_cw;
use crate::ui::views::cost_overview::render_cost_overview;
use crate::ui::views::ec2::render_ec2;
use crate::ui::views::ecs::render_ecs_clusters;
use crate::ui::views::lambda::render;
use crate::ui::views::rds::render_rds;
use crate::ui::views::secrets::render_sm;
use crate::ui::views::sqs::render_sqs;
use crate::ui::views::vpc::render_vpc;
use crate::ui::views::{account_overview, help};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
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
            Constraint::Length(7),
            Constraint::Min(1),
            Constraint::Length(4),
        ])
        .split(frame.size());

    let header_area = chunks[0];
    let main_area = chunks[1];
    let footer_area = chunks[2];

    render_header(frame, header_area, app);

    match app.active_view {
        ActiveView::AccountOverview => {
            account_overview::render(frame, main_area, app);
        }
        ActiveView::Ecs => {
            render_ecs_clusters(frame, main_area, app);
        }
        ActiveView::CostOverview => {
            render_cost_overview(frame, main_area, app);
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
        _ => {}
    }

    if app.show_help {
        help::render(frame, app);
    }

    draw_footer(frame, footer_area, app);
}
