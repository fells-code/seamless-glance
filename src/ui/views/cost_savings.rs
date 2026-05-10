use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(0)])
        .split(area);

    let total_estimated_savings = app
        .cost_savings_opportunities
        .iter()
        .map(|opportunity| opportunity.estimated_monthly_savings)
        .sum::<f64>();
    let budget_gap = app.budget.forecast - app.budget.monthly_budget;
    let forecast_range = match (app.budget.forecast_low, app.budget.forecast_high) {
        (Some(low), Some(high)) => format!("Forecast range ${low:.2} - ${high:.2}"),
        _ => "Forecast range unavailable".into(),
    };

    let summary_text = format!(
        "Potential monthly savings: ${total_estimated_savings:.2}\n\
         Month-to-date: ${:.2}  |  Forecast: ${:.2}  |  Budget gap: {:+.2}\n\
         {}\n\
         This view combines spend, usage types, and waste-oriented findings into operator recommendations.",
        app.budget.month_to_date_cost, app.budget.forecast, budget_gap, forecast_range
    );

    let summary = Paragraph::new(summary_text)
        .style(Style::default().fg(app.theme.text))
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title("Cost Savings")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.primary)),
        );

    frame.render_widget(summary, layout[0]);

    let total_rows = app.cost_savings_opportunities.len();
    if total_rows == 0 {
        app.selected_row = 0;
        app.scroll_offset = 0;

        let empty = Paragraph::new(
            "No concrete cost-savings opportunities are available yet.\n\
             This view will highlight savings when spend and waste signals line up.",
        )
        .style(Style::default().fg(app.theme.text))
        .block(
            Block::default()
                .title("Opportunities")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.primary)),
        );

        frame.render_widget(empty, layout[1]);
        return;
    }

    app.selected_row = app.selected_row.min(total_rows - 1);
    let visible_height = layout[1].height.saturating_sub(3) as usize;

    if app.selected_row < app.scroll_offset as usize {
        app.scroll_offset = app.selected_row as u16;
    } else if app.selected_row >= app.scroll_offset as usize + visible_height {
        app.scroll_offset = (app.selected_row + 1 - visible_height) as u16;
    }

    let rows = app
        .cost_savings_opportunities
        .iter()
        .enumerate()
        .skip(app.scroll_offset as usize)
        .take(visible_height)
        .map(|(index, opportunity)| {
            let style = if index == app.selected_row {
                Style::default()
                    .fg(app.theme.highlight)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.text)
            };

            Row::new(vec![
                Cell::from(format!("${:.2}", opportunity.estimated_monthly_savings)),
                Cell::from(opportunity.service.clone()),
                Cell::from(format!("${:.2}", opportunity.monthly_cost)),
                Cell::from(opportunity.title.clone()),
                Cell::from(opportunity.evidence.clone()),
                Cell::from(opportunity.usage_context.clone()),
                Cell::from(opportunity.recommendation.clone()),
            ])
            .style(style)
        })
        .collect::<Vec<_>>();

    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Length(16),
            Constraint::Length(10),
            Constraint::Length(24),
            Constraint::Percentage(26),
            Constraint::Percentage(24),
            Constraint::Percentage(24),
        ],
    )
    .header(
        Row::new([
            "EST SAVE",
            "SERVICE",
            "CURRENT",
            "OPPORTUNITY",
            "EVIDENCE",
            "USAGE",
            "NEXT STEP",
        ])
        .style(
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .title("Savings Opportunities")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.primary)),
    );

    frame.render_widget(table, layout[1]);
}
