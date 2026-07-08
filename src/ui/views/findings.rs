use crate::{
    app::App,
    models::finding::{FindingCategory, FindingSeverity},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let total_rows = app.findings.len();
    if total_rows == 0 {
        app.selected_row = 0;
        app.scroll_offset = 0;

        let empty = Paragraph::new(
            "No findings detected right now.\n\
             This view will surface incidents, waste, and hygiene issues as they appear.",
        )
        .style(Style::default().fg(app.theme.text))
        .block(
            Block::default()
                .title("Findings")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.primary)),
        );

        frame.render_widget(empty, area);
        return;
    }

    app.selected_row = app.selected_row.min(total_rows - 1);

    if app.wrap_mode_active() {
        render_wrapped_detail(frame, area, app);
        return;
    }

    let visible_height = area.height.saturating_sub(3) as usize;

    if app.selected_row < app.scroll_offset as usize {
        app.scroll_offset = app.selected_row as u16;
    } else if app.selected_row >= app.scroll_offset as usize + visible_height {
        app.scroll_offset = (app.selected_row + 1 - visible_height) as u16;
    }

    let rows: Vec<Row> = app
        .findings
        .iter()
        .enumerate()
        .skip(app.scroll_offset as usize)
        .take(visible_height)
        .map(|(i, finding)| {
            let severity_style = match finding.severity {
                FindingSeverity::High => Style::default().fg(app.theme.primary),
                FindingSeverity::Medium => Style::default().fg(app.theme.accent),
            };

            let style = if i == app.selected_row {
                severity_style.add_modifier(Modifier::BOLD)
            } else {
                severity_style
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
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            ratatui::layout::Constraint::Length(6),
            ratatui::layout::Constraint::Length(10),
            ratatui::layout::Constraint::Length(16),
            ratatui::layout::Constraint::Length(12),
            ratatui::layout::Constraint::Percentage(38),
            ratatui::layout::Constraint::Percentage(34),
        ],
    )
    .header(
        Row::new([
            "SEV",
            "CATEGORY",
            "SERVICE",
            "REGION",
            "SUMMARY",
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
            .title("Findings")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.primary)),
    );

    frame.render_widget(table, area);
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
