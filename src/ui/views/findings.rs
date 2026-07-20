use crate::{
    app::App,
    models::finding::{FindingCategory, FindingSeverity},
    ui::views::list_table::{render_list_table, visible_rows, ListSelection, ListTable},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Paragraph, Row, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    // Wrap mode replaces the table with a single wrapped detail pane, so it has
    // to divert before the shared table renderer. Guarded on the filtered rows:
    // diverting with nothing to show would render an empty pane with no
    // explanation, since the empty-state message lives on the table path.
    let filtered_count = app.visible_indices().len();
    if filtered_count > 0 && app.wrap_mode_active() {
        app.selected_row = app.selected_row.min(filtered_count - 1);
        render_wrapped_detail(frame, area, app);
        return;
    }

    let theme = app.theme;

    let visible = app.visible_indices();
    let rows = visible_rows(&visible, &app.findings);

    render_list_table(
        frame,
        area,
        ListSelection {
            selected_row: &mut app.selected_row,
            scroll_offset: &mut app.scroll_offset,
        },
        &theme,
        ListTable {
            title: "Findings",
            headers: &["SEV", "CATEGORY", "SERVICE", "REGION", "RESOURCE", "COST", "SUMMARY"],
            widths: &[
                Constraint::Length(6),
                Constraint::Length(10),
                Constraint::Length(16),
                Constraint::Length(12),
                Constraint::Percentage(26),
                Constraint::Length(16),
                Constraint::Percentage(38),
            ],
            empty_message: "No findings detected right now.\n\
                            This view will surface incidents, waste, and hygiene issues as they appear.",
        },
        &rows,
        |finding| {
            let severity_style = match finding.severity {
                FindingSeverity::High => Style::default().fg(theme.primary),
                FindingSeverity::Medium => Style::default().fg(theme.accent),
                FindingSeverity::Low => Style::default().fg(theme.text),
            };

            let category = match finding.category {
                FindingCategory::Incident => "Incident",
                FindingCategory::Waste => "Waste",
                FindingCategory::Hygiene => "Hygiene",
            };

            Row::new(vec![
                Cell::from(finding.severity.as_str()),
                Cell::from(category),
                Cell::from(finding.service.clone()),
                Cell::from(finding.region.clone()),
                // Aggregate findings have no single resource to name.
                Cell::from(finding.resource_id.clone().unwrap_or_else(|| "-".into())),
                Cell::from(
                    finding
                        .cost
                        .map(|cost| cost.label())
                        .unwrap_or_else(|| "-".into()),
                ),
                Cell::from(finding.summary.clone()),
            ])
            .style(severity_style)
        },
    );
}

fn render_wrapped_detail(frame: &mut Frame, area: Rect, app: &mut App) {
    // Indexing here would panic if the selection ever outran the list, so read
    // it fallibly and bail rather than trusting an upstream clamp.
    let visible = app.visible_indices();
    let Some(finding) = visible
        .get(app.selected_row)
        .and_then(|&index| app.findings.get(index))
    else {
        return;
    };
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    let metadata = Paragraph::new(format!(
        "Finding {}/{}{}\n{}  |  {}  |  {}  |  {}  |  {}",
        app.selected_row + 1,
        visible.len(),
        if app.filter_is_active() {
            format!(" (filtered from {})", app.findings.len())
        } else {
            String::new()
        },
        finding.severity.as_str(),
        finding.category.as_str(),
        finding.service,
        finding.region,
        finding.resource_id.as_deref().unwrap_or("-")
    ))
    .style(Style::default().fg(app.theme.text))
    .wrap(Wrap { trim: true })
    .block(
        Block::default()
            .title("Findings Detail")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.primary)),
    );

    frame.render_widget(metadata, layout[0]);

    let cost_line = match finding.cost {
        Some(cost) => format!(
            "\n\nEstimated Cost\n{} at AWS public list price. Not billed spend: discounts, \
             Savings Plans, and Reserved Instances are not reflected.",
            cost.label()
        ),
        None => String::new(),
    };

    let detail = Paragraph::new(format!(
        "Summary\n{}\n\nNext Step\n{}{cost_line}",
        finding.summary, finding.next_step
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

    frame.render_widget(detail, layout[1]);
}
