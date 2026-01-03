use crate::app::ActiveView;
use crate::ui::footer::draw_footer;
use crate::ui::header::render_header;
use crate::ui::views::apigateway::render_apigatway;
use crate::ui::views::cloudwatch::render_cw;
use crate::ui::views::ec2::render_ec2;
use crate::ui::views::ecs::render_ecs_clusters;
use crate::ui::views::lambda::render;
use crate::ui::views::sqs::render_sqs;
use crate::ui::views::vpc::render_vpc;
use crate::ui::views::{account_overview, help};
use crate::{app::App, aws::cost::last_6_month_labels};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{BarChart, Block, Borders, List, ListItem},
    Frame,
};

fn render_cost_6mo_chart(frame: &mut Frame, area: Rect, app: &App) {
    let labels = last_6_month_labels(); // using the helper above

    let data: Vec<(&str, u64)> = labels
        .iter()
        .zip(app.monthly_costs.iter())
        .map(|(label, val)| (label.as_str(), *val as u64))
        .collect();

    let chart = BarChart::default()
        .block(
            Block::default()
                .title("Last 6 Months Spend")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.primary)),
        )
        .data(&data)
        .bar_width(8)
        .bar_gap(2)
        .bar_set(ratatui::symbols::bar::NINE_LEVELS)
        .style(Style::default().fg(app.theme.primary));

    frame.render_widget(chart, area);
}

fn render_service_cost_chart(frame: &mut Frame, area: Rect, app: &App) {
    let total: f64 = app.service_costs.iter().map(|(_, amt)| amt).sum();

    // Sort descending by cost
    let mut sorted = app.service_costs.clone();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Fixed column widths
    let name_col_width = 39;
    let cost_col_width = 10;
    let pct_col_width = 7;

    // Compute the bar column width based on screen width
    let bar_col_width =
        (area.width as usize).saturating_sub(name_col_width + cost_col_width + pct_col_width + 6);

    let items: Vec<ListItem> = sorted
        .iter()
        .map(|(name, cost)| {
            let pct = if total > 0.0 { cost / total } else { 0.0 };

            // Calculate bar length proportional to usage
            let bar_len = ((pct * bar_col_width as f64).round() as usize).min(bar_col_width);

            let bar = "█".repeat(bar_len);

            // Build a fully aligned line
            let line = format!(
                "{:<name_w$} {:<bar_w$} ${:>7.2} {:>pct_w$}",
                name,
                bar,
                cost,
                format!("({:.1}%)", pct * 100.0),
                name_w = name_col_width,
                bar_w = bar_col_width,
                pct_w = pct_col_width
            );

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title("Service Cost Breakdown")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.primary)),
        )
        .style(Style::default().fg(app.theme.text));

    frame.render_widget(list, area);
}

fn render_overview(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_cost_6mo_chart(frame, chunks[0], app);
    render_service_cost_chart(frame, chunks[1], app);
}

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

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // header (NEW, taller)
            Constraint::Min(1),    // main content
            Constraint::Length(4), // footer
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
            render_overview(frame, main_area, app);
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
        _ => {
            // placeholders
        }
    }

    if app.show_help {
        help::render(frame, app);
    }

    draw_footer(frame, footer_area, app);
}
