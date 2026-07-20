use crate::app::App;
use crate::ui::views::list_table::{
    filter_query, render_list_table, visible_rows, ListSelection, ListTable, RowCells,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    if crate::ui::views::status::render_unavailable(
        frame,
        area,
        "Cost Explorer",
        &app.cost_status,
        &app.theme,
    ) {
        return;
    }

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

    // Wrap mode replaces the table with a detail pane, so it diverts before the
    // shared table renderer (and only with a row to describe).
    // Guarded on the filtered rows: diverting with nothing to show would leave
    // an empty pane, since the empty-state message lives on the table path.
    let filtered_count = app.visible_indices().len();
    if filtered_count > 0 && app.wrap_mode_active() {
        app.selected_row = app.selected_row.min(filtered_count - 1);
        render_wrapped_detail(frame, area, app, &summary_text);
        return;
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(0)])
        .split(area);

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

    let theme = app.theme;

    let filter = filter_query(&app.row_filter);
    let visible = app.visible_indices();
    let rows = visible_rows(&visible, &app.cost_savings_opportunities);

    render_list_table(
        frame,
        layout[1],
        ListSelection {
            selected_row: &mut app.selected_row,
            scroll_offset: &mut app.scroll_offset,
        },
        &theme,
        ListTable {
            title: "Savings Opportunities",
            headers: &[
                "EST SAVE",
                "SERVICE",
                "CURRENT",
                "OPPORTUNITY",
                "EVIDENCE",
                "USAGE",
                "NEXT STEP",
            ],
            widths: &[
                Constraint::Length(10),
                Constraint::Length(16),
                Constraint::Length(10),
                Constraint::Length(24),
                Constraint::Percentage(26),
                Constraint::Percentage(24),
                Constraint::Percentage(24),
            ],
            empty_message: "No concrete cost-savings opportunities are available yet.\n\
                            This view will highlight savings when spend and waste signals line up.",
            filter,
            // This view renders its own wrapped detail before reaching here.
            wrapped: false,
        },
        &rows,
        |opportunity| RowCells {
            cells: vec![
                format!("${:.2}", opportunity.estimated_monthly_savings),
                opportunity.service.clone(),
                format!("${:.2}", opportunity.monthly_cost),
                opportunity.title.clone(),
                opportunity.evidence.clone(),
                opportunity.usage_context.clone(),
                opportunity.recommendation.clone(),
            ],
            style: Style::default().fg(theme.text),
        },
    );
}

fn render_wrapped_detail(frame: &mut Frame, area: Rect, app: &mut App, summary_text: &str) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Length(6),
            Constraint::Min(0),
        ])
        .split(area);

    let summary = Paragraph::new(summary_text.to_string())
        .style(Style::default().fg(app.theme.text))
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title("Cost Savings")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.primary)),
        );

    frame.render_widget(summary, layout[0]);

    // Indexing here would panic if the selection ever outran the list, so read
    // it fallibly and bail rather than trusting an upstream clamp.
    let visible = app.visible_indices();
    let Some(opportunity) = visible
        .get(app.selected_row)
        .and_then(|&index| app.cost_savings_opportunities.get(index))
    else {
        return;
    };
    let metadata = Paragraph::new(format!(
        "Opportunity {}/{}{}\n{}  |  Current ${:.2}  |  Estimated savings ${:.2}\n{}",
        app.selected_row + 1,
        visible.len(),
        if app.filter_is_active() {
            format!(" (filtered from {})", app.cost_savings_opportunities.len())
        } else {
            String::new()
        },
        opportunity.service,
        opportunity.monthly_cost,
        opportunity.estimated_monthly_savings,
        opportunity.title
    ))
    .style(Style::default().fg(app.theme.text))
    .wrap(Wrap { trim: true })
    .block(
        Block::default()
            .title("Selected Opportunity")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.primary)),
    );

    frame.render_widget(metadata, layout[1]);

    let detail = Paragraph::new(format!(
        "Evidence\n{}\n\nUsage Context\n{}\n\nRecommendation\n{}",
        opportunity.evidence, opportunity.usage_context, opportunity.recommendation
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

    frame.render_widget(detail, layout[2]);
}
