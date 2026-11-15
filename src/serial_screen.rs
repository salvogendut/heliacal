//! A bounded output-only terminal screen, fed from the same raw bytes as the PTY.
//! Parsing stays on the worker; only changed snapshots cross to SDL at publish time.
use std::sync::Arc;

pub struct SerialScreen {
    parser: vt100::Parser,
    changed: bool,
}

impl Default for SerialScreen {
    fn default() -> Self {
        Self {
            // 80x24, no scrollback. Disabling vt100's std feature also bounds
            // VTE's OSC string buffer to 1024 bytes. No terminal reply callbacks.
            parser: vt100::Parser::default(),
            changed: true,
        }
    }
}

impl SerialScreen {
    pub fn process(&mut self, bytes: &[u8]) {
        if !bytes.is_empty() {
            self.parser.process(bytes);
            self.changed = true;
        }
    }

    pub fn publish(&mut self) -> Option<Arc<vt100::Screen>> {
        if !std::mem::take(&mut self.changed) {
            return None;
        }
        Some(Arc::new(self.parser.screen().clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_progress_and_split_escape_sequences_render_without_literal_controls() {
        let mut terminal = SerialScreen::default();
        terminal.process(b"old text\r\nother row\x1b[2");
        terminal.process(b"J\x1b[HBoot 10%\rBoot 20%\r\nabc\x08Z\tX");
        terminal.process(b"\x1b[4;3H\x1b[31;44;1;4mOK\x1b[0m\x1b[?25l");
        let screen = terminal.publish().unwrap();
        assert_eq!(screen.cell(0, 5).unwrap().contents(), "2");
        assert_eq!(screen.cell(1, 2).unwrap().contents(), "Z");
        assert_eq!(screen.cell(1, 8).unwrap().contents(), "X");
        let cell = screen.cell(3, 2).unwrap();
        assert_eq!(cell.contents(), "O");
        assert_eq!(cell.fgcolor(), vt100::Color::Idx(1));
        assert_eq!(cell.bgcolor(), vt100::Color::Idx(4));
        assert!(cell.bold() && cell.underline());
        assert!(screen.hide_cursor());
        assert_eq!(screen.cursor_position(), (3, 4));
        assert!(!screen.contents().contains("old text"));
        assert!(!screen.contents().contains("[31"));
    }

    #[test]
    fn output_scrolls_wraps_and_snapshots_remain_immutable() {
        let mut terminal = SerialScreen::default();
        for row in 0..30 {
            terminal.process(format!("ROW-{row:02}\r\n").as_bytes());
        }
        let first = terminal.publish().unwrap();
        assert!(first.contents().starts_with("ROW-07\n"));
        assert!(first.contents().contains("ROW-29"));
        assert!(terminal.publish().is_none());
        terminal.process(&[b'x'; 81]);
        let second = terminal.publish().unwrap();
        assert_eq!(second.cursor_position(), (23, 1));
        assert_eq!(second.cell(22, 79).unwrap().contents(), "x");
        assert_eq!(second.cell(23, 0).unwrap().contents(), "x");
        assert!(!first.contents().contains('x'));
        terminal = SerialScreen::default();
        assert!(terminal.publish().unwrap().contents().is_empty());
    }

    #[test]
    fn oversized_terminal_strings_are_ignored_without_changing_screen_geometry() {
        let mut terminal = SerialScreen::default();
        terminal.process(b"hello\x1b]0;");
        terminal.process(&vec![b'x'; 128 * 1024]);
        terminal.process(b"\x07\x1b[9999;9999H!");
        let screen = terminal.publish().unwrap();
        let (rows, cols) = screen.size();
        assert_eq!((rows.get(), cols.get()), (24, 80));
        assert!(screen.contents().starts_with("hello"));
        assert_eq!(screen.cell(23, 79).unwrap().contents(), "!");
    }
}
