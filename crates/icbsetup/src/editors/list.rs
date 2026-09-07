use icy_board_tui::insert_table::InsertTable;
use ratatui::{Frame, layout::Rect};

/// List mechanics only; validation, insertion position and persistence belong to the editor.
pub(crate) trait EditorList {
    /// Clamp an existing selection, preserve explicit None, and clear it for an empty list.
    fn sync_rows(&mut self, len: usize, selected: Option<usize>);
    /// Invalid/stale selections and out-of-bounds destinations never mutate the rows.
    fn move_row<T>(&mut self, rows: &mut [T], delta: isize);
    fn remove_row<T>(&mut self, rows: &mut Vec<T>) -> Option<T>;
    /// Append without moving a valid selection; select the first row if none was selected.
    fn push_row<T>(&mut self, rows: &mut Vec<T>, row: T);
    fn render_list(&mut self, frame: &mut Frame, area: Rect);
}

impl EditorList for InsertTable<'_> {
    fn sync_rows(&mut self, len: usize, selected: Option<usize>) {
        let selected = selected.and_then(|index| len.checked_sub(1).map(|last| index.min(last)));
        self.content_length = len;
        self.table_state.select(selected);
        self.scroll_state = self.scroll_state.content_length(len).position(selected.unwrap_or(0));
    }

    fn move_row<T>(&mut self, rows: &mut [T], delta: isize) {
        let mut selected = self.table_state.selected();
        if let Some(index) = selected.filter(|index| *index < rows.len())
            && let Some(target) = index.checked_add_signed(delta).filter(|target| *target < rows.len())
        {
            rows.swap(index, target);
            selected = Some(target);
        }
        self.sync_rows(rows.len(), selected);
    }

    fn remove_row<T>(&mut self, rows: &mut Vec<T>) -> Option<T> {
        let selected = self.table_state.selected();
        let removed = selected.filter(|index| *index < rows.len()).map(|index| rows.remove(index));
        self.sync_rows(rows.len(), selected);
        removed
    }

    fn push_row<T>(&mut self, rows: &mut Vec<T>, row: T) {
        let selected = self.table_state.selected().or(Some(0));
        rows.push(row);
        self.sync_rows(rows.len(), selected);
    }

    fn render_list(&mut self, frame: &mut Frame, area: Rect) {
        // Normalize before rendering; never restore a stale selection afterwards.
        self.sync_rows(self.content_length, self.table_state.selected());
        // InsertTable reserves one column by subtraction.
        if area.width > 1 && area.height > 0 {
            self.render_table(frame, area);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use icy_board_tui::insert_table::Column;
    use ratatui::{
        Terminal,
        backend::TestBackend,
        text::Line,
        widgets::{ScrollbarState, TableState},
    };

    fn table(len: usize, selected: Option<usize>) -> InsertTable<'static> {
        InsertTable {
            content_length: len,
            table_state: TableState::default().with_selected(selected),
            scroll_state: ScrollbarState::default().content_length(99).position(99),
            columns: vec![Column::new("Row")],
            numbered: true,
            get_content: Box::new(|table, row, _| {
                assert!(table.table_state.selected().is_none_or(|index| index < table.content_length));
                Line::from(format!("row {row}"))
            }),
        }
    }

    fn assert_state(table: &InsertTable<'_>, len: usize, selected: Option<usize>) {
        assert_eq!(table.content_length, len);
        assert_eq!(table.table_state.selected(), selected);
        assert_eq!(
            table.scroll_state,
            ScrollbarState::default().content_length(len).position(selected.unwrap_or(0))
        );
    }

    #[test]
    fn sync_clamps_stale_and_clears_empty_but_preserves_none() {
        let mut table = table(3, Some(usize::MAX));
        table.sync_rows(3, table.table_state.selected());
        assert_state(&table, 3, Some(2));
        table.sync_rows(3, None);
        assert_state(&table, 3, None);
        table.sync_rows(0, Some(2));
        assert_state(&table, 0, None);
    }

    #[test]
    fn removal_selects_successor_then_previous_then_none() {
        let mut rows = vec![10, 20, 30];
        let mut table = table(rows.len(), Some(1));
        assert_eq!(table.remove_row(&mut rows), Some(20));
        assert_eq!(rows, [10, 30]);
        assert_state(&table, 2, Some(1));
        assert_eq!(table.remove_row(&mut rows), Some(30));
        assert_state(&table, 1, Some(0));
        assert_eq!(table.remove_row(&mut rows), Some(10));
        assert_state(&table, 0, None);
        assert_eq!(table.remove_row(&mut rows), None);
        assert_state(&table, 0, None);
    }

    #[test]
    fn stale_or_missing_selection_never_removes_another_row() {
        let mut rows = vec![10, 20];
        for selected in [Some(2), Some(usize::MAX), None] {
            let mut table = table(99, selected);
            assert_eq!(table.remove_row(&mut rows), None);
            assert_eq!(rows, [10, 20]);
            assert_state(&table, 2, selected.map(|_| 1));
        }
        let mut table = table(99, Some(usize::MAX));
        assert_eq!(table.remove_row(&mut Vec::<u8>::new()), None);
        assert_state(&table, 0, None);
    }

    #[test]
    fn reorder_tracks_selection_and_scrollbar() {
        let mut rows = vec![10, 20, 30];
        let mut table = table(99, Some(1));
        table.move_row(&mut rows, -1);
        assert_eq!(rows, [20, 10, 30]);
        assert_state(&table, 3, Some(0));
        table.move_row(&mut rows, 1);
        assert_eq!(rows, [10, 20, 30]);
        assert_state(&table, 3, Some(1));
    }

    #[test]
    fn reorder_boundaries_overflow_empty_and_stale_are_noops() {
        for (selected, delta) in [
            (Some(0), -1),
            (Some(2), 1),
            (Some(1), isize::MIN),
            (Some(1), isize::MAX),
            (Some(usize::MAX), 1),
            (Some(3), -1),
            (None, 1),
            (Some(1), 0),
        ] {
            let mut rows = vec![10, 20, 30];
            let mut table = table(99, selected);
            table.move_row(&mut rows, delta);
            assert_eq!(rows, [10, 20, 30]);
            assert_state(&table, 3, selected.map(|index| index.min(2)));
        }
        let mut table = table(99, Some(1));
        table.move_row(&mut [] as &mut [u8], -1);
        assert_state(&table, 0, None);
    }

    #[test]
    fn append_keeps_valid_selection_and_selects_first_when_missing() {
        let mut rows = vec![10, 20];
        let mut table = table(2, Some(1));
        table.push_row(&mut rows, 30);
        assert_eq!(rows, [10, 20, 30]);
        assert_state(&table, 3, Some(1));
        table.sync_rows(rows.len(), None);
        table.push_row(&mut rows, 40);
        assert_state(&table, 4, Some(0));
        rows.clear();
        table.sync_rows(0, None);
        table.push_row(&mut rows, 50);
        assert_state(&table, 1, Some(0));
    }

    #[test]
    fn render_normalizes_before_content_and_updates_viewport() {
        let mut table = table(20, Some(usize::MAX));
        let mut terminal = Terminal::new(TestBackend::new(30, 6)).unwrap();
        terminal.draw(|frame| table.render_list(frame, frame.area())).unwrap();
        assert_state(&table, 20, Some(19));
        assert!(table.table_state.offset() > 0);
        let buffer = terminal.backend().buffer();
        let last_line: String = (0..buffer.area.width).map(|x| buffer[(x, 5)].symbol()).collect();
        assert!(last_line.contains("row 19"), "selected record must be visible: {last_line:?}");
        table.content_length = 0;
        terminal.draw(|frame| table.render_list(frame, frame.area())).unwrap();
        assert_state(&table, 0, None);
        assert_eq!(table.table_state.offset(), 0);
    }

    #[test]
    fn render_preserves_unselected_mode_and_handles_tiny_areas() {
        let mut table = table(3, None);
        let mut terminal = Terminal::new(TestBackend::new(30, 6)).unwrap();
        terminal.draw(|frame| table.render_list(frame, frame.area())).unwrap();
        assert_state(&table, 3, None);
        for area in [Rect::default(), Rect::new(0, 0, 1, 2), Rect::new(0, 0, 5, 0)] {
            table.table_state.select(Some(usize::MAX));
            terminal.draw(|frame| table.render_list(frame, area)).unwrap();
            assert_state(&table, 3, Some(2));
        }
    }
}
