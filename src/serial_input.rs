//! Embedded terminal input. Printable text comes from SDL TextInput, so host
//! keyboard layouts and composition are preserved. Special keys use VT sequences.
use sdl3::keyboard::{Keycode, Mod, Scancode};
use std::collections::VecDeque;

const LIMIT: usize = 4096;

#[derive(Default)]
pub struct SerialInput {
    bytes: VecDeque<u8>,
    generation: u64,
    enabled: bool,
}

impl SerialInput {
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn reset(&mut self) {
        self.clear();
        self.generation = self.generation.wrapping_add(1);
    }
    pub fn enable(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.clear();
        }
    }
    pub fn clear(&mut self) {
        self.bytes.clear();
    }
    pub fn push(&mut self, generation: u64, bytes: &[u8]) -> Result<(), String> {
        if !self.enabled || generation != self.generation {
            return Err("Serial console changed or is not running; input was not sent.".into());
        }
        if bytes.len() > LIMIT - self.bytes.len() {
            return Err("Serial input queue full; wait for the guest, then retry.".into());
        }
        self.bytes.extend(bytes);
        Ok(())
    }
    pub fn front(&self) -> Option<u8> {
        self.bytes.front().copied()
    }
    pub fn consume(&mut self) {
        self.bytes.pop_front();
    }
}

/// These are handled by the frontend, including repeated keydown events.
pub fn reserved(key: Scancode, mods: Mod) -> bool {
    let ctrl = mods.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD)
        && !mods.intersects(Mod::RALTMOD | Mod::MODEMOD | Mod::LEVEL5MOD);
    let shift = mods.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
    (ctrl && matches!(key, Scancode::Return | Scancode::P))
        || (ctrl && shift && matches!(key, Scancode::C | Scancode::V))
        || (!shift
            && matches!(
                key,
                Scancode::F4 | Scancode::F5 | Scancode::F9 | Scancode::F11 | Scancode::F12
            ))
}

/// Return only non-text input here: printable keys must not also be sent by
/// keydown, since SDL sends their composed text separately (including repeats).
pub fn key_bytes(
    key: Scancode,
    keycode: Option<Keycode>,
    mods: Mod,
    application_cursor: bool,
) -> Option<Vec<u8>> {
    let ctrl = mods.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD)
        && !mods.intersects(Mod::RALTMOD | Mod::MODEMOD | Mod::LEVEL5MOD);
    let shift = mods.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
    if ctrl {
        let value = match keycode.map(|k| k as u32) {
            Some(n @ (0x40..=0x5f | 0x61..=0x7a)) => Some((n & 31) as u8),
            Some(0x20 | 0x32) => Some(0), // Ctrl+Space / Ctrl+2
            Some(0x36) => Some(30),       // Ctrl+6
            Some(0x2d) => Some(31),       // Ctrl+-
            Some(0x3f) => Some(127),      // Ctrl+?
            _ => None,
        };
        if let Some(byte) = value {
            return Some(vec![byte]);
        }
    }
    use Scancode::*;
    let bytes: &[u8] = match key {
        Return | KpEnter => b"\r",
        Backspace => b"\x7f",
        Tab if shift => b"\x1b[Z",
        Tab => b"\t",
        Escape => b"\x1b",
        Up | Down | Right | Left | Home | End => {
            let last = match key {
                Up => 'A',
                Down => 'B',
                Right => 'C',
                Left => 'D',
                Home => 'H',
                _ => 'F',
            };
            let modifier = 1 + u8::from(shift) + 4 * u8::from(ctrl);
            return Some(if modifier != 1 {
                format!("\x1b[1;{modifier}{last}").into_bytes()
            } else {
                format!("\x1b{}{last}", if application_cursor { 'O' } else { '[' }).into_bytes()
            });
        }
        Insert => b"\x1b[2~",
        Delete => b"\x1b[3~",
        PageUp => b"\x1b[5~",
        PageDown => b"\x1b[6~",
        F1 => b"\x1bOP",
        F2 => b"\x1bOQ",
        F3 => b"\x1bOR",
        F4 => b"\x1bOS",
        F5 => b"\x1b[15~",
        F6 => b"\x1b[17~",
        F7 => b"\x1b[18~",
        F8 => b"\x1b[19~",
        F9 => b"\x1b[20~",
        F10 => b"\x1b[21~",
        F11 => b"\x1b[23~",
        F12 => b"\x1b[24~",
        _ => return None,
    };
    Some(bytes.to_vec())
}

/// Paste text, rather than terminal controls. Newlines become TTY Enter and
/// CRLF is a single Enter. Keep tabs and UTF-8; omit other C0/C1 controls.
/// Bracketed-paste delimiters are sent only when the guest has requested them.
pub fn paste_bytes(text: &str, bracketed: bool) -> Result<Vec<u8>, String> {
    if text.len() > LIMIT {
        return Err("Paste is too large; paste at most 4 KiB at a time.".into());
    }
    let mut result = Vec::new();
    if bracketed {
        result.extend(b"\x1b[200~");
    }
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                result.push(b'\r');
            }
            '\n' => result.push(b'\r'),
            '\t' => result.push(b'\t'),
            ch if !ch.is_control() => {
                let mut bytes = [0; 4];
                result.extend_from_slice(ch.encode_utf8(&mut bytes).as_bytes());
            }
            _ => {}
        }
    }
    if bracketed {
        result.extend(b"\x1b[201~");
    }
    if result.len() > LIMIT {
        return Err("Paste is too large; leave room for terminal paste delimiters.".into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn control_keys_and_application_cursor_mode_have_distinct_bytes() {
        assert_eq!(
            key_bytes(Scancode::C, Some(Keycode::C), Mod::LCTRLMOD, false),
            Some(vec![3])
        );
        // Logical keycodes, rather than a forced US physical-key layout.
        assert_eq!(
            key_bytes(Scancode::Q, Some(Keycode::A), Mod::LCTRLMOD, false),
            Some(vec![1])
        );
        assert_eq!(
            key_bytes(Scancode::Return, Some(Keycode::Return), Mod::NOMOD, false),
            Some(vec![13])
        );
        assert_eq!(
            key_bytes(Scancode::Backspace, None, Mod::NOMOD, false),
            Some(vec![127])
        );
        assert_eq!(
            key_bytes(Scancode::H, Some(Keycode::H), Mod::LCTRLMOD, false),
            Some(vec![8])
        );
        assert_eq!(
            key_bytes(Scancode::Up, None, Mod::NOMOD, false),
            Some(b"\x1b[A".to_vec())
        );
        assert_eq!(
            key_bytes(Scancode::Up, None, Mod::NOMOD, true),
            Some(b"\x1bOA".to_vec())
        );
        assert_eq!(
            key_bytes(Scancode::Left, None, Mod::LCTRLMOD, true),
            Some(b"\x1b[1;5D".to_vec())
        );
        assert_eq!(
            key_bytes(Scancode::Tab, None, Mod::LSHIFTMOD, false),
            Some(b"\x1b[Z".to_vec())
        );
        assert_eq!(
            key_bytes(Scancode::A, Some(Keycode::A), Mod::NOMOD, false),
            None
        );
        // AltGr composition belongs to TextInput, not Ctrl-key injection.
        assert_eq!(
            key_bytes(
                Scancode::Q,
                Some(Keycode::Q),
                Mod::LCTRLMOD | Mod::RALTMOD,
                false
            ),
            None
        );
        assert!(!reserved(Scancode::P, Mod::LCTRLMOD | Mod::RALTMOD));
    }
    #[test]
    fn clipboard_shortcut_does_not_replace_guest_control_c() {
        assert!(!reserved(Scancode::C, Mod::LCTRLMOD));
        assert!(reserved(Scancode::C, Mod::LCTRLMOD | Mod::LSHIFTMOD));
        assert!(reserved(Scancode::Return, Mod::LCTRLMOD));
        assert!(reserved(Scancode::F9, Mod::NOMOD));
        assert!(!reserved(Scancode::F9, Mod::LSHIFTMOD));
    }
    #[test]
    fn queue_is_bounded_atomic_and_rejects_input_from_a_previous_session() {
        let mut input = SerialInput::default();
        assert!(input.push(0, b"no machine").is_err());
        input.enable(true);
        input.push(0, b"first").unwrap();
        assert!(input.push(0, &[0; LIMIT]).is_err());
        for byte in b"first" {
            assert_eq!(input.front(), Some(*byte));
            input.consume();
        }
        input.push(0, b"old command\r").unwrap();
        input.reset();
        assert!(input.front().is_none());
        assert!(input.push(0, b"late command").is_err());
        input.push(input.generation(), b"new").unwrap();
        input.enable(false);
        assert!(input.front().is_none());
        assert!(input.push(input.generation(), b"paused").is_err());
    }
    #[test]
    fn paste_normalizes_newlines_filters_controls_and_honors_bracketed_mode() {
        assert_eq!(
            paste_bytes("a\r\nb\nc\rd\t\x03é", false).unwrap(),
            "a\rb\rc\rd\té".as_bytes()
        );
        assert_eq!(
            paste_bytes("a\nb", true).unwrap(),
            b"\x1b[200~a\rb\x1b[201~"
        );
        assert!(paste_bytes(&"x".repeat(4097), false).is_err());
        assert!(paste_bytes(&"x".repeat(4096), true).is_err());
        assert!(reserved(Scancode::V, Mod::LCTRLMOD | Mod::LSHIFTMOD));
        assert!(!reserved(Scancode::V, Mod::LCTRLMOD));
        assert_eq!(
            key_bytes(Scancode::V, Some(Keycode::V), Mod::LCTRLMOD, false),
            Some(vec![22])
        );
    }
}
