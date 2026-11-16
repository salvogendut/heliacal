//! A selection owns one immutable viewport; arriving output cannot change what
//! the user is highlighting or copies. Scrolling/typing returns to current output.
use crate::serial_screen::{COLS, ROWS, SerialFrame};
use std::sync::Arc;

pub struct Selection {
    pub screen: Arc<SerialFrame>,
    pub start: (u16, u16),
    pub end: (u16, u16),
    pub dragging: bool,
}
impl Selection {
    pub fn new(screen: Arc<SerialFrame>, at: (u16, u16)) -> Self {
        Self {
            screen,
            start: at,
            end: at,
            dragging: true,
        }
    }
    pub fn nonempty(&self) -> bool {
        self.start != self.end
    }
    pub fn contains(&self, row: u16, col: u16) -> bool {
        self.nonempty()
            && self.start.min(self.end) <= (row, col)
            && (row, col) <= self.start.max(self.end)
    }
    pub fn text(&self) -> String {
        if !self.nonempty() {
            return String::new();
        }
        let (start, end) = (self.start.min(self.end), self.start.max(self.end));
        let mut result = String::new();
        for row in start.0..=end.0.min(ROWS - 1) {
            let first = if row == start.0 { start.1 } else { 0 };
            let last = if row == end.0 { end.1 } else { COLS - 1 };
            let mut line = String::new();
            for col in first..=last.min(COLS - 1) {
                let cell = self.screen.cell(row, col).unwrap();
                if cell.is_wide_continuation() {
                    continue;
                }
                if cell.has_contents() {
                    line.push_str(cell.contents());
                } else {
                    line.push(' ');
                }
            }
            result.push_str(line.trim_end_matches(' '));
            if row != end.0 && !self.screen.wrapped[usize::from(row)] {
                result.push('\n');
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serial_screen::SerialScreen;
    #[test]
    fn reversed_multiline_selection_keeps_text_without_ansi_or_padding() {
        let mut terminal = SerialScreen::default();
        terminal.process(b"\x1b[31mfirst row\x1b[0m\r\nsecond row");
        let frame = terminal.publish().unwrap();
        let mut selected = Selection::new(frame, (1, 5));
        selected.end = (0, 6);
        assert_eq!(selected.text(), "row\nsecond");
        assert!(selected.contains(0, 79));
        assert!(!selected.contains(0, 5));
        assert!(!selected.contains(1, 6));
        terminal.process(b"changed later");
        assert_eq!(selected.text(), "row\nsecond");
    }
    #[test]
    fn selection_joins_soft_wraps_and_preserves_unicode_cells() {
        let mut terminal = SerialScreen::default();
        terminal.process(&[b'x'; 81]);
        terminal.process("é".as_bytes());
        let mut selection = Selection::new(terminal.publish().unwrap(), (0, 78));
        selection.end = (1, 1);
        assert_eq!(selection.text(), "xxxé");
    }
}
