mod activity;
mod emulator;
mod input;
mod overlay;
mod settings;
mod view;

use emulator::{Emulator, Phase};
use overlay::{Confirm, Field, Overlay};
use sdl3::{
    dialog::DialogFileFilter,
    event::{Event, WindowEvent},
    keyboard::{Mod, Scancode},
    pixels::PixelFormat,
    render::ScaleMode,
};
use settings::{ScsiTarget, Settings, TargetKind};
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

type FilePicker = (
    Field,
    mpsc::Receiver<std::result::Result<Vec<PathBuf>, String>>,
);

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const PROM_FILTERS: [DialogFileFilter<'static>; 2] = [
    DialogFileFilter {
        name: "Sun PROM firmware",
        pattern: "bin;rom;prom",
    },
    DialogFileFilter {
        name: "All files",
        pattern: "*",
    },
];
const ROM_FILTERS: [DialogFileFilter<'static>; 2] = [
    DialogFileFilter {
        name: "FCode ROM",
        pattern: "bin;rom;fcode",
    },
    DialogFileFilter {
        name: "All files",
        pattern: "*",
    },
];
const MEDIA_FILTERS: [DialogFileFilter<'static>; 2] = [
    DialogFileFilter {
        name: "Disk and CD images",
        pattern: "chd;img;raw;dsk;hdd;iso;efs;bin;image",
    },
    DialogFileFilter {
        name: "All files",
        pattern: "*",
    },
];

#[derive(Default)]
struct Options {
    config: Option<PathBuf>,
    prom: Option<String>,
    rom: Option<String>,
    disk: Option<String>,
    cdrom: Option<String>,
    smoke: bool,
    boot_smoke: bool,
    screenshot: Option<PathBuf>,
}
impl Options {
    fn parse() -> Result<Option<Self>> {
        let mut o = Self::default();
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" | "-h" => {
                    println!(
                        "Heliacal - Happy Years frontend for Helios\n\nUsage: heliacal [--config PATH] [--prom PATH] [--cgsix-rom PATH]\n                [--disk RAW] [--cdrom ISO]\n                [--smoke-test | --boot-smoke-test] [--screenshot PATH.bmp]\n\nSun SPARCstation 5 / microSPARC-II 85 MHz / 128 MiB RAM / TurboGX.\nStarts paused. Supply your own firmware. Hard disks default to read/write; CDs are read-only.\nCtrl+P Boot/pause/resume   F4 Screenshot   F5 Confirm cold reset\nF9 Settings   F11 Fullscreen   F12 Confirm quit\nClick the guest display to capture the keyboard and mouse; Ctrl+Enter releases both.\nPause + A sends Sun Stop-A. Hold Shift to pass reserved function keys.\nOptions: arrows/Tab navigate, Enter/Ctrl+B choose a file, Ctrl+E type a path, F9 save.\nEsc closes a draft; unsaved changes prompt Save/Discard/Cancel (default Cancel).\nConfig paths are relative to the config; CLI paths use the working directory.\nSmoke tests use temporary state. --boot-smoke-test waits up to 180 seconds\nfor guest display output; it does not verify OS boot."
                    );
                    return Ok(None);
                }
                "--version" | "-V" => {
                    println!("heliacal {}", env!("CARGO_PKG_VERSION"));
                    return Ok(None);
                }
                "--config" => o.config = Some(args.next().ok_or("--config needs a path")?.into()),
                "--prom" => o.prom = Some(args.next().ok_or("--prom needs a path")?),
                "--cgsix-rom" => o.rom = Some(args.next().ok_or("--cgsix-rom needs a path")?),
                "--disk" => o.disk = Some(args.next().ok_or("--disk needs a path")?),
                "--cdrom" => o.cdrom = Some(args.next().ok_or("--cdrom needs a path")?),
                "--smoke-test" => o.smoke = true,
                "--boot-smoke-test" => o.boot_smoke = true,
                "--screenshot" => {
                    o.screenshot = Some(args.next().ok_or("--screenshot needs a path")?.into())
                }
                _ => return Err(format!("Unknown option {arg}; use --help").into()),
            }
        }
        if o.smoke && o.boot_smoke {
            return Err("Choose one smoke test".into());
        }
        if o.screenshot.is_some() && !o.smoke && !o.boot_smoke {
            return Err("--screenshot requires a smoke test".into());
        }
        Ok(Some(o))
    }
}

struct App {
    settings: Settings,
    config: PathBuf,
    emulator: Emulator,
    overlay: Option<Overlay>,
    confirm: Option<(Confirm, usize)>,
    picker: Option<FilePicker>,
    held: input::HeldKeys,
    captured: bool,
    quitting: bool,
    notice: Option<(String, Instant)>,
}
impl App {
    fn notify(&mut self, text: impl Into<String>) {
        self.notice = Some((text.into(), Instant::now()));
    }
    fn release(&mut self) {
        self.captured = false;
        self.held.clear();
        self.emulator.release_keys();
    }
    fn ask(&mut self, confirm: Confirm) {
        self.release();
        self.confirm = Some((confirm, confirm.default_choice()));
    }
    fn save(&mut self) {
        let Some(o) = &mut self.overlay else {
            return;
        };
        o.finish_edit();
        match o.draft.save(&self.config) {
            Ok(()) => {
                let changed = !self.settings.hardware_eq(&o.draft);
                self.settings = o.draft.clone();
                self.overlay = None;
                self.notify(if changed {
                    "Settings saved. Firmware, media and disk access apply on the next cold reset (F5)."
                } else {
                    "Settings saved."
                });
            }
            Err(error) => self.notify(error),
        }
    }
    fn close_overlay(&mut self) {
        if self
            .overlay
            .as_ref()
            .is_some_and(|o| o.dirty(&self.settings))
        {
            self.ask(Confirm::Unsaved);
        } else {
            self.overlay = None;
        }
    }
    fn browse(
        &mut self,
        field: Field,
        window: &sdl3::video::Window,
    ) -> std::result::Result<(), String> {
        let filters: &[DialogFileFilter] = match field {
            Field::Prom => &PROM_FILTERS,
            Field::Rom => &ROM_FILTERS,
            Field::Scsi(_) => &MEDIA_FILTERS,
            _ => return Ok(()),
        };
        let location = self
            .overlay
            .as_ref()
            .and_then(|o| o.path(field))
            .filter(|path| !path.is_empty())
            .map(|path| {
                let path = PathBuf::from(path);
                if path.is_relative() {
                    self.config.parent().unwrap_or(Path::new(".")).join(path)
                } else {
                    path
                }
            });
        let (send, receive) = mpsc::sync_channel(1);
        sdl3::dialog::show_open_file_dialog(
            filters,
            location,
            false,
            window,
            Box::new(move |files, _| {
                let _ = send.try_send(files.map_err(|e| e.to_string()));
            }),
        )
        .map_err(|e| e.to_string())?;
        self.picker = Some((field, receive));
        Ok(())
    }
    fn picked(&mut self, field: Field, path: &std::path::Path) {
        if let Some(o) = &mut self.overlay {
            o.set_path(field, path.display().to_string());
        }
    }
    fn activate(&mut self, window: &sdl3::video::Window) -> std::result::Result<(), String> {
        if let Some(o) = &mut self.overlay {
            let field = o.field();
            match field {
                field if field.is_path() => self.browse(field, window)?,
                _ => o.activate(),
            }
        }
        Ok(())
    }
    fn confirm(&mut self) {
        let Some((action, choice)) = self.confirm.take() else {
            return;
        };
        let result = match action {
            Confirm::Quit if choice == 0 => self.emulator.shutdown().map(|()| self.quitting = true),
            Confirm::Reset if choice == 0 => self
                .emulator
                .boot(self.settings.resolve(self.config.parent().unwrap())),
            Confirm::Unsaved if choice == 0 => {
                self.save();
                Ok(())
            }
            Confirm::Unsaved if choice == 1 => {
                self.overlay = None;
                Ok(())
            }
            _ => Ok(()),
        };
        if let Err(error) = result {
            self.notify(error);
        }
    }
    fn screenshot(&mut self) -> Result<()> {
        let state = self.emulator.snapshot();
        if !state.has_machine {
            self.notify("Boot the workstation before capturing its display.");
            return Ok(());
        }
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
        let path = format!("heliacal-{stamp}.ppm");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        write!(
            file,
            "P6\n{} {}\n255\n",
            helios::devices::cgsix::WIDTH,
            helios::devices::cgsix::HEIGHT
        )?;
        let rgb: Vec<u8> = state
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| p[..3].iter().copied())
            .collect();
        file.write_all(&rgb)?;
        self.notify(format!("Saved guest display to {path}"));
        Ok(())
    }
    fn key(
        &mut self,
        key: Scancode,
        mods: Mod,
        video: &sdl3::VideoSubsystem,
        window: &mut sdl3::video::Window,
    ) -> Result<()> {
        let ctrl = mods.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD);
        let shift = mods.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
        if self.quitting {
            return Ok(());
        }
        if self.confirm.is_some() {
            match key {
                Scancode::Escape => self.confirm = None,
                Scancode::Left | Scancode::Up | Scancode::Tab => {
                    let (action, choice) = self.confirm.as_mut().unwrap();
                    let count = action.choices().len();
                    *choice = (*choice + count - 1) % count;
                }
                Scancode::Right | Scancode::Down => {
                    let (action, choice) = self.confirm.as_mut().unwrap();
                    *choice = (*choice + 1) % action.choices().len();
                }
                Scancode::Return => self.confirm(),
                _ => {}
            }
            return Ok(());
        }
        if self.overlay.as_ref().is_some_and(|o| o.edit.is_some()) {
            let o = self.overlay.as_mut().unwrap();
            match key {
                Scancode::Escape => o.edit = None,
                Scancode::Return => o.finish_edit(),
                Scancode::F9 => self.save(),
                Scancode::Backspace => {
                    o.edit.as_mut().unwrap().pop();
                }
                Scancode::U if ctrl => o.edit.as_mut().unwrap().clear(),
                Scancode::V if ctrl => {
                    let clipboard = video.clipboard().clipboard_text()?;
                    let remaining = 4096usize.saturating_sub(o.edit.as_ref().unwrap().len());
                    o.edit.as_mut().unwrap().extend(
                        clipboard
                            .chars()
                            .filter(|c| !c.is_control())
                            .take(remaining),
                    );
                }
                _ => {}
            }
            return Ok(());
        }
        if key == Scancode::F9 && (!shift || self.overlay.is_some()) {
            self.release();
            if self.overlay.is_some() {
                self.save();
            } else {
                self.overlay = Some(Overlay::new(&self.settings));
            }
        } else if let Some(o) = &mut self.overlay {
            match key {
                Scancode::Escape => self.close_overlay(),
                Scancode::Up => o.move_row(-1),
                Scancode::Down => o.move_row(1),
                Scancode::Left => o.switch_tab(-1),
                Scancode::Right => o.switch_tab(1),
                Scancode::Tab => o.switch_tab(if shift { -1 } else { 1 }),
                Scancode::Space if matches!(o.field(), Field::Scsi(_)) => o.cycle_target(),
                Scancode::B if ctrl && o.field().is_path() => {
                    let field = o.field();
                    self.browse(field, window)?;
                }
                Scancode::E if ctrl && o.field().is_path() => o.activate(),
                Scancode::Return | Scancode::Space => self.activate(window)?,
                Scancode::Delete => o.detach(),
                _ => {}
            }
        } else if ctrl && key == Scancode::Return {
            if self.captured {
                self.release();
            } else if self.emulator.snapshot().phase == Phase::Running {
                self.captured = true;
            }
        } else if ctrl && key == Scancode::P {
            self.release();
            let state = self.emulator.snapshot();
            let result = if state.has_machine {
                self.emulator.pause()
            } else {
                self.emulator
                    .boot(self.settings.resolve(self.config.parent().unwrap()))
            };
            if state.phase == Phase::Failed {
                self.notify("The core stopped. F5 cold reset starts a fresh machine.");
            }
            if let Err(error) = result {
                self.notify(error);
            }
        } else if key == Scancode::F5 && !shift {
            self.ask(Confirm::Reset);
        } else if key == Scancode::F12 && !shift {
            self.ask(Confirm::Quit);
        } else if key == Scancode::F11 && !shift {
            window.set_fullscreen(window.fullscreen_state() == sdl3::video::FullscreenType::Off)?;
        } else if key == Scancode::F4 && !shift {
            if let Err(error) = self.screenshot() {
                self.notify(error.to_string());
            }
        } else if self.captured
            && let Some((code, down)) = self.held.event(key, true)
            && !self.emulator.key(code, down)
        {
            self.release();
            self.notify("Keyboard queue full; released input. Click display to capture again.");
        }
        Ok(())
    }
}

fn logo() -> Result<(Vec<u8>, u32, u32)> {
    let mut decoder =
        png::Decoder::new(std::io::Cursor::new(include_bytes!("../heliacal_logo.png")));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;
    let mut bytes = vec![0; reader.output_buffer_size().ok_or("Logo is too large")?];
    let info = reader.next_frame(&mut bytes)?;
    bytes.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => bytes,
        png::ColorType::Rgb => bytes
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        _ => return Err("Logo must be RGB or RGBA".into()),
    };
    Ok((rgba, info.width, info.height))
}

fn run(options: Options) -> Result<()> {
    let testing = options.smoke || options.boot_smoke;
    let temp = testing.then(tempfile::tempdir).transpose()?;
    let source_config = match options.config {
        Some(path) => std::path::absolute(path)?,
        None => settings::default_path()?,
    };
    let mut settings = if options.smoke {
        Settings::default()
    } else {
        Settings::load(&source_config)?
    };
    if testing {
        settings = settings.resolve(source_config.parent().unwrap());
    }
    for (dest, source) in [
        (&mut settings.prom, options.prom),
        (&mut settings.cgsix_rom, options.rom),
    ] {
        if let Some(path) = source {
            *dest = std::path::absolute(path)?.to_string_lossy().into_owned();
        }
    }
    for (id, kind, path) in [
        (3, TargetKind::Disk, options.disk),
        (6, TargetKind::Cdrom, options.cdrom),
    ] {
        if let Some(path) = path {
            settings.scsi[id] = ScsiTarget {
                kind,
                path: std::path::absolute(path)?.to_string_lossy().into_owned(),
            };
        }
    }
    let config = temp
        .as_ref()
        .map_or(source_config, |dir| dir.path().join("heliacal.cfg"));
    let _config_lock = settings::lock_config(&config)?;
    let emulator = Emulator::new(settings::companion_path(&config, ".nvram"))?;
    let mut app = App {
        settings,
        config,
        emulator,
        overlay: None,
        confirm: None,
        picker: None,
        held: input::HeldKeys::default(),
        captured: false,
        quitting: false,
        notice: None,
    };
    let sdl = sdl3::init()?;
    let video = sdl.video()?;
    let mut window = video
        .window(
            "Heliacal - Happy Years / Sun SPARCstation 5",
            view::WIDTH,
            view::HEIGHT,
        )
        .resizable()
        .high_pixel_density()
        .position_centered()
        .build()?;
    window.set_minimum_size(720, 570)?;
    let (mut logo_bytes, lw, lh) = logo()?;
    let icon =
        sdl3::surface::Surface::from_data(&mut logo_bytes, lw, lh, lw * 4, PixelFormat::RGBA32)?;
    window.set_icon(&icon);
    drop(icon);
    let mut canvas = window.into_canvas();
    canvas.set_logical_size(
        view::WIDTH,
        view::HEIGHT,
        sdl3::sys::render::SDL_RendererLogicalPresentation::LETTERBOX,
    )?;
    let creator = canvas.texture_creator();
    let mut logo_texture = creator.create_texture_streaming(PixelFormat::RGBA32, lw, lh)?;
    logo_texture.update(None, &logo_bytes, lw as usize * 4)?;
    logo_texture.set_blend_mode(sdl3::render::BlendMode::Blend);
    logo_texture.set_scale_mode(ScaleMode::Linear);
    let (gw, gh) = (
        helios::devices::cgsix::WIDTH as u32,
        helios::devices::cgsix::HEIGHT as u32,
    );
    let mut guest = creator.create_texture_streaming(PixelFormat::RGBA32, gw, gh)?;
    let mut events = sdl.event_pump()?;
    let mut frame_id = u64::MAX;
    let mut count = 0;
    let mut leds = activity::Leds::default();
    let mut last_steps = 0;
    let mut last_scsi = [(0u64, 0u64); settings::SCSI_TARGETS];
    let mut last_network = 0u64;
    let started = Instant::now();
    let mut display_at: Option<Instant> = None;
    if options.boot_smoke {
        app.emulator.boot(app.settings.clone())?;
    }
    loop {
        let frame_started = Instant::now();
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } => app.ask(Confirm::Quit),
                Event::Window {
                    win_event: WindowEvent::FocusLost,
                    ..
                } => app.release(),
                Event::KeyDown {
                    scancode: Some(key),
                    keymod,
                    repeat: false,
                    ..
                } if app.picker.is_none() => {
                    if let Err(error) = app.key(key, keymod, &video, canvas.window_mut()) {
                        app.notify(error.to_string());
                    }
                }
                Event::KeyUp {
                    scancode: Some(key),
                    ..
                } => {
                    if let Some((code, down)) = app.held.event(key, false)
                        && !app.emulator.key(code, down)
                    {
                        app.release();
                    }
                }
                Event::TextInput { text, .. } if app.picker.is_none() => {
                    if let Some(edit) = app.overlay.as_mut().and_then(|o| o.edit.as_mut()) {
                        let remaining = 4096usize.saturating_sub(edit.len());
                        edit.extend(text.chars().filter(|ch| !ch.is_control()).take(remaining));
                    }
                }
                Event::MouseMotion { xrel, yrel, .. } if app.captured => {
                    app.emulator.mouse_motion(xrel, -yrel);
                }
                Event::MouseButtonDown { mouse_btn, .. } if app.captured => {
                    if let Some(button) = pointer_button(mouse_btn) {
                        app.emulator.mouse_button(button, true);
                    }
                }
                Event::MouseButtonUp { mouse_btn, .. } if app.captured => {
                    if let Some(button) = pointer_button(mouse_btn) {
                        app.emulator.mouse_button(button, false);
                    }
                }
                Event::MouseButtonDown {
                    mouse_btn: sdl3::mouse::MouseButton::Left,
                    x,
                    y,
                    ..
                } if app.picker.is_none() && !app.quitting && !app.captured => {
                    let (x, y) = view::point(x, y, canvas.window().size());
                    if let Some((action, choice)) = app.confirm.as_mut() {
                        if let Some(index) = view::confirm_choice_at(x, y, action.choices().len()) {
                            *choice = index;
                            app.confirm();
                        }
                    } else if let Some(o) = app.overlay.as_mut() {
                        if o.edit.is_none() {
                            if (150..178).contains(&y) && (128..808).contains(&x) {
                                if let Some(tab) = o.tabs().get(((x - 128) / 112) as usize) {
                                    o.tab = *tab;
                                    o.selected = 0;
                                }
                            } else if (196..552).contains(&y) && (128..832).contains(&x) {
                                let row = ((y - 196) / 32) as usize;
                                if row < o.rows().len() {
                                    o.selected = row;
                                    if matches!(o.field(), Field::Scsi(_)) && x < 420 {
                                        o.cycle_target();
                                    } else {
                                        let result = app.activate(canvas.window());
                                        if let Err(error) = result {
                                            app.notify(error.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    } else if view::guest_rect(app.settings.show_status).contains_point((x, y))
                        && app.emulator.snapshot().phase == Phase::Running
                    {
                        app.captured = true;
                    }
                }
                _ => {}
            }
        }
        let polled = app
            .picker
            .as_ref()
            .map(|(field, receive)| (*field, receive.try_recv()));
        match polled {
            Some((field, Ok(Ok(files)))) => {
                if let Some(path) = files.first() {
                    app.picked(field, path);
                }
                app.picker = None;
            }
            Some((_, Ok(Err(error)))) => {
                if error != "Canceled" {
                    app.notify(error);
                }
                app.picker = None;
            }
            Some((_, Err(mpsc::TryRecvError::Disconnected))) => {
                app.picker = None;
                app.notify("File picker closed without a selection.");
            }
            Some((_, Err(mpsc::TryRecvError::Empty))) | None => {}
        }
        let text_input = video.text_input();
        let editing = app.overlay.as_ref().is_some_and(|o| o.edit.is_some());
        if editing != text_input.is_active(canvas.window()) {
            if editing {
                text_input.start(canvas.window());
            } else {
                text_input.stop(canvas.window());
            }
        }
        canvas.window_mut().set_keyboard_grab(app.captured);
        sdl.mouse()
            .set_relative_mouse_mode(canvas.window(), app.captured);
        let state = app.emulator.snapshot();
        let mut events = 0;
        if state.phase == Phase::Running && state.steps > last_steps {
            events |= activity::cpu();
        }
        last_steps = state.steps;
        if state.network_activity > last_network {
            events |= activity::ethernet();
        }
        last_network = state.network_activity;
        for (id, (reads, writes)) in state.scsi_activity.iter().enumerate() {
            let (last_reads, last_writes) = last_scsi[id];
            if *reads > last_reads {
                events |= activity::disk_read(id);
            }
            if *writes > last_writes {
                events |= activity::disk_write(id);
            }
            last_scsi[id] = (*reads, *writes);
        }
        leds.update(events, Instant::now());
        if app.quitting && state.shutdown_failed {
            app.quitting = false;
            app.notify("NVRAM could not be saved; machine retained. Fix the path/permissions and retry quit.");
        }
        if state.phase != Phase::Running && app.captured {
            app.release();
        }
        if frame_id != state.frame {
            guest.update(None, &state.pixels, gw as usize * 4)?;
            frame_id = state.frame;
        }
        guest.set_scale_mode(if app.settings.smoothing {
            ScaleMode::Linear
        } else {
            ScaleMode::Nearest
        });
        if app
            .notice
            .as_ref()
            .is_some_and(|(_, time)| time.elapsed() > Duration::from_secs(8))
        {
            app.notice = None;
        }
        view::draw(
            &mut canvas,
            &guest,
            &logo_texture,
            view::Presentation {
                settings: &app.settings,
                state: &state,
                overlay: app.overlay.as_ref(),
                confirm: app.confirm,
                captured: app.captured,
                leds: leds.lit,
                notice: app.notice.as_ref().map(|(s, _)| s.as_str()),
                quitting: app.quitting,
            },
        )?;
        count += 1;
        if options.smoke {
            if count == 2 {
                app.key(Scancode::F9, Mod::NOMOD, &video, canvas.window_mut())?;
            }
            if count == 4 {
                app.key(Scancode::Right, Mod::NOMOD, &video, canvas.window_mut())?;
                app.key(Scancode::Return, Mod::NOMOD, &video, canvas.window_mut())?;
                assert!(!app.settings.smoothing && app.overlay.as_ref().unwrap().draft.smoothing);
                app.key(Scancode::F9, Mod::NOMOD, &video, canvas.window_mut())?;
                assert!(Settings::load(&app.config)?.smoothing);
                app.key(Scancode::F9, Mod::NOMOD, &video, canvas.window_mut())?;
                app.notice = None;
            }
            if count == 6 {
                app.key(Scancode::Right, Mod::NOMOD, &video, canvas.window_mut())?;
                app.key(Scancode::Right, Mod::NOMOD, &video, canvas.window_mut())?;
                let base = app.config.parent().unwrap();
                let disk = base.join("smoke-disk.raw");
                let cdrom = base.join("smoke-cd.iso");
                std::fs::write(&disk, vec![0; 512])?;
                std::fs::write(&cdrom, vec![0; 2048])?;
                for (id, path, cycles) in [(0, disk, 1), (1, cdrom, 2)] {
                    app.overlay.as_mut().unwrap().selected = id;
                    for _ in 0..cycles {
                        app.key(Scancode::Space, Mod::NOMOD, &video, canvas.window_mut())?;
                    }
                    app.picked(Field::Scsi(id), &path);
                }
                assert!(app.settings.scsi[0].path.is_empty());
                app.key(Scancode::F9, Mod::NOMOD, &video, canvas.window_mut())?;
                let saved = Settings::load(&app.config)?;
                assert_eq!(saved.scsi[0].kind, TargetKind::Disk);
                assert_eq!(saved.scsi[1].kind, TargetKind::Cdrom);
                assert!(!saved.scsi[0].path.is_empty() && !saved.scsi[1].path.is_empty());
                app.key(Scancode::F9, Mod::NOMOD, &video, canvas.window_mut())?;
                app.key(Scancode::Right, Mod::NOMOD, &video, canvas.window_mut())?;
                app.key(Scancode::Right, Mod::NOMOD, &video, canvas.window_mut())?;
                app.notice = None;
            }
            if count == 8 {
                app.key(Scancode::Left, Mod::NOMOD, &video, canvas.window_mut())?;
                app.key(Scancode::Return, Mod::NOMOD, &video, canvas.window_mut())?;
                app.key(Scancode::Escape, Mod::NOMOD, &video, canvas.window_mut())?;
                assert!(matches!(app.confirm, Some((Confirm::Unsaved, 2))));
                app.key(Scancode::Return, Mod::NOMOD, &video, canvas.window_mut())?;
                assert!(app.overlay.is_some() && app.confirm.is_none());
                app.key(Scancode::Escape, Mod::NOMOD, &video, canvas.window_mut())?;
                app.key(Scancode::Left, Mod::NOMOD, &video, canvas.window_mut())?;
                assert!(matches!(app.confirm, Some((Confirm::Unsaved, 1))));
                app.key(Scancode::Return, Mod::NOMOD, &video, canvas.window_mut())?;
                assert!(app.overlay.is_none());
                app.key(Scancode::F9, Mod::NOMOD, &video, canvas.window_mut())?;
                app.key(Scancode::Right, Mod::NOMOD, &video, canvas.window_mut())?;
                app.key(Scancode::Right, Mod::NOMOD, &video, canvas.window_mut())?;
                app.notice = None;
            }
            if count == 10 {
                assert_eq!(state.phase, Phase::Ready);
                if let Some(path) = &options.screenshot {
                    canvas.read_pixels(None)?.save_bmp(path)?;
                }
                println!(
                    "SDL smoke test passed: paused launch, overlay editing, unsaved draft dialog, SCSI targets and saved settings."
                );
                app.emulator.shutdown()?;
                app.quitting = true;
            }
        }
        if options.boot_smoke && !app.quitting {
            if let Some(error) = &state.error {
                return Err(error.clone().into());
            }
            if display_at.is_none()
                && state
                    .pixels
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .any(|p| p[..3].iter().any(|&c| c != 0))
            {
                display_at = Some(Instant::now());
            }
            if display_at.is_some_and(|t| t.elapsed() >= Duration::from_secs(2)) {
                if let Some(path) = &options.screenshot {
                    canvas.read_pixels(None)?.save_bmp(path)?;
                }
                println!(
                    "Helios display smoke test passed: {} steps, PC={:08x}. This does not establish OS boot.",
                    state.steps, state.pc
                );
                app.emulator.shutdown()?;
                app.quitting = true;
            } else if started.elapsed() > Duration::from_secs(180) {
                return Err("Timed out waiting for guest display output".into());
            }
        }
        if app.quitting && state.phase == Phase::Stopped {
            if let Some(error) = state.error {
                return Err(error.into());
            }
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(16).saturating_sub(frame_started.elapsed()));
    }
}
fn pointer_button(button: sdl3::mouse::MouseButton) -> Option<u8> {
    match button {
        sdl3::mouse::MouseButton::Left => Some(0),
        sdl3::mouse::MouseButton::Middle => Some(1),
        sdl3::mouse::MouseButton::Right => Some(2),
        _ => None,
    }
}

fn main() {
    let result = Options::parse().and_then(|options| options.map_or(Ok(()), run));
    if let Err(error) = result {
        eprintln!("Heliacal: {error}");
        std::process::exit(1);
    }
}
