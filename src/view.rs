use crate::{
    emulator::{Phase, Snapshot},
    overlay::{Confirm, Field, Overlay, Tab},
    settings::Settings,
};
use sdl3::{
    pixels::Color,
    rect::Rect,
    render::{BlendMode, Texture, WindowCanvas},
};

pub const WIDTH: u32 = 960;
pub const HEIGHT: u32 = 760;
const PANEL: Color = Color::RGB(25, 32, 52);
const BORDER: Color = Color::RGB(137, 163, 203);
const TEXT: Color = Color::RGB(240, 240, 240);
const MUTED: Color = Color::RGB(166, 181, 205);
const ACCENT: Color = Color::RGB(239, 181, 81);

fn text(
    c: &mut WindowCanvas,
    x: i32,
    y: i32,
    value: &str,
    color: Color,
) -> Result<(), sdl3::Error> {
    c.set_draw_color(color);
    let ascii: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii() && !ch.is_ascii_control() {
                ch
            } else {
                '?'
            }
        })
        .collect();
    c.draw_debug_text(&ascii, (x as f32, y as f32))
}
fn fill(c: &mut WindowCanvas, rect: Rect, color: Color) -> Result<(), sdl3::Error> {
    c.set_draw_color(color);
    c.fill_rect(rect)
}
fn panel(c: &mut WindowCanvas, rect: Rect) -> Result<(), sdl3::Error> {
    fill(c, rect, PANEL)?;
    c.set_draw_color(BORDER);
    c.draw_rect(rect)
}
pub fn guest_rect() -> Rect {
    Rect::new(26, 0, 908, 710)
}

pub struct Presentation<'a> {
    pub settings: &'a Settings,
    pub state: &'a Snapshot,
    pub overlay: Option<&'a Overlay>,
    pub confirm: Option<(Confirm, bool)>,
    pub captured: bool,
    pub notice: Option<&'a str>,
    pub quitting: bool,
}

pub fn draw(
    c: &mut WindowCanvas,
    guest: &Texture,
    logo: &Texture,
    p: Presentation<'_>,
) -> Result<(), sdl3::Error> {
    c.set_draw_color(Color::RGB(5, 8, 14));
    c.clear();
    c.set_blend_mode(BlendMode::Blend);
    if p.state.has_machine {
        c.copy(guest, None, guest_rect())?;
        if p.settings.scanlines {
            for y in (0..710).step_by(3) {
                fill(c, Rect::new(26, y, 908, 1), Color::RGBA(0, 0, 0, 65))?;
            }
        }
    } else {
        let info = logo.query();
        let h = (260.0 * info.height as f32 / info.width as f32) as u32;
        c.copy(logo, None, Rect::new(350, 90, 260, h))?;
        text(c, 392, 390, "H E L I A C A L", TEXT)?;
        text(c, 368, 418, "Happy Years / Sun SPARC", BORDER)?;
        text(
            c,
            304,
            470,
            "Sun SPARCstation 5  /  microSPARC-II 85 MHz",
            TEXT,
        )?;
        text(c, 336, 494, "128 MiB RAM  /  TurboGX graphics", MUTED)?;
        text(
            c,
            288,
            552,
            "F9  Select your Sun PROM and TurboGX ROM",
            ACCENT,
        )?;
        text(c, 352, 578, "Ctrl+P  Boot the workstation", MUTED)?;
    }
    if p.settings.show_status {
        text(
            c,
            24,
            718,
            "Ctrl+P Boot/pause   F4 Capture   F5 Reset   F9 Options   F11 Fullscreen   F12 Quit",
            MUTED,
        )?;
    }
    fill(c, Rect::new(0, 738, WIDTH, 22), PANEL)?;
    fill(
        c,
        Rect::new(14, 745, 8, 8),
        if p.state.phase == Phase::Running {
            Color::RGB(100, 210, 130)
        } else {
            ACCENT
        },
    )?;
    text(
        c,
        32,
        746,
        if p.quitting {
            "SAVING NVRAM..."
        } else {
            p.state.phase.label()
        },
        TEXT,
    )?;
    text(c, 300, 746, "SS-5  |  128 MiB  |  CGsix", MUTED)?;
    text(
        c,
        638,
        746,
        if p.captured {
            "KEYBOARD CAPTURED / Ctrl+Enter"
        } else {
            "Click display to capture keyboard"
        },
        BORDER,
    )?;
    if let Some(o) = p.overlay {
        draw_overlay(c, o, p.state)?;
    }
    if let Some((confirm, yes)) = p.confirm {
        fill(c, Rect::new(0, 0, WIDTH, HEIGHT), Color::RGBA(0, 0, 0, 170))?;
        panel(c, Rect::new(220, 300, 520, 154))?;
        text(c, 244, 326, confirm.text(), TEXT)?;
        if matches!(confirm, Confirm::Quit | Confirm::Reset) {
            text(
                c,
                244,
                352,
                "Shut down the guest OS first if it is running.",
                MUTED,
            )?;
        }
        text(
            c,
            308,
            400,
            if yes {
                "[ Yes ]       Cancel"
            } else {
                "  Yes       [ Cancel ]"
            },
            ACCENT,
        )?;
        text(
            c,
            244,
            430,
            "Left/Right select   Enter confirm   Escape cancel",
            MUTED,
        )?;
    }
    if let Some(notice) = p.notice.or(p.state.error.as_deref()) {
        panel(c, Rect::new(24, 654, 912, 52))?;
        for (i, line) in notice
            .chars()
            .collect::<Vec<_>>()
            .chunks(108)
            .take(2)
            .enumerate()
        {
            text(
                c,
                40,
                667 + i as i32 * 16,
                &line.iter().collect::<String>(),
                ACCENT,
            )?;
        }
    }
    c.present();
    Ok(())
}

fn draw_overlay(c: &mut WindowCanvas, o: &Overlay, state: &Snapshot) -> Result<(), sdl3::Error> {
    fill(c, Rect::new(0, 0, WIDTH, 710), Color::RGBA(0, 0, 0, 150))?;
    panel(c, Rect::new(112, 100, 736, 540))?;
    text(c, 136, 122, "HELIACAL / OPTIONS", TEXT)?;
    text(c, 568, 122, "SPARCstation 5 / 128 MiB", MUTED)?;
    for (i, tab) in o.tabs().iter().enumerate() {
        let x = 136 + i as i32 * 136;
        if *tab == o.tab {
            fill(c, Rect::new(x - 8, 150, 124, 28), Color::RGB(128, 96, 32))?;
        }
        text(c, x, 160, tab.label(), TEXT)?;
    }
    for (i, row) in o.rows().iter().enumerate() {
        let y = 202 + i as i32 * 32;
        if i == o.selected {
            fill(c, Rect::new(128, y - 6, 704, 28), Color::RGB(57, 60, 90))?;
        }
        let color = if row.field == Field::Fixed {
            MUTED
        } else {
            TEXT
        };
        text(c, 144, y + 3, row.label, color)?;
        let (value_x, limit) = if let Field::Scsi(id) = row.field {
            text(c, 280, y + 3, o.draft.scsi[id].kind.label(), ACCENT)?;
            (420, 48)
        } else {
            (396, 51)
        };
        let value = if i == o.selected {
            o.edit.as_deref().unwrap_or(&row.value)
        } else {
            &row.value
        };
        let chars: Vec<_> = value.chars().collect();
        let tail: String = if chars.len() > limit {
            format!(
                "...{}",
                chars[chars.len() - (limit - 3)..]
                    .iter()
                    .collect::<String>()
            )
        } else {
            value.into()
        };
        text(
            c,
            value_x,
            y + 3,
            &tail,
            if o.edit.is_some() && i == o.selected {
                ACCENT
            } else {
                color
            },
        )?;
    }
    match o.tab {
        Tab::Advanced => {
            text(
                c,
                144,
                300,
                &format!("Helios / PC {:08x} / {} steps", state.pc, state.steps),
                BORDER,
            )?;
            text(
                c,
                144,
                324,
                "Serial A output (recent printable text):",
                MUTED,
            )?;
            let lines: Vec<_> = state.serial.lines().rev().take(11).collect();
            for (i, line) in lines.iter().rev().enumerate() {
                text(
                    c,
                    144,
                    348 + i as i32 * 18,
                    &line.chars().take(82).collect::<String>(),
                    TEXT,
                )?;
            }
        }
        Tab::About => {
            for (i, line) in [
                concat!("Heliacal ", env!("CARGO_PKG_VERSION"), " / Happy Years"),
                "Native Rust + SDL3 frontend for the Helios SPARC emulator.",
                "Sun SPARCstation 5 / sun4m / SPARC V8 / 128 MiB RAM.",
                "Interface conventions follow Prismatic and the Happy Years family.",
                "Sun Type 5 US keyboard: Pause + A sends Stop-A.",
                "Guest mouse, audio and host networking are not connected.",
                "Firmware and operating systems are not bundled.",
            ]
            .iter()
            .enumerate()
            {
                text(
                    c,
                    144,
                    320 + i as i32 * 26,
                    line,
                    if i == 0 { ACCENT } else { MUTED },
                )?;
            }
        }
        Tab::Media => {
            text(
                c,
                144,
                510,
                "SCSI ID 7: host adapter (reserved). All images are read-only.",
                MUTED,
            )?;
            text(
                c,
                144,
                534,
                "Click the device type or press Space to select disk / CD-ROM / off.",
                MUTED,
            )?;
        }
        Tab::Display => {
            text(
                c,
                144,
                428,
                "Display settings apply after Save. Guest timing is unchanged.",
                MUTED,
            )?;
        }
        Tab::General => {}
    }
    text(
        c,
        144,
        572,
        if o.edit.is_some() {
            "Type path / Ctrl+V paste / Ctrl+U clear / Enter accept / Esc cancel"
        } else if o.tab == Tab::Media {
            "Space type / Enter image path / Ctrl+B browse / Delete detach"
        } else {
            "Arrows navigate / Enter edit / Ctrl+B browse / Delete detach"
        },
        MUTED,
    )?;
    text(
        c,
        144,
        602,
        "F9 Save   Esc Close   Firmware and media apply on next cold reset.",
        BORDER,
    )?;
    Ok(())
}

pub fn point(x: f32, y: f32, window: (u32, u32)) -> (i32, i32) {
    let scale = (window.0 as f32 / WIDTH as f32).min(window.1 as f32 / HEIGHT as f32);
    if scale <= 0.0 {
        return (-1, -1);
    }
    (
        ((x - (window.0 as f32 - WIDTH as f32 * scale) / 2.0) / scale) as i32,
        ((y - (window.1 as f32 - HEIGHT as f32 * scale) / 2.0) / scale) as i32,
    )
}
