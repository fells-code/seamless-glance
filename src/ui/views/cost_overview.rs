use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{BarChart, Block, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::{app::App, aws::cost::last_6_month_labels};

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

fn render_service_cost_chart(frame: &mut Frame, area: Rect, app: &mut App) {
    let total: f64 = app
        .service_cost_insights
        .iter()
        .map(|insight| insight.monthly_cost)
        .sum();

    let mut sorted = app.service_cost_insights.clone();
    sorted.sort_by(|a, b| {
        b.monthly_cost
            .partial_cmp(&a.monthly_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let visible_height = area.height.saturating_sub(3) as usize;
    let max_scroll = sorted.len().saturating_sub(visible_height);
    app.scroll_offset = app.scroll_offset.min(max_scroll as u16);

    let rows = sorted
        .iter()
        .skip(app.scroll_offset as usize)
        .take(visible_height)
        .map(|insight| {
            let pct = if total > 0.0 {
                insight.monthly_cost / total
            } else {
                0.0
            };

            Row::new(vec![
                Cell::from(insight.service.clone()),
                Cell::from(format!("${:.2}", insight.monthly_cost)),
                Cell::from(format!("{:.1}%", pct * 100.0)),
                Cell::from(insight.primary_usage_summary()),
            ])
        })
        .collect::<Vec<_>>();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(32),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Percentage(58),
        ],
    )
    .header(
        Row::new(["SERVICE", "COST", "SHARE", "TOP USAGE"]).style(
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .title("Service Cost + Usage")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.primary)),
    )
    .style(Style::default().fg(app.theme.text));

    frame.render_widget(table, area);
}

pub fn render_cost_overview(frame: &mut Frame, area: Rect, app: &mut App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(0)])
        .split(area);

    let forecast_range = match (app.budget.forecast_low, app.budget.forecast_high) {
        (Some(low), Some(high)) => format!("Forecast range ${low:.2} - ${high:.2}"),
        _ => "Forecast range unavailable".into(),
    };
    let budget_gap = app.budget.forecast - app.budget.monthly_budget;
    let summary = Paragraph::new(format!(
        "Month-to-date ${:.2}  |  Forecast ${:.2}  |  Budget ${:.2}  |  Gap {:+.2}\n{}",
        app.budget.month_to_date_cost,
        app.budget.forecast,
        app.budget.monthly_budget,
        budget_gap,
        forecast_range
    ))
    .wrap(Wrap { trim: true })
    .style(Style::default().fg(app.theme.text))
    .block(
        Block::default()
            .title("Cost Summary")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.primary)),
    );

    frame.render_widget(summary, layout[0]);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(layout[1]);

    render_cost_6mo_chart(frame, chunks[0], app);
    render_service_cost_chart(frame, chunks[1], app);
}
