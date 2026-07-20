use ratatui::{
    layout::Constraint,
    style::Style,
    widgets::{Cell, Row},
    Frame,
};

use crate::app::App;
use crate::ui::views::list_table::{render_list_table, visible_rows, ListSelection, ListTable};

pub fn render_apigatway(frame: &mut Frame, area: ratatui::layout::Rect, app: &mut App) {
    if crate::ui::views::status::render_unavailable(
        frame,
        area,
        "API Gateway",
        &app.apigateway_status,
        &app.theme,
    ) {
        return;
    }

    let theme = app.theme;

    let visible = app.visible_indices();
    let rows = visible_rows(&visible, &app.apigateway_apis);

    render_list_table(
        frame,
        area,
        ListSelection {
            selected_row: &mut app.selected_row,
            scroll_offset: &mut app.scroll_offset,
        },
        &theme,
        ListTable {
            title: "API Gateway",
            headers: &["Name", "Type", "ID", "Created", "Signals"],
            widths: &[
                Constraint::Percentage(30),
                Constraint::Percentage(10),
                Constraint::Percentage(20),
                Constraint::Percentage(25),
                Constraint::Percentage(15),
            ],
            empty_message: "No API Gateway APIs found in this region.",
        },
        &rows,
        |api| {
            let style = if api.needs_review() {
                Style::default().fg(theme.primary)
            } else {
                Style::default().fg(theme.text)
            };

            Row::new(vec![
                Cell::from(api.name.clone()),
                Cell::from(api.api_type.clone()),
                Cell::from(api.id.clone()),
                Cell::from(api.created_at.clone()),
                Cell::from({
                    let signals = api.review_signals();
                    if signals.is_empty() {
                        "-".into()
                    } else {
                        signals.join(",")
                    }
                }),
            ])
            .style(style)
        },
    );
}
