//! One place for the list-view table plumbing every service view repeated:
//! clamping the selection, windowing rows to the visible height, keeping the
//! selected row on screen, the empty state, and assembling the table.
//!
//! Views supply the columns and their own content styling; selection styling is
//! applied here so it looks the same everywhere.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::ui::theme::Theme;

/// Rows reserved by the table chrome (top border, header, bottom border).
const CHROME_ROWS: u16 = 3;

/// Columns reserved by the table chrome (left and right borders).
const CHROME_COLUMNS: u16 = 2;

/// Blank column ratatui leaves between cells, its `Table` default.
const COLUMN_SPACING: u16 = 1;

/// Marks a value the column was too narrow to show in full.
pub const ELLIPSIS: char = '\u{2026}';

/// One row's cell text, before any truncation.
///
/// Views hand over plain strings rather than assembled `Row`s so the table can
/// see how wide each value is, shorten it to fit, and hand the full value to the
/// row-detail overlay. A view that built its own `Row` would hide both.
pub struct RowCells {
    pub cells: Vec<String>,
    pub style: Style,
}

/// Shorten `value` to `width`, marking that it was cut.
///
/// Counts characters rather than bytes so a multi-byte value is not split
/// mid-character. Width is in cells, so this still assumes one cell per
/// character, which holds for the ASCII identifiers AWS returns.
pub fn ellipsize(value: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    if value.chars().count() <= width {
        return value.to_string();
    }

    // One cell goes to the ellipsis itself.
    let kept: String = value.chars().take(width.saturating_sub(1)).collect();

    format!("{kept}{ELLIPSIS}")
}

/// Resolve the width each column actually gets, so cells can be shortened to
/// what will be drawn.
///
/// Uses the same solver `Table` does, against the same inner area, rather than
/// reimplementing how constraints are satisfied.
fn column_widths(area: Rect, widths: &[Constraint]) -> Vec<usize> {
    let inner = Rect {
        x: 0,
        y: 0,
        width: area.width.saturating_sub(CHROME_COLUMNS),
        height: 1,
    };

    Layout::horizontal(widths)
        .spacing(COLUMN_SPACING)
        .split(inner)
        .iter()
        .map(|rect| rect.width as usize)
        .collect()
}

pub struct ListTable<'a> {
    pub title: &'a str,
    pub headers: &'a [&'a str],
    pub widths: &'a [Constraint],
    /// Shown instead of the table when the view has no rows at all.
    pub empty_message: &'a str,
    /// The active row filter, when one is narrowing this view.
    ///
    /// An empty view means something different when a filter is on, so the
    /// empty state has to say which emptiness it is rather than claiming the
    /// account has no such resources.
    pub filter: Option<&'a str>,
    /// Show the selected row's values in full instead of the table.
    ///
    /// The table shortens values to fit their column, so this is the path that
    /// reveals what was cut.
    pub wrapped: bool,
}

/// The mutable selection state a list view drives. Passed as separate borrows so
/// callers can hand over disjoint `App` fields alongside the item slice.
/// Project a view's backing data down to the rows a filter leaves visible.
///
/// Takes indices rather than the `App` so the borrow of the data is a plain
/// field borrow, leaving the selection fields free to be borrowed mutably.
pub fn visible_rows<'a, T>(indices: &[usize], items: &'a [T]) -> Vec<&'a T> {
    indices
        .iter()
        .filter_map(|&index| items.get(index))
        .collect()
}

/// The active filter query, or `None` when nothing is narrowing the view.
///
/// Takes the query directly rather than the `App` so the borrow is a plain
/// field borrow, leaving the selection fields free to be borrowed mutably.
pub fn filter_query(row_filter: &str) -> Option<&str> {
    let query = row_filter.trim();

    (!query.is_empty()).then_some(query)
}

pub struct ListSelection<'a> {
    pub selected_row: &'a mut usize,
    pub scroll_offset: &'a mut u16,
}

/// Show one row's values in full, one per line.
///
/// The table shortens values to their column width, so this is where a value
/// too long to fit becomes readable. Values wrap rather than being cut.
fn render_row_detail(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    spec: &ListTable<'_>,
    row: RowCells,
    selected_row: usize,
    total: usize,
) {
    let width = spec
        .headers
        .iter()
        .map(|header| header.chars().count())
        .max()
        .unwrap_or(0);

    let body = spec
        .headers
        .iter()
        .zip(row.cells.iter())
        .map(|(header, value)| format!("{header:<width$}  {value}"))
        .collect::<Vec<_>>()
        .join("\n");

    let detail = Paragraph::new(format!("Row {}/{}\n\n{body}", selected_row + 1, total))
        .style(Style::default().fg(theme.text))
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title(format!("{} Detail", spec.title))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.primary)),
        );

    frame.render_widget(detail, area);
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
    make_row: impl Fn(&T) -> RowCells,
) {
    let ListSelection {
        selected_row,
        scroll_offset,
    } = selection;

    if items.is_empty() {
        *selected_row = 0;
        *scroll_offset = 0;

        let message = match spec.filter {
            Some(query) => format!(
                "No rows match {query:?}.\nPress Esc to clear the filter and show every row."
            ),
            None => spec.empty_message.to_string(),
        };

        let empty = Paragraph::new(message)
            .style(Style::default().fg(theme.text))
            // Without this the second line of a multi-line message keeps the
            // indentation of the source continuation and is clipped at the
            // pane width instead of wrapping.
            .wrap(Wrap { trim: true })
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
    if spec.wrapped {
        render_row_detail(
            frame,
            area,
            theme,
            &spec,
            make_row(&items[*selected_row]),
            *selected_row,
            items.len(),
        );
        return;
    }

    let widths = column_widths(area, spec.widths);

    let rows: Vec<Row> = items
        .iter()
        .enumerate()
        .skip(*scroll_offset as usize)
        .take(visible_height)
        .map(|(index, item)| {
            let built = make_row(item);

            let cells = built
                .cells
                .iter()
                .enumerate()
                .map(|(column, value)| {
                    Cell::from(ellipsize(value, widths.get(column).copied().unwrap_or(0)))
                })
                .collect::<Vec<_>>();

            let row = Row::new(cells).style(built.style);
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
    use ratatui::{backend::TestBackend, Terminal};

    /// Render `items` into a `height`-row viewport and return the drawn text
    /// plus the selection state the helper settled on.
    /// Render with a narrow terminal so the single column is forced to cut.
    fn draw_wrapped(items: &[String], selected: usize) -> String {
        render_with(items, selected, 40, 12, true, None)
    }

    fn draw_empty_with_filter(filter: Option<&str>) -> String {
        render_with(&[], 0, 40, 8, false, filter)
    }

    fn render_with(
        items: &[String],
        selected: usize,
        width: u16,
        height: u16,
        wrapped: bool,
        filter: Option<&str>,
    ) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let theme = Theme::autumn();
        let mut selected_row = selected;
        let mut scroll_offset = 0;

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
                        filter,
                        wrapped,
                    },
                    items,
                    |s: &String| RowCells {
                        cells: vec![s.clone()],
                        style: Style::default(),
                    },
                );
            })
            .unwrap();

        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

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
                        filter: None,
                        wrapped: false,
                    },
                    items,
                    |s: &String| RowCells {
                        cells: vec![s.clone()],
                        style: Style::default(),
                    },
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

    #[test]
    fn a_value_that_fits_is_left_alone() {
        assert_eq!(ellipsize("web-1", 10), "web-1");
        assert_eq!(ellipsize("exactly-10", 10), "exactly-10");
    }

    #[test]
    fn a_value_too_long_is_cut_and_marked() {
        let cut = ellipsize("prod/payments/stripe/webhook-key", 10);

        assert_eq!(cut.chars().count(), 10, "the mark fits inside the column");
        assert!(cut.ends_with(ELLIPSIS));
        assert!(cut.starts_with("prod/payt".chars().next().unwrap()));
    }

    /// Counting characters rather than bytes, so a multi-byte value is never
    /// split part-way through one.
    #[test]
    fn a_multi_byte_value_is_not_split_mid_character() {
        let cut = ellipsize("ünïcödé-resource-name", 6);

        assert_eq!(cut.chars().count(), 6);
        assert!(cut.ends_with(ELLIPSIS));
    }

    #[test]
    fn a_zero_width_column_renders_nothing() {
        assert_eq!(ellipsize("anything", 0), "");
    }

    #[test]
    fn a_long_cell_is_shortened_to_its_column() {
        let items = vec!["a-very-long-resource-identifier-that-overflows".to_string()];
        let (text, _, _) = draw(&items, 0, 0, 6);

        assert!(text.contains(ELLIPSIS), "the cut is marked: {text}");
        assert!(
            !text.contains("that-overflows"),
            "the tail is not drawn: {text}"
        );
    }

    /// The reveal path: the table cuts values to fit, so wrap mode has to show
    /// the whole thing or a truncated value would be unrecoverable.
    #[test]
    fn wrap_mode_shows_the_selected_value_in_full() {
        let items = vec!["a-very-long-resource-identifier-that-overflows".to_string()];
        let text = draw_wrapped(&items, 0);

        // The tail the table dropped is what has to reappear. It is asserted as
        // a fragment because the detail pane wraps the value across lines and
        // the test buffer is flattened without row separators.
        assert!(
            text.contains("verflows"),
            "the cut-off tail is revealed: {text}"
        );
        assert!(text.contains("NAME"), "the column is labelled: {text}");
    }

    #[test]
    fn wrap_mode_reports_which_row_is_shown() {
        let items = vec!["first".to_string(), "second".to_string()];
        let text = draw_wrapped(&items, 1);

        assert!(text.contains("Row 2/2"), "{text}");
        assert!(text.contains("second"), "{text}");
    }

    /// An empty view means something different when a filter is on. Claiming
    /// the account has no such resources would be wrong.
    #[test]
    fn a_filtered_empty_view_says_the_filter_hid_everything() {
        let text = draw_empty_with_filter(Some("orders"));

        assert!(text.contains("orders"), "names the query: {text}");
        assert!(text.contains("Esc"), "offers a way out: {text}");
        assert!(!text.contains("nothing here"), "not the unfiltered message");
    }

    #[test]
    fn an_unfiltered_empty_view_keeps_the_view_message() {
        let text = draw_empty_with_filter(None);

        assert!(text.contains("nothing here"), "{text}");
    }

    #[test]
    fn a_blank_query_does_not_count_as_a_filter() {
        assert_eq!(filter_query("  "), None);
        assert_eq!(filter_query(""), None);
        assert_eq!(filter_query(" orders "), Some("orders"));
    }
}
