use sdl3::keyboard::Scancode;

/// Physical Sun Type 5 US key stations, not host Unicode/terminal bytes.
pub fn station(key: Scancode) -> Option<u8> {
    use Scancode::*;
    Some(match key {
        Pause => 0x01,
        F1 => 5,
        F2 => 6,
        F3 => 8,
        F4 => 10,
        F5 => 12,
        F6 => 14,
        F7 => 16,
        F8 => 17,
        F9 => 18,
        F10 => 7,
        F11 => 9,
        F12 => 11,
        Escape => 0x1d,
        Scancode::_1 => 0x1e,
        Scancode::_2 => 0x1f,
        Scancode::_3 => 0x20,
        Scancode::_4 => 0x21,
        Scancode::_5 => 0x22,
        Scancode::_6 => 0x23,
        Scancode::_7 => 0x24,
        Scancode::_8 => 0x25,
        Scancode::_9 => 0x26,
        Scancode::_0 => 0x27,
        Minus => 0x28,
        Equals => 0x29,
        Grave => 0x2a,
        Backspace => 0x2b,
        Tab => 0x35,
        Q => 0x36,
        W => 0x37,
        E => 0x38,
        R => 0x39,
        T => 0x3a,
        Y => 0x3b,
        U => 0x3c,
        I => 0x3d,
        O => 0x3e,
        P => 0x3f,
        LeftBracket => 0x40,
        RightBracket => 0x41,
        Delete => 0x42,
        Backslash => 0x58,
        LCtrl | RCtrl => 0x4c,
        A => 0x4d,
        S => 0x4e,
        D => 0x4f,
        F => 0x50,
        G => 0x51,
        H => 0x52,
        J => 0x53,
        K => 0x54,
        L => 0x55,
        Semicolon => 0x56,
        Apostrophe => 0x57,
        Return => 0x59,
        LShift => 0x63,
        RShift => 0x6e,
        Z => 0x64,
        X => 0x65,
        C => 0x66,
        V => 0x67,
        B => 0x68,
        N => 0x69,
        M => 0x6a,
        Comma => 0x6b,
        Period => 0x6c,
        Slash => 0x6d,
        CapsLock => 0x77,
        LGui => 0x78,
        RGui => 0x7a,
        Space => 0x79,
        LAlt => 0x13,
        RAlt => 0x0d,
        Kp0 => 0x5e,
        Kp1 => 0x70,
        Kp2 => 0x71,
        Kp3 => 0x72,
        Kp4 => 0x5b,
        Kp5 => 0x5c,
        Kp6 => 0x5d,
        Kp7 => 0x44,
        Kp8 => 0x45,
        Kp9 => 0x46,
        KpPeriod => 0x32,
        KpDivide => 0x2e,
        KpMultiply => 0x2f,
        KpMinus => 0x47,
        KpPlus => 0x7d,
        KpEnter => 0x5a,
        NumLockClear => 0x62,
        ScrollLock => 0x17,
        Application => 0x43,
        Up => 0x14,
        Down => 0x1b,
        Left => 0x18,
        Right => 0x1c,
        Home => 0x34,
        End => 0x4a,
        PageUp => 0x60,
        PageDown => 0x7b,
        Insert => 0x2c,
        F13 => 0x01,
        F14 => 0x03,
        F15 => 0x76,
        _ => return None,
    })
}

#[derive(Default)]
pub struct HeldKeys(std::collections::HashSet<Scancode>);
impl HeldKeys {
    pub fn event(&mut self, key: Scancode, down: bool) -> Option<(u8, bool)> {
        let code = station(key)?;
        let before = self.0.iter().any(|&k| station(k) == Some(code));
        if down {
            self.0.insert(key);
        } else {
            self.0.remove(&key);
        }
        let after = self.0.iter().any(|&k| station(k) == Some(code));
        (before != after).then_some((code, after))
    }
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

/// Graphical consoles receive physical key presses, never serial text. The
/// emulated keyboard advertises the US layout (0x21); reject characters it
/// cannot produce before enqueueing any part of a paste.
pub fn paste_events(text: &str) -> Result<std::collections::VecDeque<(u8, bool)>, String> {
    let bytes = crate::serial_input::paste_bytes(text, false)?;
    if !bytes.is_ascii() {
        return Err("Graphical paste supports ASCII with the guest US keyboard layout.".into());
    }
    use Scancode::*;
    let letters = [
        Q, W, E, R, T, Y, U, I, O, P, A, S, D, F, G, H, J, K, L, Z, X, C, V, B, N, M,
    ];
    let symbols = [
        Grave,
        _1,
        _2,
        _3,
        _4,
        _5,
        _6,
        _7,
        _8,
        _9,
        _0,
        Minus,
        Equals,
        LeftBracket,
        RightBracket,
        Backslash,
        Semicolon,
        Apostrophe,
        Comma,
        Period,
        Slash,
    ];
    let mut events = std::collections::VecDeque::new();
    for byte in bytes {
        let (key, shift) = if byte.is_ascii_alphabetic() {
            let index = b"qwertyuiopasdfghjklzxcvbnm"
                .iter()
                .position(|b| *b == byte.to_ascii_lowercase())
                .unwrap();
            (letters[index], byte.is_ascii_uppercase())
        } else if let Some(index) = b"`1234567890-=[]\\;',./".iter().position(|b| *b == byte) {
            (symbols[index], false)
        } else if let Some(index) = b"~!@#$%^&*()_+{}|:\"<>?".iter().position(|b| *b == byte) {
            (symbols[index], true)
        } else {
            (
                match byte {
                    b' ' => Space,
                    b'\t' => Tab,
                    b'\r' => Return,
                    _ => return Err("Unsupported graphical paste character.".into()),
                },
                false,
            )
        };
        if shift {
            events.push_back((0x63, true));
        }
        let code = station(key).unwrap();
        events.extend([(code, true), (code, false)]);
        if shift {
            events.push_back((0x63, false));
        }
    }
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_control_station_stays_down_until_both_keys_are_released() {
        let mut keys = HeldKeys::default();
        assert_eq!(keys.event(Scancode::LCtrl, true), Some((0x4c, true)));
        assert_eq!(keys.event(Scancode::RCtrl, true), None);
        assert_eq!(keys.event(Scancode::LCtrl, false), None);
        assert_eq!(keys.event(Scancode::RCtrl, false), Some((0x4c, false)));
    }
    #[test]
    fn repeated_keys_are_suppressed_and_stop_a_is_physical() {
        let mut keys = HeldKeys::default();
        assert_eq!(keys.event(Scancode::Pause, true), Some((1, true)));
        assert_eq!(keys.event(Scancode::A, true), Some((0x4d, true)));
        assert_eq!(keys.event(Scancode::A, true), None);
        keys.clear();
        assert_eq!(keys.event(Scancode::A, false), None);
    }
    #[test]
    fn graphical_paste_generates_balanced_sun_us_keys_and_rejects_unsupported_text() {
        let events = paste_events("A!a\r\n").unwrap();
        assert_eq!(
            events.into_iter().collect::<Vec<_>>(),
            vec![
                (0x63, true),
                (0x4d, true),
                (0x4d, false),
                (0x63, false),
                (0x63, true),
                (0x1e, true),
                (0x1e, false),
                (0x63, false),
                (0x4d, true),
                (0x4d, false),
                (0x59, true),
                (0x59, false),
            ]
        );
        let printable = (32..=126).map(char::from).collect::<String>();
        let mut held = [false; 128];
        for (key, down) in paste_events(&printable).unwrap() {
            assert_ne!(held[key as usize], down);
            held[key as usize] = down;
        }
        assert!(!held.iter().any(|v| *v));
        assert!(paste_events("aé").is_err());
        assert!(paste_events(&"x".repeat(4097)).is_err());
    }
}
