use crate::{
    app::App,
    models::finding::{FindingCategory, FindingSeverity},
    ui::views::list_table::{render_list_table, ListSelection, ListTable},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Paragraph, Row, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    // Wrap mode replaces the table with a single wrapped detail pane, so it has
    // to divert before the shared table renderer.
    if !app.findings.is_empty() && app.wrap_mode_active() {
        app.selected_row = app.selected_row.min(app.findings.len() - 1);
        render_wrapped_detail(frame, area, app);
        return;
    }

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
            title: "Findings",
            headers: &["SEV", "CATEGORY", "SERVICE", "REGION", "SUMMARY", "NEXT STEP"],
            widths: &[
                Constraint::Length(6),
                Constraint::Length(10),
                Constraint::Length(16),
                Constraint::Length(12),
                Constraint::Percentage(38),
                Constraint::Percentage(34),
            ],
            empty_message: "No findings detected right now.\n\
                            This view will surface incidents, waste, and hygiene issues as they appear.",
        },
        &app.findings,
        |finding| {
            let severity_style = match finding.severity {
                FindingSeverity::High => Style::default().fg(theme.primary),
                FindingSeverity::Medium => Style::default().fg(theme.accent),
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
                Cell::from(finding.summary.clone()),
                Cell::from(finding.next_step.clone()),
            ])
            .style(severity_style)
        },
    );
}

fn render_wrapped_detail(frame: &mut Frame, area: Rect, app: &mut App) {
    let finding = &app.findings[app.selected_row];
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    let metadata = Paragraph::new(format!(
        "Finding {}/{}\n{}  |  {}  |  {}  |  {}",
        app.selected_row + 1,
        app.findings.len(),
        finding.severity.as_str(),
        finding.category.as_str(),
        finding.service,
        finding.region
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

    let detail = Paragraph::new(format!(
        "Summary\n{}\n\nNext Step\n{}",
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
