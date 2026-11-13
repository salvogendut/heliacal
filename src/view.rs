use crate::{
    activity,
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
const LED_HEIGHT: u32 = 22;
const HINT_HEIGHT: u32 = 16;
const PANEL: Color = Color::RGB(25, 32, 52);
const BORDER: Color = Color::RGB(137, 163, 203);
const TEXT: Color = Color::RGB(240, 240, 240);
const MUTED: Color = Color::RGB(166, 181, 205);
const ACCENT: Color = Color::RGB(239, 181, 81);
const STATUS: Color = Color::RGB(255, 64, 64);
const CONFIRM_SLOT: i32 = 140;
const CONFIRM_Y: i32 = 390;

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
pub fn guest_rect(show_status: bool) -> Rect {
    let height = HEIGHT - LED_HEIGHT - if show_status { HINT_HEIGHT } else { 0 };
    let scale = (WIDTH as f32 / 1152.0).min(height as f32 / 900.0);
    let width = (1152.0 * scale) as u32;
    let height = (900.0 * scale) as u32;
    Rect::new(((WIDTH - width) / 2) as i32, 0, width, height)
}

pub struct Presentation<'a> {
    pub settings: &'a Settings,
    pub state: &'a Snapshot,
    pub overlay: Option<&'a Overlay>,
    pub confirm: Option<(Confirm, usize)>,
    pub captured: bool,
    pub leds: u64,
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
    let destination = guest_rect(p.settings.show_status);
    if p.state.has_machine {
        c.copy(guest, None, destination)?;
        if p.settings.scanlines {
            for y in (destination.y()..destination.bottom()).step_by(3) {
                fill(
                    c,
                    Rect::new(destination.x(), y, destination.width(), 1),
                    Color::RGBA(0, 0, 0, 65),
                )?;
            }
        }
    } else {
        let info = logo.query();
        let h = (260.0 * info.height as f32 / info.width as f32) as u32;
        c.copy(logo, None, Rect::new(350, 90, 260, h))?;
        text(c, 392, 390, "H E L I A C A L", TEXT)?;
        text(c, 368, 418, "Happy Years / Sun SPARC", BORDER)?;
        let machine = p.settings.model;
        text(
            c,
            304,
            470,
            &format!("{}  /  {}", machine.label(), machine.processor()),
            TEXT,
        )?;
        text(
            c,
            336,
            494,
            &format!(
                "{} MiB RAM  /  {}",
                p.settings.ram_mib,
                machine.graphics(p.settings.display)
            ),
            MUTED,
        )?;
        text(
            c,
            288,
            552,
            &format!(
                "F9  Select {}{}",
                machine.prom_label(),
                if p.settings.display == crate::settings::Display::Sx {
                    ""
                } else {
                    " and TurboGX ROM"
                }
            ),
            ACCENT,
        )?;
        text(c, 352, 578, "Ctrl+P  Boot the workstation", MUTED)?;
    }
    draw_hints(c, &p)?;
    draw_leds(c, &p)?;
    if let Some(o) = p.overlay {
        draw_overlay(c, o, p.state, p.settings)?;
    }
    if let Some((confirm, choice)) = p.confirm {
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
        } else {
            text(
                c,
                244,
                352,
                "F9 saves, or discard to close without applying the draft.",
                MUTED,
            )?;
        }
        let choices = confirm.choices();
        let origin = confirm_origin(choices.len());
        for (index, label) in choices.iter().enumerate() {
            let value = if index == choice {
                format!("[ {label} ]")
            } else {
                format!("  {label}  ")
            };
            text(c, origin + index as i32 * CONFIRM_SLOT, 400, &value, ACCENT)?;
        }
        text(
            c,
            244,
            430,
            "Arrows select   Enter confirm   Escape cancel",
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

#[derive(Clone, Copy)]
struct LedPalette {
    idle: Color,
    active: Color,
}
const RUN_LED: LedPalette = LedPalette {
    idle: Color::RGB(25, 50, 110),
    active: Color::RGB(90, 160, 255),
};
const CPU_LED: LedPalette = LedPalette {
    idle: Color::RGB(55, 55, 55),
    active: Color::RGB(255, 255, 255),
};
const DISK_LED: LedPalette = LedPalette {
    idle: Color::RGB(18, 70, 18),
    active: Color::RGB(80, 255, 80),
};
const CD_LED: LedPalette = LedPalette {
    idle: Color::RGB(15, 60, 60),
    active: Color::RGB(80, 220, 220),
};
const NET_LED: LedPalette = LedPalette {
    idle: Color::RGB(25, 35, 75),
    active: Color::RGB(120, 180, 255),
};
const WRITE_LED: Color = Color::RGB(255, 200, 90);

fn draw_hints(c: &mut WindowCanvas, p: &Presentation<'_>) -> Result<(), sdl3::Error> {
    if !p.settings.show_status {
        return Ok(());
    }
    let y = (HEIGHT - LED_HEIGHT - HINT_HEIGHT) as i32;
    fill(
        c,
        Rect::new(0, y, WIDTH, HINT_HEIGHT),
        Color::RGB(0x10, 0x10, 0x14),
    )?;
    let status = if p.quitting {
        "SAVING NVRAM...".into()
    } else {
        format!(
            "{}  |  {} / {} MiB / {} / NAT {}",
            p.state.phase.label(),
            p.settings.model.short_label(),
            p.settings.ram_mib,
            p.settings.model.graphics_short(p.settings.display),
            if p.settings.network { "on" } else { "off" }
        )
    };
    let keys = if p.captured {
        "  Ctrl+Enter=release  Ctrl+P=pause  F4=shot  F5=reset  F9=options  F12=quit"
    } else {
        "  Ctrl+P=boot/pause  F4=shot  F5=reset  F9=options  F11=full  F12=quit"
    };
    let width = (status.len() + keys.len()) as f32 * 8.0;
    let scale = ((WIDTH - 16) as f32 / width).min(1.0);
    let x = ((WIDTH as f32 - width * scale) / 2.0 / scale) as i32;
    let text_y = (y as f32 + (HINT_HEIGHT as f32 - 8.0 * scale) / 2.0) / scale;
    c.set_scale(scale, scale)?;
    text(c, x, text_y as i32, &status, STATUS)?;
    text(c, x + 1, text_y as i32, &status, STATUS)?;
    text(
        c,
        x + status.len() as i32 * 8,
        text_y as i32,
        keys,
        Color::RGB(224, 224, 224),
    )?;
    c.set_scale(1.0, 1.0)?;
    Ok(())
}

fn draw_leds(c: &mut WindowCanvas, p: &Presentation<'_>) -> Result<(), sdl3::Error> {
    let y = (HEIGHT - LED_HEIGHT) as i32;
    fill(
        c,
        Rect::new(0, y, WIDTH, LED_HEIGHT),
        Color::RGB(18, 18, 18),
    )?;
    fill(c, Rect::new(0, y, WIDTH, 1), Color::RGB(50, 50, 50))?;
    if p.captured {
        text(
            c,
            8,
            y + 7,
            "CAPTURED / Ctrl+Enter",
            Color::RGB(255, 96, 80),
        )?;
    }
    let mut lamps = vec![
        (
            "RUN".to_owned(),
            p.state.phase == Phase::Running,
            false,
            RUN_LED,
        ),
        (
            "CPU".to_owned(),
            p.leds & activity::cpu() != 0,
            false,
            CPU_LED,
        ),
    ];
    if p.settings.network {
        lamps.push((
            "NET".to_owned(),
            p.leds & activity::ethernet() != 0,
            false,
            NET_LED,
        ));
    }
    for (id, target) in p.settings.scsi.iter().enumerate() {
        let Some(kind) = target.kind.media_kind() else {
            continue;
        };
        let disk = kind == helios::devices::scsi::MediaKind::Disk;
        let read = activity::disk_read(id);
        let write = activity::disk_write(id);
        lamps.push((
            format!("{}{id}", if disk { "HD" } else { "CD" }),
            p.leds & (read | write) != 0,
            p.leds & write != 0,
            if disk { DISK_LED } else { CD_LED },
        ));
    }
    let width: i32 = lamps
        .iter()
        .map(|(label, ..)| 20 + label.len() as i32 * 8)
        .sum::<i32>()
        + (lamps.len() as i32 - 1) * 8;
    let mut x = (WIDTH as i32 - width) / 2;
    let lamp_y = y + (LED_HEIGHT as i32 - 10) / 2;
    for (label, lit, write, palette) in lamps {
        let color = if !lit {
            palette.idle
        } else if write {
            WRITE_LED
        } else {
            palette.active
        };
        let rect = Rect::new(x, lamp_y, 14, 10);
        fill(c, rect, color)?;
        c.set_draw_color(Color::RGB(0, 0, 0));
        c.draw_rect(rect)?;
        text(c, x + 20, lamp_y + 1, &label, Color::RGB(205, 205, 205))?;
        x += 20 + label.len() as i32 * 8 + 8;
    }
    Ok(())
}

fn confirm_origin(count: usize) -> i32 {
    (WIDTH as i32 - count as i32 * CONFIRM_SLOT) / 2 + 30
}

pub fn confirm_choice_at(x: i32, y: i32, count: usize) -> Option<usize> {
    let origin = confirm_origin(count);
    if !(CONFIRM_Y..CONFIRM_Y + 35).contains(&y) || x < origin {
        return None;
    }
    let index = ((x - origin) / CONFIRM_SLOT) as usize;
    (index < count).then_some(index)
}

fn draw_overlay(
    c: &mut WindowCanvas,
    o: &Overlay,
    state: &Snapshot,
    settings: &Settings,
) -> Result<(), sdl3::Error> {
    fill(c, Rect::new(0, 0, WIDTH, HEIGHT), Color::RGBA(0, 0, 0, 150))?;
    panel(c, Rect::new(112, 100, 736, 540))?;
    text(c, 136, 122, "HELIACAL / OPTIONS", TEXT)?;
    text(
        c,
        568,
        122,
        &format!(
            "{} / {} MiB",
            settings.model.short_label(),
            settings.ram_mib
        ),
        MUTED,
    )?;
    if o.dirty(settings) {
        text(c, 696, 122, "Unsaved changes", Color::RGB(255, 208, 128))?;
    }
    for (i, tab) in o.tabs().iter().enumerate() {
        let x = 136 + i as i32 * 112;
        if *tab == o.tab {
            fill(c, Rect::new(x - 8, 150, 104, 28), Color::RGB(128, 96, 32))?;
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
            if o.draft.tinker {
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
            } else {
                text(
                    c,
                    144,
                    300,
                    "Diagnostics are hidden. Enable Tinker in General.",
                    MUTED,
                )?;
            }
        }
        Tab::About => {
            for (i, line) in [
                concat!("Heliacal ", env!("CARGO_PKG_VERSION"), " / Happy Years"),
                "Native Rust + SDL3 frontend for the Helios SPARC emulator.",
                "Sun SPARCstation 5/20 / Axil 320 / sun4m / SPARC V8 / 128 MiB RAM.",
                "Interface conventions follow Prismatic and the Happy Years family.",
                "Sun Type 5 US keyboard: Pause + A sends Stop-A.",
                "Guest mouse is connected after capture; audio is not.",
                "Host networking uses a user-mode NAT when enabled in Network.",
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
                "SCSI ID 7: host adapter (reserved). CD images are always read-only.",
                MUTED,
            )?;
            text(
                c,
                144,
                534,
                "Set hard-disk access in Advanced; click the type or press Space to cycle.",
                MUTED,
            )?;
        }
        Tab::Network => {
            text(
                c,
                144,
                428,
                "Guest NAT: DHCP/BOOTP, DNS and UDP relay, 192.168.0.2/24.",
                MUTED,
            )?;
            text(
                c,
                144,
                452,
                "Configure le0 in Solaris with DHCP or a static address and default route.",
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
        } else if o.tab == Tab::Advanced {
            "Arrows navigate / Enter toggles disk access"
        } else if o.tab == Tab::Network {
            "Arrows navigate / Enter toggles host networking"
        } else if o.tab == Tab::Media {
            "Space type / Enter choose image / Ctrl+E type path / Delete detach"
        } else {
            "Arrows navigate / Enter choose file / Ctrl+E type path / Delete detach"
        },
        MUTED,
    )?;
    text(
        c,
        144,
        602,
        "F9 Save   Esc Close   Firmware/media/access/network apply on next cold reset.",
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
