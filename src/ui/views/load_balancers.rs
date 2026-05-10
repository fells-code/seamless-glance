use crate::app::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

pub fn render_lbs(frame: &mut Frame, area: Rect, app: &mut App) {
    let total_rows = app.load_balancers.len();
    if total_rows == 0 {
        app.selected_row = 0;
        app.scroll_offset = 0;
    }

    if total_rows > 0 {
        app.selected_row = app.selected_row.min(total_rows - 1);
    }

    let visible_height = area.height.saturating_sub(3) as usize; // header + borders

    // Keep selected row in view
    if app.selected_row < app.scroll_offset as usize {
        app.scroll_offset = app.selected_row as u16;
    } else if app.selected_row >= app.scroll_offset as usize + visible_height {
        app.scroll_offset = (app.selected_row + 1 - visible_height) as u16;
    }

    let rows: Vec<Row> = app
        .load_balancers
        .iter()
        .enumerate()
        .skip(app.scroll_offset as usize)
        .take(visible_height)
        .map(|(i, lb)| {
            let style = if i == app.selected_row {
                Style::default().fg(app.theme.highlight)
            } else if lb.has_zero_healthy_targets() {
                Style::default()
                    .fg(app.theme.primary)
                    .add_modifier(Modifier::BOLD)
            } else if lb.has_no_active_targets() {
                Style::default().fg(app.theme.accent)
            } else {
                Style::default().fg(app.theme.text)
            };
            Row::new(vec![
                Cell::from(lb.name.clone()),
                Cell::from(lb.lb_type.clone()),
                Cell::from(lb.scheme.clone()),
                Cell::from(lb.state.clone()),
                Cell::from(lb.az_count.to_string()),
                Cell::from(lb.attached_target_groups.to_string()),
                Cell::from(lb.total_targets.to_string()),
                Cell::from(lb.healthy_targets.to_string()),
                Cell::from({
                    let signals = lb.review_signals();
                    if signals.is_empty() {
                        "-".into()
                    } else {
                        signals.join(",")
                    }
                }),
            ])
            .style(style)
        })
        .collect();

    let header = Row::new(vec![
        Cell::from("Name"),
        Cell::from("Type"),
        Cell::from("Scheme"),
        Cell::from("State"),
        Cell::from("AZs"),
        Cell::from("TGs"),
        Cell::from("Targets"),
        Cell::from("Healthy"),
        Cell::from("Signals"),
    ])
    .style(Style::default().fg(app.theme.accent));

    let table = Table::new(
        rows,
        &[
            Constraint::Percentage(24),
            Constraint::Length(18),
            Constraint::Length(18),
            Constraint::Length(10),
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(18),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title("Load Balancers")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.primary)),
    );

    frame.render_widget(table, area);
}
