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
    let mut sorted = app.service_cost_insights.clone();
    sorted.sort_by(|a, b| {
        b.monthly_cost
            .partial_cmp(&a.monthly_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let total: f64 = sorted.iter().map(|insight| insight.monthly_cost).sum();
    let total_rows = sorted.len();
    if total_rows == 0 {
        app.selected_row = 0;
        app.scroll_offset = 0;

        let empty = Paragraph::new("No service cost insight is available yet.")
            .style(Style::default().fg(app.theme.text))
            .block(
                Block::default()
                    .title("Service Cost + Usage")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.primary)),
            );

        frame.render_widget(empty, area);
        return;
    }

    app.selected_row = app.selected_row.min(total_rows - 1);
    let visible_height = area.height.saturating_sub(3) as usize;
    if app.selected_row < app.scroll_offset as usize {
        app.scroll_offset = app.selected_row as u16;
    } else if app.selected_row >= app.scroll_offset as usize + visible_height {
        app.scroll_offset = (app.selected_row + 1 - visible_height) as u16;
    }

    let rows = sorted
        .iter()
        .enumerate()
        .skip(app.scroll_offset as usize)
        .take(visible_height)
        .map(|(index, insight)| {
            let pct = if total > 0.0 {
                insight.monthly_cost / total
            } else {
                0.0
            };

            let style = if index == app.selected_row {
                Style::default()
                    .fg(app.theme.highlight)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.text)
            };

            Row::new(vec![
                Cell::from(insight.service.clone()),
                Cell::from(format!("${:.2}", insight.monthly_cost)),
                Cell::from(format!("{:.1}%", pct * 100.0)),
                Cell::from(insight.primary_usage_summary()),
            ])
            .style(style)
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

fn render_service_cost_detail(frame: &mut Frame, area: Rect, app: &mut App) {
    let mut sorted = app.service_cost_insights.clone();
    sorted.sort_by(|a, b| {
        b.monthly_cost
            .partial_cmp(&a.monthly_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if sorted.is_empty() {
        let empty = Paragraph::new("No selected service detail is available yet.")
            .style(Style::default().fg(app.theme.text))
            .block(
                Block::default()
                    .title("Wrapped Detail")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.primary)),
            );

        frame.render_widget(empty, area);
        return;
    }

    let selected = &sorted[app.selected_row.min(sorted.len() - 1)];
    let total: f64 = sorted.iter().map(|insight| insight.monthly_cost).sum();
    let share = if total > 0.0 {
        selected.monthly_cost / total * 100.0
    } else {
        0.0
    };

    let usage_lines = if selected.top_usage_types.is_empty() {
        "No usage detail available".to_string()
    } else {
        selected
            .top_usage_types
            .iter()
            .map(|usage| format!("- {}", usage.summary()))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let detail = Paragraph::new(format!(
        "Service {}/{}\n{}\n\nMonthly cost ${:.2} ({share:.1}% of tracked spend)\n\nUsage Types\n{}",
        app.selected_row + 1,
        sorted.len(),
        selected.service,
        selected.monthly_cost,
        usage_lines
    ))
    .style(Style::default().fg(app.theme.text))
    .wrap(Wrap { trim: true })
    .scroll((app.detail_scroll_offset, 0))
    .block(
        Block::default()
            .title("Wrapped Detail")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.primary)),
    );

    frame.render_widget(detail, area);
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

    if app.wrap_mode_active() {
        let detail_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Min(0),
            ])
            .split(layout[1]);

        render_cost_6mo_chart(frame, detail_layout[0], app);
        render_service_cost_chart(frame, detail_layout[1], app);
        render_service_cost_detail(frame, detail_layout[2], app);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(layout[1]);

        render_cost_6mo_chart(frame, chunks[0], app);
        render_service_cost_chart(frame, chunks[1], app);
    }
}
