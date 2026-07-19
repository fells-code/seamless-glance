//! One place for the list-view table plumbing every service view repeated:
//! clamping the selection, windowing rows to the visible height, keeping the
//! selected row on screen, the empty state, and assembling the table.
//!
//! Views supply the columns and their own content styling; selection styling is
//! applied here so it looks the same everywhere.

use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph, Row, Table},
    Frame,
};

use crate::ui::theme::Theme;

/// Rows reserved by the table chrome (top border, header, bottom border).
const CHROME_ROWS: u16 = 3;

pub struct ListTable<'a> {
    pub title: &'a str,
    pub headers: &'a [&'a str],
    pub widths: &'a [Constraint],
    /// Shown instead of the table when there are no rows.
    pub empty_message: &'a str,
}

/// The mutable selection state a list view drives. Passed as separate borrows so
/// callers can hand over disjoint `App` fields alongside the item slice.
pub struct ListSelection<'a> {
    pub selected_row: &'a mut usize,
    pub scroll_offset: &'a mut u16,
}

/// Render `items` as a scrolling, selectable table.
///
/// `make_row` builds one row and applies any content styling (for example a
/// warning color). The selected row's style is overridden here, matching the
/// previous per-view behavior where selection won over content styling.
pub fn render_list_table<T>(
    frame: &mut Frame,
    area: Rect,
    selection: ListSelection<'_>,
    theme: &Theme,
    spec: ListTable<'_>,
    items: &[T],
    make_row: impl Fn(&T) -> Row<'static>,
) {
    let ListSelection {
        selected_row,
        scroll_offset,
    } = selection;

    if items.is_empty() {
        *selected_row = 0;
        *scroll_offset = 0;

        let empty = Paragraph::new(spec.empty_message)
            .style(Style::default().fg(theme.text))
            .block(
                Block::default()
                    .title(spec.title.to_string())
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.primary)),
            );

        frame.render_widget(empty, area);
        return;
    }

    *selected_row = (*selected_row).min(items.len() - 1);

    let visible_height = area.height.saturating_sub(CHROME_ROWS) as usize;

    // Keep the selected row on screen.
    if visible_height > 0 {
        if *selected_row < *scroll_offset as usize {
            *scroll_offset = *selected_row as u16;
        } else if *selected_row >= *scroll_offset as usize + visible_height {
            *scroll_offset = (*selected_row + 1 - visible_height) as u16;
        }
    }

    let selected_style = Style::default()
        .fg(theme.highlight)
        .add_modifier(Modifier::BOLD);

    // `enumerate` before `skip` so the index compared against the selection is
    // the absolute row index, not the position within the window.
    let rows: Vec<Row> = items
        .iter()
        .enumerate()
        .skip(*scroll_offset as usize)
        .take(visible_height)
        .map(|(index, item)| {
            let row = make_row(item);
            if index == *selected_row {
                row.style(selected_style)
            } else {
                row
            }
        })
        .collect();

    let table = Table::new(rows, spec.widths)
        .header(
            Row::new(spec.headers.to_vec()).style(
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(
            Block::default()
                .title(spec.title.to_string())
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.primary)),
        )
        .style(Style::default().fg(theme.text));

    frame.render_widget(table, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::Theme;
    use ratatui::{backend::TestBackend, widgets::Cell, Terminal};

    /// Render `items` into a `height`-row viewport and return the drawn text
    /// plus the selection state the helper settled on.
    fn draw(items: &[String], selected: usize, offset: u16, height: u16) -> (String, usize, u16) {
        let mut terminal = Terminal::new(TestBackend::new(40, height)).unwrap();
        let theme = Theme::autumn();
        let mut selected_row = selected;
        let mut scroll_offset = offset;

        terminal
            .draw(|frame| {
                render_list_table(
                    frame,
                    frame.size(),
                    ListSelection {
                        selected_row: &mut selected_row,
                        scroll_offset: &mut scroll_offset,
                    },
                    &theme,
                    ListTable {
                        title: "Things",
                        headers: &["NAME"],
                        widths: &[Constraint::Percentage(100)],
                        empty_message: "nothing here",
                    },
                    items,
                    |s: &String| Row::new(vec![Cell::from(s.clone())]),
                );
            })
            .unwrap();

        let text = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();

        (text, selected_row, scroll_offset)
    }

    fn rows(count: usize) -> Vec<String> {
        (0..count).map(|i| format!("row-{i}")).collect()
    }

    #[test]
    fn renders_the_empty_message_when_there_are_no_rows() {
        let (text, selected, offset) = draw(&[], 4, 3, 8);
        assert!(text.contains("nothing here"));
        assert_eq!(selected, 0, "selection resets when the list empties");
        assert_eq!(offset, 0);
    }

    #[test]
    fn clamps_a_selection_past_the_end_of_the_list() {
        let (_, selected, _) = draw(&rows(3), 99, 0, 10);
        assert_eq!(selected, 2);
    }

    #[test]
    fn scrolls_to_keep_the_selected_row_visible() {
        // Height 8 minus borders and header leaves 5 visible rows, so reaching
        // row 15 has to scroll the window down.
        let (text, selected, offset) = draw(&rows(20), 15, 0, 8);

        assert_eq!(selected, 15);
        assert!(offset > 0, "must scroll down to reveal row 15");
        assert!(
            text.contains("row-15"),
            "the selected row must be on screen"
        );
        assert!(
            !text.contains("row-0 "),
            "the top of the list should have scrolled out of view"
        );
    }

    #[test]
    fn a_shrinking_list_cannot_leave_the_selection_out_of_bounds() {
        // Select deep into a long list, then render a much shorter one, the way
        // a refresh that returns fewer rows would.
        let (_, selected, _) = draw(&rows(30), 27, 20, 10);
        assert_eq!(selected, 27);

        let (text, selected, offset) = draw(&rows(4), 27, 20, 10);
        assert_eq!(selected, 3, "selection clamps to the last row that exists");
        assert!(
            (offset as usize) < 4,
            "the window cannot sit past the end of the list"
        );
        assert!(text.contains("row-3"));
    }

    #[test]
    fn scrolls_back_up_when_the_selection_moves_above_the_window() {
        let (text, _, offset) = draw(&rows(20), 2, 11, 8);
        assert_eq!(offset, 2, "window follows the selection upward");
        assert!(text.contains("row-2"));
    }
}
