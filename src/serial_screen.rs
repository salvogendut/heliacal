//! Bounded terminal history stays on the worker. Only the visible 80x24 cells
//! cross to SDL, so publishing a frame never clones the scrollback buffer.
use std::sync::Arc;

pub const ROWS: u16 = 24;
pub const COLS: u16 = 80;
pub const HISTORY_LINES: usize = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scroll {
    By(i32),
    To(usize),
}
impl Scroll {
    pub fn then(self, next: Self) -> Self {
        match (self, next) {
            (Self::By(a), Self::By(b)) => Self::By(a.saturating_add(b)),
            (Self::To(a), Self::By(b)) => Self::To(a.saturating_add_signed(b as isize)),
            (_, absolute) => absolute,
        }
    }
}

pub struct SerialFrame {
    cells: Box<[vt100::Cell]>,
    pub wrapped: [bool; ROWS as usize],
    pub history: usize,
    pub offset: usize,
    cursor: (u16, u16),
    hidden: bool,
    application_cursor: bool,
    pub bracketed_paste: bool,
    #[cfg(test)]
    contents: String,
}
impl SerialFrame {
    fn capture(screen: &vt100::Screen) -> Self {
        Self {
            cells: (0..ROWS)
                .flat_map(|row| (0..COLS).map(move |col| screen.cell(row, col).unwrap().clone()))
                .collect(),
            wrapped: std::array::from_fn(|row| screen.row_wrapped(row as u16)),
            history: screen.scrollback_len(),
            offset: screen.scrollback(),
            cursor: screen.cursor_position(),
            hidden: screen.hide_cursor() || screen.scrollback() != 0,
            application_cursor: screen.application_cursor(),
            bracketed_paste: screen.bracketed_paste(),
            #[cfg(test)]
            contents: screen.contents(),
        }
    }
    pub fn cell(&self, row: u16, col: u16) -> Option<&vt100::Cell> {
        if row >= ROWS || col >= COLS {
            return None;
        }
        self.cells
            .get(usize::from(row) * usize::from(COLS) + usize::from(col))
    }
    pub fn cursor_position(&self) -> (u16, u16) {
        self.cursor
    }
    pub fn hide_cursor(&self) -> bool {
        self.hidden
    }
    pub fn application_cursor(&self) -> bool {
        self.application_cursor
    }
    #[cfg(test)]
    pub fn contents(&self) -> &str {
        &self.contents
    }
    #[cfg(test)]
    pub fn size(&self) -> (std::num::NonZeroU16, std::num::NonZeroU16) {
        (ROWS.try_into().unwrap(), COLS.try_into().unwrap())
    }
}

pub struct SerialScreen {
    parser: vt100::Parser,
    changed: bool,
}
impl Default for SerialScreen {
    fn default() -> Self {
        Self {
            // VTE's OSC storage is bounded by disabling vt100's std feature.
            // No terminal reply callbacks or clipboard escape processing.
            parser: vt100::Parser::new(
                vt100::Parser::DEFAULT_ROWS,
                vt100::Parser::DEFAULT_COLS,
                HISTORY_LINES,
            ),
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
    pub fn scroll(&mut self, request: Scroll) -> bool {
        let screen = self.parser.screen_mut();
        let before = screen.scrollback();
        let offset = match request {
            Scroll::By(rows) => before.saturating_add_signed(rows as isize),
            Scroll::To(rows) => rows,
        };
        screen.set_scrollback(offset);
        let changed = screen.scrollback() != before;
        self.changed |= changed;
        changed
    }
    pub fn publish(&mut self) -> Option<Arc<SerialFrame>> {
        if !std::mem::take(&mut self.changed) {
            return None;
        }
        Some(Arc::new(SerialFrame::capture(self.parser.screen())))
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
    #[test]
    fn scrollback_anchors_during_output_and_snapshots_stay_one_page() {
        let mut terminal = SerialScreen::default();
        for n in 0..100 {
            terminal.process(format!("ROW-{n:04}\r\n").as_bytes());
        }
        let live = terminal.publish().unwrap();
        assert_eq!((live.history, live.offset), (77, 0));
        assert!(live.contents().starts_with("ROW-0077"));
        assert!(terminal.scroll(Scroll::By(30)));
        let old = terminal.publish().unwrap();
        assert!(old.contents().starts_with("ROW-0047"));
        terminal.process(b"ROW-0100\r\nROW-0101\r\n");
        let anchored = terminal.publish().unwrap();
        assert_eq!(anchored.offset, 32);
        assert_eq!(anchored.contents(), old.contents());
        assert!(anchored.hide_cursor());
        terminal.scroll(Scroll::To(0));
        let live_again = terminal.publish().unwrap();
        assert!(live_again.contents().contains("ROW-0101"));
        assert!(!live_again.hide_cursor());
        assert_eq!(old.cells.len(), 80 * 24);
        assert_eq!(anchored.cells.len(), 80 * 24);
        assert_eq!(live_again.cells.len(), 80 * 24);
    }
    #[test]
    fn history_capacity_alt_screen_erasure_and_reset_are_bounded() {
        let mut terminal = SerialScreen::default();
        for n in 0..HISTORY_LINES + 40 {
            terminal.process(format!("{n}\r\n").as_bytes());
        }
        terminal.scroll(Scroll::To(usize::MAX));
        let first = terminal.publish().unwrap();
        assert_eq!(
            (first.history, first.offset),
            (HISTORY_LINES, HISTORY_LINES)
        );
        assert!(first.contents().starts_with("17\n"));
        terminal.process(b"new\r\n");
        let overflow = terminal.publish().unwrap();
        assert_eq!(overflow.history, HISTORY_LINES);
        assert!(overflow.contents().starts_with("18\n"));
        assert_eq!(overflow.cells.len(), 80 * 24);
        terminal.process(b"\x1b[?1049hALT\r\n");
        let alt = terminal.publish().unwrap();
        assert_eq!((alt.history, alt.offset), (0, 0));
        assert!(!terminal.scroll(Scroll::By(10)));
        terminal.process(b"\x1b[?1049l");
        assert_eq!(terminal.publish().unwrap().history, HISTORY_LINES);
        terminal.process(b"\x1b[3J");
        let clear = terminal.publish().unwrap();
        assert_eq!((clear.history, clear.offset), (0, 0));
        terminal.process(b"\x1bc");
        let reset = terminal.publish().unwrap();
        assert_eq!((reset.history, reset.offset), (0, 0));
        assert!(reset.contents().is_empty());
    }
    #[test]
    fn coalesced_scroll_input_preserves_relative_moves_and_live_jump() {
        assert_eq!(Scroll::By(3).then(Scroll::By(6)), Scroll::By(9));
        assert_eq!(Scroll::To(40).then(Scroll::By(-50)), Scroll::To(0));
        assert_eq!(Scroll::By(50).then(Scroll::To(0)), Scroll::To(0));
    }
}
