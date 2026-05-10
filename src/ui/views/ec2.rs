use crate::app::App;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

pub fn render_ec2(frame: &mut Frame, area: Rect, app: &mut App) {
    let total_rows = app.ec2_instances.len();
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
        .ec2_instances
        .iter()
        .enumerate()
        .skip(app.scroll_offset as usize)
        .take(visible_height)
        .map(|(i, inst)| {
            let style = if i == app.selected_row {
                Style::default().fg(app.theme.highlight)
            } else if inst.has_tag_coverage_gap() {
                Style::default().fg(app.theme.accent)
            } else if inst.has_sustained_low_cpu() || inst.needs_stopped_review() {
                Style::default().fg(app.theme.primary)
            } else {
                Style::default().fg(app.theme.text)
            };

            Row::new(vec![
                Cell::from(inst.id.clone()),
                Cell::from(inst.name.clone().unwrap_or("-".into())),
                Cell::from(inst.region.clone()),
                Cell::from(inst.state.clone()),
                Cell::from(inst.instance_type.clone()),
                Cell::from(inst.formatted_avg_cpu()),
                Cell::from(inst.owner.clone().unwrap_or("-".into())),
                Cell::from(inst.environment.clone().unwrap_or("-".into())),
                Cell::from(inst.public_ip.clone().unwrap_or("-".into())),
                Cell::from(inst.private_ip.clone().unwrap_or("-".into())),
                Cell::from(inst.az.clone()),
                Cell::from({
                    let signals = inst.review_signals();
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
        "Instance ID",
        "Name",
        "Region",
        "State",
        "Type",
        "Avg CPU",
        "Owner",
        "Env",
        "Public IP",
        "Private IP",
        "AZ",
        "Signals",
    ])
    .style(Style::default().fg(app.theme.accent));

    let widths = [
        ratatui::layout::Constraint::Percentage(30),
        ratatui::layout::Constraint::Percentage(16),
        ratatui::layout::Constraint::Percentage(10),
        ratatui::layout::Constraint::Percentage(10),
        ratatui::layout::Constraint::Percentage(10),
        ratatui::layout::Constraint::Percentage(9),
        ratatui::layout::Constraint::Percentage(10),
        ratatui::layout::Constraint::Percentage(9),
        ratatui::layout::Constraint::Percentage(11),
        ratatui::layout::Constraint::Percentage(11),
        ratatui::layout::Constraint::Percentage(8),
        ratatui::layout::Constraint::Percentage(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title("EC2 Instances")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.primary)),
        )
        .widths([
            ratatui::layout::Constraint::Percentage(30),
            ratatui::layout::Constraint::Percentage(16),
            ratatui::layout::Constraint::Percentage(10),
            ratatui::layout::Constraint::Percentage(10),
            ratatui::layout::Constraint::Percentage(10),
            ratatui::layout::Constraint::Percentage(9),
            ratatui::layout::Constraint::Percentage(10),
            ratatui::layout::Constraint::Percentage(9),
            ratatui::layout::Constraint::Percentage(11),
            ratatui::layout::Constraint::Percentage(11),
            ratatui::layout::Constraint::Percentage(8),
            ratatui::layout::Constraint::Percentage(12),
        ]);

    frame.render_widget(table, area);
}
