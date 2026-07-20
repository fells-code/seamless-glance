use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{BarChart, Block, Borders, Cell, Paragraph, Row, Wrap},
    Frame,
};

use crate::{
    app::App,
    aws::cost::last_6_month_labels,
    ui::views::list_table::{render_list_table, visible_rows, ListSelection, ListTable},
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

fn render_service_cost_chart(frame: &mut Frame, area: Rect, app: &mut App) {
    // Rows are ranked by spend, so the selection index refers to sort order.
    let sorted = app.sorted_cost_insights();
    let visible = app.visible_indices();
    let rows = visible_rows(&visible, &sorted);

    let total: f64 = sorted.iter().map(|insight| insight.monthly_cost).sum();
    let theme = app.theme;

    render_list_table(
        frame,
        area,
        ListSelection {
            selected_row: &mut app.selected_row,
            scroll_offset: &mut app.scroll_offset,
        },
        &theme,
        ListTable {
            title: "Service Cost + Usage",
            headers: &["SERVICE", "COST", "SHARE", "TOP USAGE"],
            widths: &[
                Constraint::Percentage(32),
                Constraint::Length(10),
                Constraint::Length(8),
                Constraint::Percentage(58),
            ],
            empty_message: "No service cost insight is available yet.",
        },
        &rows,
        |insight| {
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
            .style(Style::default().fg(theme.text))
        },
    );
}

fn render_service_cost_detail(frame: &mut Frame, area: Rect, app: &mut App) {
    let sorted = app.sorted_cost_insights();
    let visible = app.visible_indices();
    let rows = visible_rows(&visible, &sorted);

    if rows.is_empty() {
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

    let selected = rows[app.selected_row.min(rows.len() - 1)];
    // Share is of total spend, which the filter does not change.
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
    if crate::ui::views::status::render_unavailable(
        frame,
        area,
        "Cost Explorer",
        &app.cost_status,
        &app.theme,
    ) {
        return;
    }

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
