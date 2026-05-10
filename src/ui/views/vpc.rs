use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

use crate::app::App;

pub fn render_vpc(frame: &mut Frame, area: Rect, app: &mut App) {
    let total_rows = app.vpcs.len();
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
        .vpcs
        .iter()
        .enumerate()
        .skip(app.scroll_offset as usize)
        .take(visible_height)
        .map(|(i, v)| {
            let style = if i == app.selected_row {
                Style::default().fg(app.theme.highlight)
            } else if v.is_default {
                Style::default().fg(app.theme.primary)
            } else {
                Style::default().fg(app.theme.text)
            };

            Row::new(vec![
                Cell::from(v.vpc_id.clone()),
                Cell::from(v.cidr.clone()),
                Cell::from(v.state.clone()),
                Cell::from(if v.is_default { "Yes" } else { "No" }),
                Cell::from(v.subnet_count.to_string()),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        &[
            Constraint::Percentage(28),
            Constraint::Percentage(22),
            Constraint::Percentage(12),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
        ],
    )
    .header(
        Row::new(vec!["VPC ID", "CIDR", "State", "Default", "Subnets"])
            .style(Style::default().fg(app.theme.accent)),
    )
    .block(
        Block::default()
            .title("VPCs")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.primary)),
    );

    frame.render_widget(table, area);
}
