use sdl3::{
    keyboard::{Keycode, Mod, Scancode},
    video::{FullscreenType, Window, WindowPos},
};
use std::collections::HashSet;

pub const MINIMUM: (u32, u32) = (720, 570);

/// Match host symbols, including non-US layouts, while leaving AltGr alone.
pub fn shortcut(scan: Scancode, key: Option<Keycode>, mods: Mod) -> Option<bool> {
    if !mods.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD)
        || mods.intersects(
            Mod::LALTMOD
                | Mod::RALTMOD
                | Mod::LGUIMOD
                | Mod::RGUIMOD
                | Mod::MODEMOD
                | Mod::LEVEL5MOD,
        )
    {
        return None;
    }
    match key {
        Some(Keycode::Plus | Keycode::Equals | Keycode::KpPlus) => Some(true),
        Some(Keycode::Minus | Keycode::Underscore | Keycode::KpMinus) => Some(false),
        None => match scan {
            Scancode::Equals | Scancode::KpPlus => Some(true),
            Scancode::Minus | Scancode::KpMinus => Some(false),
            _ => None,
        },
        _ => None,
    }
}

#[derive(Default)]
pub struct Resizer {
    pending: Option<(u32, u32)>,
    held: HashSet<Scancode>,
}

impl Resizer {
    pub fn key_down(&mut self, key: Scancode) {
        self.held.insert(key);
    }

    pub fn key_up(&mut self, key: Scancode) {
        self.held.remove(&key);
    }

    pub fn focus_lost(&mut self) {
        self.held.clear();
    }

    pub fn suppress_text(&self) -> bool {
        !self.held.is_empty()
    }

    pub fn request(&mut self, window: &Window, increase: bool) -> Result<(), String> {
        if window.fullscreen_state() != FullscreenType::Off {
            return Ok(());
        }
        let bounds = window
            .get_display()
            .and_then(|d| d.get_usable_bounds())
            .map_err(|e| e.to_string())?;
        let (top, left, bottom, right) = window.border_size().unwrap_or_default();
        let maximum = (
            bounds
                .width()
                .saturating_sub(u32::from(left) + u32::from(right)),
            bounds
                .height()
                .saturating_sub(u32::from(top) + u32::from(bottom)),
        );
        if let Some(size) = next_size(
            self.pending.unwrap_or_else(|| window.size()),
            maximum,
            increase,
        ) {
            self.pending = Some(size);
        }
        Ok(())
    }

    /// Restore maximized windows asynchronously, then apply the requested size.
    pub fn apply(&mut self, window: &mut Window) -> Result<bool, String> {
        let Some((width, height)) = self.pending else {
            return Ok(false);
        };
        if window.fullscreen_state() != FullscreenType::Off {
            self.pending = None;
            return Ok(false);
        }
        if window.is_minimized() {
            return Ok(false);
        }
        if window.is_maximized() {
            if !window.restore() {
                self.pending = None;
                return Err(sdl3::get_error().to_string());
            }
            return Ok(false);
        }
        self.pending = None;
        window.set_size(width, height).map_err(|e| e.to_string())?;
        // Keep the enlarged window inside its current display's work area.
        if let Ok(bounds) = window.get_display().and_then(|d| d.get_usable_bounds()) {
            let (top, left, bottom, right) = window.border_size().unwrap_or_default();
            let (x, y) = window.position();
            let min_x = bounds.x() + i32::from(left);
            let min_y = bounds.y() + i32::from(top);
            let max_x = (bounds.right() - width as i32 - i32::from(right)).max(min_x);
            let max_y = (bounds.bottom() - height as i32 - i32::from(bottom)).max(min_y);
            let (next_x, next_y) = (x.clamp(min_x, max_x), y.clamp(min_y, max_y));
            if (x, y) != (next_x, next_y) {
                window.set_position(WindowPos::Positioned(next_x), WindowPos::Positioned(next_y));
            }
        }
        Ok(true)
    }
}

fn next_size(current: (u32, u32), maximum: (u32, u32), increase: bool) -> Option<(u32, u32)> {
    let (width, height) = current;
    if width == 0 || height == 0 {
        return None;
    }
    let scale = if increase {
        1.25_f64
            .min(f64::from(maximum.0) / f64::from(width))
            .min(f64::from(maximum.1) / f64::from(height))
    } else {
        0.8_f64
            .max(f64::from(MINIMUM.0) / f64::from(width))
            .max(f64::from(MINIMUM.1) / f64::from(height))
    };
    if (increase && scale <= 1.0) || (!increase && scale >= 1.0) {
        return None;
    }
    let next = (
        (f64::from(width) * scale).round() as u32,
        (f64::from(height) * scale).round() as u32,
    );
    (next != current).then_some(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_layout_keypad_and_altgr_are_distinguished() {
        for (scan, key, increase) in [
            (Scancode::Equals, Keycode::Equals, true),
            (Scancode::Equals, Keycode::Plus, true),
            (Scancode::RightBracket, Keycode::Plus, true),
            (Scancode::KpPlus, Keycode::KpPlus, true),
            (Scancode::Minus, Keycode::Minus, false),
            (Scancode::Minus, Keycode::Underscore, false),
            (Scancode::KpMinus, Keycode::KpMinus, false),
        ] {
            for ctrl in [Mod::LCTRLMOD, Mod::RCTRLMOD | Mod::LSHIFTMOD] {
                assert_eq!(shortcut(scan, Some(key), ctrl), Some(increase));
                for extra in [
                    Mod::RALTMOD,
                    Mod::LALTMOD,
                    Mod::LGUIMOD,
                    Mod::MODEMOD,
                    Mod::LEVEL5MOD,
                ] {
                    assert_eq!(shortcut(scan, Some(key), ctrl | extra), None);
                }
            }
            assert_eq!(shortcut(scan, Some(key), Mod::NOMOD), None);
        }
        assert_eq!(
            shortcut(Scancode::Equals, Some(Keycode::A), Mod::LCTRLMOD),
            None
        );
        assert_eq!(shortcut(Scancode::KpPlus, None, Mod::RCTRLMOD), Some(true));
    }

    #[test]
    fn resizing_preserves_proportions_and_direction_at_limits() {
        assert_eq!(next_size((960, 760), (1920, 1080), true), Some((1200, 950)));
        assert_eq!(
            next_size((1200, 950), (1920, 1080), false),
            Some((960, 760))
        );
        assert_eq!(next_size((800, 600), (1920, 1080), false), Some((760, 570)));
        assert_eq!(next_size((760, 570), (1920, 1080), false), None);
        assert_eq!(
            next_size((1200, 950), (1280, 1000), true),
            Some((1263, 1000))
        );
        assert_eq!(next_size((1263, 1000), (1280, 1000), true), None);
        assert_eq!(next_size((2000, 1200), (1280, 1000), true), None);
        assert_eq!(next_size((0, 0), (1280, 1000), true), None);
        assert_eq!(
            next_size((1600, 700), (1920, 1080), true),
            Some((1920, 840))
        );
    }

    #[test]
    fn consumed_chords_suppress_text_until_release_or_focus_loss() {
        let mut resizer = Resizer::default();
        resizer.key_down(Scancode::Equals);
        resizer.key_down(Scancode::KpMinus);
        resizer.key_up(Scancode::Equals);
        assert!(resizer.suppress_text());
        resizer.key_up(Scancode::KpMinus);
        assert!(!resizer.suppress_text());
        resizer.key_down(Scancode::Equals);
        resizer.focus_lost();
        assert!(!resizer.suppress_text());
    }
}
