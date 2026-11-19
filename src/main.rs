mod activity;
#[cfg(test)]
mod chd_tests;
#[cfg(test)]
mod desktop_tests;
mod disk_creation;
mod emulator;
mod file_picker;
#[cfg(test)]
mod headless_tests;
mod input;
mod obp_console;
mod overlay;
#[cfg(feature = "parking-profile")]
mod parking_profile;
mod profiles;
#[cfg(test)]
mod ram_acceptance_tests;
mod serial_input;
mod serial_pty;
mod serial_screen;
mod serial_selection;
mod settings;
mod shutter;
mod terminal;
mod view;
mod window_size;

use emulator::{Emulator, Phase};
use overlay::{Confirm, Field, Overlay};
use sdl3::{
    dialog::DialogFileFilter,
    event::{Event, WindowEvent},
    keyboard::{Keycode, Mod, Scancode},
    pixels::PixelFormat,
    render::ScaleMode,
};
use settings::{Display, Model, ScsiTarget, Settings, TargetKind};
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
        name: "OBP or OpenBIOS firmware",
        pattern: "bin;rom;prom;elf",
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
    machine_label: Option<String>,
    machine: Option<Model>,
    display: Option<Display>,
    cpus: Option<u8>,
    ram_mib: Option<u32>,
    headless: Option<bool>,
    serial_pty_link: Option<String>,
    prom: Option<String>,
    rom: Option<String>,
    disk: Option<String>,
    cdrom: Option<String>,
    smoke: bool,
    boot_smoke: bool,
    screenshot: Option<PathBuf>,
    idle_parking: Option<bool>,
    execution_jit: Option<bool>,
}
impl Options {
    fn parse() -> Result<Option<Self>> {
        Self::parse_from(std::env::args().skip(1))
    }

    fn apply_hardware(&self, settings: &mut Settings) -> Result<()> {
        if let Some(model) = self.machine {
            settings.model = model;
        }
        if let Some(cpus) = self.cpus {
            settings.cpus = cpus;
        }
        if let Some(ram_mib) = self.ram_mib {
            settings.ram_mib = ram_mib;
        }
        if let Some(display) = self.display {
            settings.display = display;
        }
        if let Some(headless) = self.headless {
            settings.headless = headless;
        }
        if let Some(idle_parking) = self.idle_parking {
            settings.idle_parking = idle_parking;
        }
        if let Some(execution_jit) = self.execution_jit {
            settings.execution_jit = execution_jit;
        }
        settings.validate_hardware()?;
        Ok(())
    }

    fn parse_from(args: impl Iterator<Item = String>) -> Result<Option<Self>> {
        let mut o = Self::default();
        let mut args = args;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" | "-h" => {
                    println!(
                        "Heliacal - Happy Years frontend for Helios\n\nUsage: heliacal [--config PATH] [--machine ss5|ss20|axil320]\n                [--cpus 1|2] [--display cgsix|sx] [--ram-mib MIB]\n                [--headless | --graphical] [--idle-parking] [--serial-pty-link PATH]\n                [--prom PATH] [--cgsix-rom PATH]\n                [--disk RAW_OR_CHD] [--cdrom ISO] [--machine-label LABEL]\n                [--smoke-test | --boot-smoke-test] [--screenshot PATH.bmp]\n\nSun SPARCstation 5, SPARCstation 20 or Axil 320; default 128 MiB RAM.\nSS-20 supports native SX/CG14 or TurboGX; SS-5 and Axil 320 use TurboGX.\nSS-5: 1 CPU; SS-20/Axil: 1 or 2 CPUs (experimental SMP).\n--ram-mib accepts only the exact DSIMM totals valid for the selected model/display.\nHeadless retains the control window and uses a TTYA PTY; no framebuffer ROM required.\n--idle-parking enables experimental single-CPU SS-20 Solaris idle parking (default off).\nF9 > General > Tinker enables Headless/PTY controls in Advanced.\nStarts paused. An empty PROM path selects the installed OpenBIOS fallback.\nAn explicit PROM takes precedence; errors do not trigger a fallback.\nHELIOS_OPENBIOS can specify the OpenBIOS ELF file. Firmware is not bundled.\nHard disks default to read/write; CDs are read-only.\nCtrl+P Boot/pause/resume   F4 Screenshot + shutter sound   F5 Confirm cold reset\nF9 Settings   F11 Fullscreen   F12 Quit (no confirmation when STOPPED)\nCtrl+Plus / Ctrl+Minus Resize window outside fullscreen (Ctrl+Equals/keypad also work).\nClick the guest display to capture the keyboard and mouse; Ctrl+Enter releases both.\nHeadless: click the serial console to type; Ctrl+Enter focuses/releases it.\nCopy command copies minicom; Ctrl+Shift+C copies selected serial text (or its command).\nCtrl+Shift+V pastes into the focused serial console or captured graphical guest.\nWheel/scrollbar or Shift+PageUp/PageDown browse serial history; Shift+End goes live.\nPause + A sends Sun Stop-A. Hold Shift to pass reserved function keys.\nOptions: arrows/Tab navigate, Enter/Ctrl+B choose a file, Ctrl+E type a path, F9 save.\nEsc closes a draft; unsaved changes prompt Save/Discard/Cancel (default Cancel).\nConfig paths are relative to the config; CLI paths use the working directory.\nSmoke tests use temporary state. --boot-smoke-test waits up to 180 seconds\nfor guest display output; it does not verify OS boot."
                    );
                    return Ok(None);
                }
                "--version" | "-V" => {
                    println!("heliacal {}", env!("CARGO_PKG_VERSION"));
                    return Ok(None);
                }
                "--config" => o.config = Some(args.next().ok_or("--config needs a path")?.into()),
                "--machine-label" => {
                    o.machine_label = Some(args.next().ok_or("--machine-label needs a label")?)
                }
                "--machine" => {
                    let value = args.next().ok_or("--machine needs ss5, ss20 or axil320")?;
                    o.machine = Some(match value.as_str() {
                        "ss5" => Model::Sparcstation5,
                        "ss20" => Model::Sparcstation20,
                        "axil320" => Model::Axil320,
                        _ => {
                            return Err(format!(
                                "Unknown machine {value}; use ss5, ss20 or axil320"
                            )
                            .into());
                        }
                    });
                }
                "--cpus" => {
                    o.cpus = Some(
                        args.next()
                            .ok_or("--cpus needs a count")?
                            .parse()
                            .map_err(|_| "--cpus needs an integer (1 or 2)")?,
                    );
                }
                "--ram-mib" => {
                    let value = args.next().ok_or("--ram-mib needs MiB")?;
                    o.ram_mib = Some(
                        value
                            .parse::<u32>()
                            .map_err(|_| "--ram-mib needs a non-negative integer number of MiB")?,
                    );
                }
                "--display" => {
                    let value = args.next().ok_or("--display needs cgsix or sx")?;
                    o.display = Some(match value.as_str() {
                        "cgsix" => Display::Cgsix,
                        "sx" => Display::Sx,
                        _ => return Err(format!("Unknown display {value}; use cgsix or sx").into()),
                    });
                }
                "--headless" => o.headless = Some(true),
                "--graphical" => o.headless = Some(false),
                "--idle-parking" => o.idle_parking = Some(true),
                "--execution" => {
                    let value = args.next().ok_or("--execution needs interpreter or jit")?;
                    o.execution_jit = Some(match value.as_str() {
                        "interpreter" => false,
                        "jit" => true,
                        _ => return Err("--execution needs interpreter or jit".into()),
                    });
                }
                "--serial-pty-link" => {
                    o.serial_pty_link = Some(args.next().ok_or("--serial-pty-link needs a path")?)
                }
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
    picker_history: file_picker::History,
    shutter: Option<shutter::Shutter>,
    creation: Option<disk_creation::Creation>,
    held: input::HeldKeys,
    captured: bool,
    serial_focused: bool,
    serial_selection: Option<serial_selection::Selection>,
    serial_scroll_drag: Option<i32>,
    serial_wheel: f32,
    quitting: bool,
    notice: Option<(String, Instant)>,
}
impl App {
    fn notify(&mut self, text: impl Into<String>) {
        self.notice = Some((text.into(), Instant::now()));
    }
    fn release(&mut self) {
        self.captured = false;
        self.serial_focused = false;
        self.serial_scroll_drag = None;
        self.serial_selection = None;
        self.serial_wheel = 0.0;
        self.held.clear();
        self.emulator.release_keys();
    }
    fn ask(&mut self, confirm: Confirm) {
        self.release();
        self.confirm = Some((confirm, confirm.default_choice()));
    }
    fn request_quit(&mut self) {
        if self.quitting {
            return;
        }
        if self.creation.is_some() {
            self.notify("Disk creation is finishing; retry Quit when it completes.");
        } else if matches!(
            self.emulator.snapshot().phase,
            Phase::Stopped | Phase::Failed
        ) {
            self.release();
            match self.emulator.shutdown() {
                Ok(()) => self.quitting = true,
                Err(error) => self.notify(error),
            }
        } else {
            self.ask(Confirm::Quit);
        }
    }
    fn save(&mut self) {
        let Some(o) = &mut self.overlay else {
            return;
        };
        if o.new_disk.is_some() {
            self.notify("Create or cancel the new disk before saving settings.");
            return;
        }
        if let Err(error) = o.finish_edit() {
            self.notify(error);
            return;
        }
        o.draft.remember_machine();
        match o.draft.save(&self.config) {
            Ok(()) => {
                let changed = !self.settings.hardware_eq(&o.draft);
                self.settings = o.draft.clone();
                self.overlay = None;
                self.notify(if changed && self.emulator.snapshot().has_machine {
                    "Settings saved. F5 cold reset applies the selected machine; Ctrl+P resumes the current guest."
                } else if changed {
                    "Settings saved. Ctrl+P boots the selected machine."
                } else {
                    "Settings saved."
                });
            }
            Err(error) => self.notify(error),
        }
    }
    fn close_overlay(&mut self) {
        if let Some(o) = &mut self.overlay
            && o.new_disk.is_some()
        {
            o.cancel_new_disk();
            return;
        }
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
            Field::Scsi(_) | Field::NewDiskPath => &MEDIA_FILTERS,
            Field::SerialPtyLink => {
                if let Some(o) = &mut self.overlay {
                    o.activate();
                }
                return Ok(());
            }
            _ => return Ok(()),
        };
        let location = self.overlay.as_ref().and_then(|o| {
            self.picker_history
                .location(o, field, self.config.parent().unwrap_or(Path::new(".")))
        });
        let (send, receive) = mpsc::sync_channel(1);
        let callback: sdl3::dialog::DialogCallback = Box::new(
            move |files: std::result::Result<Vec<PathBuf>, sdl3::dialog::DialogError>, _| {
                let _ = send.try_send(files.map_err(|e| e.to_string()));
            },
        );
        if field == Field::NewDiskPath {
            sdl3::dialog::show_save_file_dialog(
                &[DialogFileFilter {
                    name: "CHD hard disk",
                    pattern: "chd",
                }],
                location,
                window,
                callback,
            )
        } else {
            sdl3::dialog::show_open_file_dialog(filters, location, false, window, callback)
        }
        .map_err(|e| e.to_string())?;
        self.picker = Some((field, receive));
        Ok(())
    }
    fn picked(&mut self, field: Field, path: &std::path::Path) {
        // Some native backends report cancellation as an empty filename.
        if path.as_os_str().is_empty() {
            return;
        }
        if let Some(o) = &mut self.overlay {
            o.set_path(field, path.display().to_string());
            self.picker_history.remember(
                field,
                &o.draft,
                path,
                self.config.parent().unwrap_or(Path::new(".")),
            );
            if let Err(error) = self.picker_history.save(&self.config) {
                self.notify(format!(
                    "Image selected, but picker history could not be saved: {error}"
                ));
            }
        }
    }
    fn activate(&mut self, window: &sdl3::video::Window) -> std::result::Result<(), String> {
        let notice = if let Some(o) = &mut self.overlay {
            let field = o.field();
            if field == Field::CreateDisk {
                let disk = o.new_disk.as_ref().ok_or("No disk creation draft")?;
                if disk.path.is_empty() {
                    return Err("Choose a destination first.".into());
                }
                let path = PathBuf::from(&disk.path);
                let path = if path.is_relative() {
                    self.config.parent().unwrap_or(Path::new(".")).join(path)
                } else {
                    path
                };
                self.creation = Some(disk_creation::Creation::start(
                    path,
                    disk.size_mib,
                    disk.target,
                )?);
                Some("Creating blank CHD disk...".into())
            } else if field == Field::OpenSerialTerminal || field == Field::SerialBreak {
                let state = self.emulator.snapshot();
                let endpoint = state.serial_endpoint.as_deref().ok_or(
                    "Boot a headless machine first; draft changes apply after Save and F5.",
                )?;
                if field == Field::OpenSerialTerminal {
                    terminal::open(endpoint)?;
                    Some(
                        "Opening minicom. Use 9600 8N1 with hardware/software flow control off."
                            .into(),
                    )
                } else {
                    self.emulator.serial_break()?;
                    Some("Serial BREAK queued for the running guest.".into())
                }
            } else if field.is_path() {
                self.browse(field, window)?;
                None
            } else {
                o.activate();
                o.notice.take()
            }
        } else {
            None
        };
        if let Some(notice) = notice {
            self.notify(notice);
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
        if state.headless {
            self.notify("This headless guest has no framebuffer to capture.");
            return Ok(());
        }
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
        drop(file);
        // Host feedback is optional; audio failure must not fail a saved image.
        if self.shutter.is_none() {
            match shutter::Shutter::new() {
                Ok(sound) => self.shutter = Some(sound),
                Err(error) => eprintln!("Screenshot sound unavailable: {error}"),
            }
        }
        if let Some(sound) = &self.shutter
            && let Err(error) = sound.play()
        {
            eprintln!("Screenshot sound unavailable: {error}");
        }
        self.notify(format!("Saved guest display to {path}"));
        Ok(())
    }
    fn copy_serial_command(&mut self, video: &sdl3::VideoSubsystem) -> Result<()> {
        let state = self.emulator.snapshot();
        let endpoint = state
            .serial_endpoint
            .as_deref()
            .ok_or("Boot a headless machine to create its serial endpoint.")?;
        video
            .clipboard()
            .set_clipboard_text(&terminal::recipe(endpoint))?;
        self.notify("Minicom command copied to clipboard.");
        Ok(())
    }
    fn serial_controls(&self) -> bool {
        self.settings.show_serial_console
            && self.overlay.is_none()
            && self.confirm.is_none()
            && self.picker.is_none()
            && self.creation.is_none()
            && !self.quitting
            && {
                let state = self.emulator.snapshot();
                state.headless && state.has_machine
            }
    }
    fn scroll_serial(&mut self, request: serial_screen::Scroll) {
        self.serial_selection = None;
        let state = self.emulator.snapshot();
        self.emulator
            .scroll_serial(state.serial_input_generation, request);
    }
    fn serial_history_key(&mut self, key: Scancode, mods: Mod) -> bool {
        if !self.serial_controls() {
            return false;
        }
        if key == Scancode::Escape && self.serial_selection.is_some() {
            self.serial_selection = None;
            return true;
        }
        if !mods.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD)
            || mods.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD | Mod::LALTMOD | Mod::RALTMOD)
        {
            return false;
        }
        use serial_screen::Scroll;
        let request = match key {
            Scancode::PageUp => Scroll::By(24),
            Scancode::PageDown => Scroll::By(-24),
            Scancode::Home => Scroll::To(self.emulator.snapshot().serial_screen.history),
            Scancode::End => Scroll::To(0),
            _ => return false,
        };
        self.scroll_serial(request);
        true
    }
    fn copy_console_text(&mut self, video: &sdl3::VideoSubsystem) -> Result<()> {
        if let Some(selection) = &self.serial_selection
            && selection.nonempty()
        {
            video.clipboard().set_clipboard_text(&selection.text())?;
            self.notify("Selected console text copied.");
            Ok(())
        } else {
            self.copy_serial_command(video)
        }
    }
    fn paste_clipboard(&mut self, video: &sdl3::VideoSubsystem) -> Result<()> {
        let state = self.emulator.snapshot();
        if state.phase != Phase::Running {
            return Err("Start or resume the guest before pasting.".into());
        }
        let text = video.clipboard().clipboard_text()?;
        if state.headless {
            if !self.serial_focused {
                return Err("Click the serial console before pasting.".into());
            }
            let bytes = serial_input::paste_bytes(&text, state.serial_screen.bracketed_paste)?;
            self.emulator
                .serial_input(state.serial_input_generation, &bytes)?;
            self.serial_selection = None;
        } else {
            if !self.captured {
                return Err("Capture the guest display before pasting.".into());
            }
            let keys = input::paste_events(&text)?;
            self.emulator
                .paste_keys(state.serial_input_generation, keys)?;
            self.held.clear();
        }
        self.notify("Pasting to the guest. Ctrl+Enter releases input and cancels queued paste.");
        Ok(())
    }
    fn serial_text(&mut self, text: &str) -> Result<()> {
        let state = self.emulator.snapshot();
        if self.serial_focused && state.headless && state.phase == Phase::Running {
            self.emulator
                .serial_input(state.serial_input_generation, text.as_bytes())?;
            self.serial_selection = None;
        }
        Ok(())
    }
    fn serial_key(&mut self, key: Scancode, keycode: Option<Keycode>, mods: Mod) -> Result<()> {
        let state = self.emulator.snapshot();
        if self.serial_focused
            && state.headless
            && state.phase == Phase::Running
            && let Some(bytes) = serial_input::key_bytes(
                key,
                keycode,
                mods,
                state.serial_screen.application_cursor(),
            )
        {
            self.emulator
                .serial_input(state.serial_input_generation, &bytes)?;
            self.serial_selection = None;
        }
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
                Scancode::Return => o.finish_edit()?,
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
        } else if ctrl && shift && key == Scancode::C && self.emulator.snapshot().headless {
            self.copy_console_text(video)?;
        } else if ctrl && shift && key == Scancode::V {
            self.paste_clipboard(video)?;
        } else if ctrl && key == Scancode::Return {
            if self.captured || self.serial_focused {
                self.release();
            } else {
                let state = self.emulator.snapshot();
                if state.phase == Phase::Running {
                    if state.headless {
                        self.serial_focused = self.settings.show_serial_console;
                    } else {
                        self.captured = true;
                    }
                }
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
            self.request_quit();
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
    let source_config = match &options.config {
        Some(path) => std::path::absolute(path)?,
        None => settings::default_path()?,
    };
    let mut settings = if options.smoke {
        Settings::default()
    } else {
        Settings::load(&source_config)?
    };
    if let Some(label) = &options.machine_label {
        settings.select_machine(label)?;
    }
    if testing {
        settings = settings.resolve(source_config.parent().unwrap());
    }
    options.apply_hardware(&mut settings)?;
    #[cfg(not(feature = "jit-native"))]
    if settings.execution_jit {
        return Err(
            "Native execution requested but this Heliacal build lacks the jit-native feature; rebuild with --features jit-native or select interpreter mode"
                .into(),
        );
    }
    if let Some(path) = options.prom {
        settings.prom = if path.is_empty() {
            path
        } else {
            std::path::absolute(path)?.to_string_lossy().into_owned()
        };
    }
    for (dest, source) in [
        (&mut settings.cgsix_rom, options.rom),
        (&mut settings.serial_pty_link, options.serial_pty_link),
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
    let picker_history = file_picker::History::load(&config).unwrap_or_else(|error| {
        eprintln!("File picker history unavailable: {error}");
        file_picker::History::default()
    });
    let emulator = Emulator::new(settings::companion_path(&config, ".nvram"))?;
    let mut app = App {
        settings,
        config,
        emulator,
        overlay: None,
        confirm: None,
        picker: None,
        picker_history,
        shutter: None,
        creation: None,
        held: input::HeldKeys::default(),
        captured: false,
        serial_focused: false,
        serial_selection: None,
        serial_scroll_drag: None,
        serial_wheel: 0.0,
        quitting: false,
        notice: None,
    };
    let sdl = sdl3::init()?;
    let video = sdl.video()?;
    let mut window = video
        .window(
            &format!("Heliacal - Happy Years / {}", app.settings.machine_name()),
            view::WIDTH,
            view::HEIGHT,
        )
        .resizable()
        .high_pixel_density()
        .position_centered()
        .build()?;
    window.set_minimum_size(window_size::MINIMUM.0, window_size::MINIMUM.1)?;
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
    eprintln!(
        "Heliacal video_driver={} renderer={}",
        video.current_video_driver(),
        canvas.renderer_name
    );
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
    let mut last_leds = u64::MAX;
    let mut last_phase = Phase::Ready;
    let mut last_has_machine = false;
    let mut last_waiting_for_console = false;
    let mut last_machine_name: Option<String> = None;
    let mut last_error: Option<String> = None;
    let mut last_notice: Option<String> = None;
    let mut last_captured = false;
    let mut last_serial_focused = false;
    let mut last_serial_status = (None, None, 0);
    let mut last_serial_frame = 0;
    let mut last_shutdown_failed = false;
    let mut redraw = true;
    let mut grabbed: Option<bool> = None;
    let mut scale_mode: Option<ScaleMode> = None;
    let mut pending_event: Option<Event> = None;
    let mut resizer = window_size::Resizer::default();
    let started = Instant::now();
    let mut display_at: Option<Instant> = None;
    if options.boot_smoke {
        app.emulator.boot(app.settings.clone())?;
    }
    loop {
        let mut event_seen = false;
        for event in pending_event.take().into_iter().chain(events.poll_iter()) {
            event_seen = true;
            match event {
                Event::Quit { .. } => {
                    app.request_quit();
                }
                Event::Window {
                    win_event: WindowEvent::FocusLost,
                    ..
                } => {
                    resizer.focus_lost();
                    app.release();
                }
                Event::KeyDown {
                    scancode: Some(key),
                    keycode,
                    keymod,
                    repeat,
                    ..
                } if app.picker.is_none() && app.creation.is_none() => {
                    let result = if let Some(increase) = window_size::shortcut(key, keycode, keymod)
                    {
                        resizer.key_down(key);
                        if !repeat && !app.quitting {
                            resizer
                                .request(canvas.window(), increase)
                                .map_err(Into::into)
                        } else {
                            Ok(())
                        }
                    } else if app.serial_history_key(key, keymod) {
                        Ok(())
                    } else if app.serial_focused && !serial_input::reserved(key, keymod) {
                        app.serial_key(key, keycode, keymod)
                    } else if !repeat {
                        app.key(key, keymod, &video, canvas.window_mut())
                    } else {
                        Ok(())
                    };
                    if let Err(error) = result {
                        app.notify(error.to_string());
                    }
                }
                Event::KeyUp {
                    scancode: Some(key),
                    ..
                } => {
                    resizer.key_up(key);
                    if let Some((code, down)) = app.held.event(key, false)
                        && !app.emulator.key(code, down)
                    {
                        app.release();
                    }
                }
                Event::TextInput { text, .. }
                    if app.picker.is_none()
                        && app.creation.is_none()
                        && !resizer.suppress_text() =>
                {
                    if let Some(edit) = app.overlay.as_mut().and_then(|o| o.edit.as_mut()) {
                        let remaining = 4096usize.saturating_sub(edit.len());
                        edit.extend(text.chars().filter(|ch| !ch.is_control()).take(remaining));
                    } else if app.confirm.is_none()
                        && app.overlay.is_none()
                        && !app.quitting
                        && let Err(error) = app.serial_text(&text)
                    {
                        app.notify(error.to_string());
                    }
                }
                Event::MouseWheel {
                    y,
                    direction,
                    mouse_x,
                    mouse_y,
                    ..
                } if app.serial_controls() => {
                    let at = view::point(mouse_x, mouse_y, canvas.window().size());
                    if view::serial_console_rect().contains_point(at)
                        || view::serial_scroll_rect().contains_point(at)
                    {
                        let delta = if direction == sdl3::mouse::MouseWheelDirection::Flipped {
                            -y
                        } else {
                            y
                        };
                        if delta.is_finite() {
                            app.serial_wheel += delta * 3.0;
                            let rows = app.serial_wheel as i32;
                            app.serial_wheel -= rows as f32;
                            if rows != 0 {
                                app.scroll_serial(serial_screen::Scroll::By(rows));
                            }
                        }
                    }
                }
                Event::MouseMotion { x, y, .. }
                    if app.serial_controls()
                        && (app.serial_scroll_drag.is_some()
                            || app.serial_selection.as_ref().is_some_and(|s| s.dragging)) =>
                {
                    let (x, y) = view::point(x, y, canvas.window().size());
                    if let Some(grab) = app.serial_scroll_drag {
                        let history = app.emulator.snapshot().serial_screen.history;
                        app.scroll_serial(serial_screen::Scroll::To(view::serial_scroll_offset(
                            history, y, grab,
                        )));
                    } else if let Some(selection) = &mut app.serial_selection {
                        selection.end = view::serial_cell_at(x, y);
                    }
                }
                Event::MouseButtonUp {
                    mouse_btn: sdl3::mouse::MouseButton::Left,
                    ..
                } if app.serial_scroll_drag.is_some()
                    || app.serial_selection.as_ref().is_some_and(|s| s.dragging) =>
                {
                    app.serial_scroll_drag = None;
                    if let Some(selection) = &mut app.serial_selection {
                        selection.dragging = false;
                    }
                    if app.serial_selection.as_ref().is_some_and(|s| !s.nonempty()) {
                        app.serial_selection = None;
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
                } if app.picker.is_none()
                    && app.creation.is_none()
                    && !app.quitting
                    && !app.captured =>
                {
                    let (x, y) = view::point(x, y, canvas.window().size());
                    if let Some((action, choice)) = app.confirm.as_mut() {
                        if let Some(index) = view::confirm_choice_at(x, y, action.choices().len()) {
                            *choice = index;
                            app.confirm();
                        }
                    } else if let Some(o) = app.overlay.as_mut() {
                        if o.edit.is_none() {
                            if (150..178).contains(&y) && (128..808).contains(&x) {
                                if let Some(tab) = o.tabs().get(((x - 128) / 112) as usize)
                                    && o.new_disk.is_none()
                                {
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
                    } else {
                        let state = app.emulator.snapshot();
                        if state.has_machine && state.headless {
                            app.release();
                            if state.serial_endpoint.is_some()
                                && view::serial_copy_rect(app.settings.show_serial_console)
                                    .contains_point((x, y))
                            {
                                if let Err(error) = app.copy_serial_command(&video) {
                                    app.notify(error.to_string());
                                }
                            } else if app.settings.show_serial_console
                                && view::serial_scroll_rect().contains_point((x, y))
                            {
                                let thumb = view::serial_scroll_thumb(
                                    state.serial_screen.history,
                                    state.serial_screen.offset,
                                );
                                if thumb.contains_point((x, y)) {
                                    app.serial_scroll_drag = Some(y - thumb.y());
                                } else {
                                    app.scroll_serial(serial_screen::Scroll::By(
                                        if y < thumb.y() { 24 } else { -24 },
                                    ));
                                }
                            } else if app.settings.show_serial_console
                                && view::serial_console_rect().contains_point((x, y))
                            {
                                app.serial_focused = state.phase == Phase::Running;
                                app.serial_selection = Some(serial_selection::Selection::new(
                                    state.serial_screen.clone(),
                                    view::serial_cell_at(x, y),
                                ));
                            }
                        } else if view::guest_rect(app.settings.show_status).contains_point((x, y))
                            && state.phase == Phase::Running
                        {
                            app.captured = true;
                        }
                    }
                }
                _ => {}
            }
        }
        match resizer.apply(canvas.window_mut()) {
            Ok(changed) => redraw |= changed,
            Err(error) => app.notify(format!("Window resize failed: {error}")),
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
                redraw = true;
            }
            Some((_, Ok(Err(error)))) => {
                if error != "Canceled" {
                    app.notify(error);
                }
                app.picker = None;
                redraw = true;
            }
            Some((_, Err(mpsc::TryRecvError::Disconnected))) => {
                app.picker = None;
                app.notify("File picker closed without a selection.");
                redraw = true;
            }
            Some((_, Err(mpsc::TryRecvError::Empty))) | None => {}
        }
        if app.creation.as_ref().is_some_and(|job| job.is_finished()) {
            let creation = app.creation.take().unwrap();
            match creation.finish() {
                Ok((path, target)) => {
                    if let Some(overlay) = &mut app.overlay {
                        overlay.disk_created(target, path.display().to_string());
                    }
                    app.notify("Disk created and attached to draft. Save, then F5 to apply. Discard keeps the new file.");
                }
                Err(error) => app.notify(format!("Disk creation failed: {error}")),
            }
            redraw = true;
        }
        let text_input = video.text_input();
        let editing = app.overlay.as_ref().is_some_and(|o| o.edit.is_some()) || app.serial_focused;
        if editing != text_input.is_active(canvas.window()) {
            if editing {
                text_input.start(canvas.window());
            } else {
                text_input.stop(canvas.window());
            }
        }
        if grabbed != Some(app.captured) {
            canvas.window_mut().set_keyboard_grab(app.captured);
            sdl.mouse()
                .set_relative_mouse_mode(canvas.window(), app.captured);
            grabbed = Some(app.captured);
        }
        let state = app.emulator.snapshot();
        let name = state
            .machine_name
            .clone()
            .unwrap_or_else(|| app.settings.machine_name());
        let title = format!("Heliacal - Happy Years / {name}");
        if canvas.window().title() != title {
            canvas.window_mut().set_title(&title)?;
        }
        let serial_status = (
            state.serial_endpoint.clone(),
            state.serial_error.clone(),
            state.serial_dropped,
        );
        if serial_status != last_serial_status {
            redraw = true;
            last_serial_status = serial_status;
        }
        if state.serial_frame != last_serial_frame {
            if state.headless && app.settings.show_serial_console {
                redraw = true;
            }
            last_serial_frame = state.serial_frame;
        }
        if state.phase != last_phase
            || state.has_machine != last_has_machine
            || state.waiting_for_console != last_waiting_for_console
            || state.machine_name != last_machine_name
            || state.error != last_error
            || state.shutdown_failed != last_shutdown_failed
            || app.captured != last_captured
            || app.serial_focused != last_serial_focused
        {
            redraw = true;
            last_phase = state.phase;
            last_has_machine = state.has_machine;
            last_waiting_for_console = state.waiting_for_console;
            last_machine_name = state.machine_name.clone();
            last_error = state.error.clone();
            last_shutdown_failed = state.shutdown_failed;
            last_captured = app.captured;
            last_serial_focused = app.serial_focused;
        }
        let mut activity_events = 0;
        if state.phase == Phase::Running && state.steps > last_steps {
            activity_events |= activity::cpu();
        }
        last_steps = state.steps;
        if state.network_activity > last_network {
            activity_events |= activity::ethernet();
        }
        last_network = state.network_activity;
        for (id, (reads, writes)) in state.scsi_activity.iter().enumerate() {
            let (last_reads, last_writes) = last_scsi[id];
            if *reads > last_reads {
                activity_events |= activity::disk_read(id);
            }
            if *writes > last_writes {
                activity_events |= activity::disk_write(id);
            }
            last_scsi[id] = (*reads, *writes);
        }
        leds.update(activity_events, Instant::now());
        if app.quitting && state.shutdown_failed {
            app.quitting = false;
            app.notify("NVRAM could not be saved; machine retained. Fix the path/permissions and retry quit.");
        }
        if ((state.phase != Phase::Running || state.headless) && app.captured)
            || (app.serial_focused
                && (state.phase != Phase::Running
                    || !state.headless
                    || !app.settings.show_serial_console))
        {
            app.release();
        }
        if frame_id != state.frame {
            guest.update(None, &state.pixels, gw as usize * 4)?;
            frame_id = state.frame;
            redraw = true;
        }
        let next_scale = if app.settings.smoothing {
            ScaleMode::Linear
        } else {
            ScaleMode::Nearest
        };
        if scale_mode != Some(next_scale) {
            guest.set_scale_mode(next_scale);
            scale_mode = Some(next_scale);
        }
        if app
            .notice
            .as_ref()
            .is_some_and(|(_, time)| time.elapsed() > Duration::from_secs(8))
        {
            app.notice = None;
        }
        let notice = app.notice.as_ref().map(|(s, _)| s.clone());
        if leds.lit != last_leds {
            last_leds = leds.lit;
            redraw = true;
        }
        if notice != last_notice {
            last_notice = notice;
            redraw = true;
        }
        if event_seen {
            redraw = true;
        }
        if redraw {
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
                    serial_focused: app.serial_focused,
                    serial_selection: app.serial_selection.as_ref(),
                    leds: leds.lit,
                    notice: app.notice.as_ref().map(|(s, _)| s.as_str()),
                    quitting: app.quitting,
                },
            )?;
            redraw = false;
        }
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
                let disk = base.join("smoke-disk-scsi0.raw");
                let cdrom = base.join("smoke-cd-scsi1.iso");
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
                let draft_before_cancel = app.overlay.as_ref().unwrap().draft.clone();
                let history_path = settings::companion_path(&app.config, ".picker.toml");
                let history_before_cancel = std::fs::read(&history_path)?;
                app.picked(Field::Scsi(0), Path::new(""));
                assert_eq!(app.overlay.as_ref().unwrap().draft, draft_before_cancel);
                assert_eq!(std::fs::read(history_path)?, history_before_cancel);
                let history = file_picker::History::load(&app.config)?;
                let fresh = Overlay::new(&app.settings);
                let base = app.config.parent().unwrap();
                assert_eq!(
                    history.location(&fresh, Field::Scsi(2), base),
                    Some(base.join("smoke-disk-scsi0.raw"))
                );
                assert_eq!(
                    history.location(&fresh, Field::Scsi(6), base),
                    Some(base.join("smoke-cd-scsi1.iso"))
                );
                // Detach/reselect ID 6 through the UI: use CD history and
                // restore its CD-ROM default without changing live settings.
                let cdrom = base.join("smoke-cd-scsi6.iso");
                std::fs::write(&cdrom, vec![0; 2048])?;
                app.overlay.as_mut().unwrap().selected = 6;
                app.key(Scancode::Delete, Mod::NOMOD, &video, canvas.window_mut())?;
                assert_eq!(
                    history.location(
                        app.overlay.as_ref().unwrap(),
                        Field::Scsi(6),
                        app.config.parent().unwrap(),
                    ),
                    Some(app.config.parent().unwrap().join("smoke-cd-scsi1.iso"))
                );
                app.picked(Field::Scsi(6), &cdrom);
                assert_eq!(
                    app.overlay.as_ref().unwrap().draft.scsi[6].kind,
                    TargetKind::Cdrom
                );
                assert!(app.settings.scsi[6].path.is_empty());
                app.key(Scancode::F9, Mod::NOMOD, &video, canvas.window_mut())?;
                let saved = Settings::load(&app.config)?;
                assert_eq!(saved.scsi[0].kind, TargetKind::Disk);
                assert_eq!(saved.scsi[1].kind, TargetKind::Cdrom);
                assert_eq!(saved.scsi[6].kind, TargetKind::Cdrom);
                assert_eq!(saved.scsi[6].path, cdrom.display().to_string());
                assert!(!saved.scsi[0].path.is_empty() && !saved.scsi[1].path.is_empty());
                app.key(Scancode::F9, Mod::NOMOD, &video, canvas.window_mut())?;
                app.key(Scancode::Right, Mod::NOMOD, &video, canvas.window_mut())?;
                app.key(Scancode::Right, Mod::NOMOD, &video, canvas.window_mut())?;
                app.notice = None;
                // Exercise creation through the same form and worker as the UI.
                // The native save dialog is covered separately; the smoke uses
                // its deterministic temporary path callback.
                let created = app.config.parent().unwrap().join("smoke-new-scsi3.chd");
                let o = app.overlay.as_mut().unwrap();
                o.selected = o
                    .rows()
                    .iter()
                    .position(|r| r.field == Field::NewDisk)
                    .unwrap();
                o.activate();
                assert_eq!(o.new_disk.as_ref().unwrap().target, 3);
                o.new_disk.as_mut().unwrap().size_mib = 1;
                app.picked(Field::NewDiskPath, &created);
                let o = app.overlay.as_mut().unwrap();
                o.selected = o
                    .rows()
                    .iter()
                    .position(|r| r.field == Field::CreateDisk)
                    .unwrap();
                app.activate(canvas.window())?;
                let (path, target) = app.creation.take().unwrap().finish()?;
                app.overlay
                    .as_mut()
                    .unwrap()
                    .disk_created(target, path.display().to_string());
                assert!(app.settings.scsi[3].path.is_empty());
                // Discard settings retains the explicitly created file.
                app.close_overlay();
                app.confirm = Some((Confirm::Unsaved, 1));
                app.confirm();
                assert!(created.is_file() && app.settings.scsi[3].path.is_empty());
                assert_eq!(
                    file_picker::History::load(&app.config)?.location(
                        &Overlay::new(&app.settings),
                        Field::Scsi(3),
                        app.config.parent().unwrap(),
                    ),
                    Some(created.clone())
                );
                app.key(Scancode::F9, Mod::NOMOD, &video, canvas.window_mut())?;
                app.key(Scancode::Right, Mod::NOMOD, &video, canvas.window_mut())?;
                app.key(Scancode::Right, Mod::NOMOD, &video, canvas.window_mut())?;
                app.picked(Field::Scsi(3), &created);
                app.save();
                assert_eq!(
                    Settings::load(&app.config)?.scsi[3].path,
                    created.display().to_string()
                );
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
                app.key(Scancode::Left, Mod::NOMOD, &video, canvas.window_mut())?;
                app.key(Scancode::Left, Mod::NOMOD, &video, canvas.window_mut())?;
                app.key(Scancode::Return, Mod::NOMOD, &video, canvas.window_mut())?;
                app.overlay.as_mut().unwrap().edit = Some("Original".into());
                app.key(Scancode::Return, Mod::NOMOD, &video, canvas.window_mut())?;
                assert!(app.settings.machine_label.is_none());
                app.key(Scancode::F9, Mod::NOMOD, &video, canvas.window_mut())?;
                assert_eq!(
                    Settings::load(&app.config)?.machine_label.as_deref(),
                    Some("Original")
                );
            }
            if count == 12 {
                app.key(Scancode::F9, Mod::NOMOD, &video, canvas.window_mut())?;
                let o = app.overlay.as_mut().unwrap();
                o.selected = o
                    .rows()
                    .iter()
                    .position(|r| r.field == Field::Model)
                    .unwrap();
                o.activate();
                o.selected = o
                    .rows()
                    .iter()
                    .position(|r| r.field == Field::Display)
                    .unwrap();
                o.activate();
                o.selected = o
                    .rows()
                    .iter()
                    .position(|r| r.field == Field::CpuCount)
                    .unwrap();
                o.activate();
                assert_eq!(o.draft.cpus, 2);
                assert_eq!(app.settings.cpus, 1);
                o.selected = o.rows().iter().position(|r| r.field == Field::Ram).unwrap();
                let old_ram = o.draft.ram_mib;
                o.activate();
                assert_ne!(o.draft.ram_mib, old_ram);
                let selected_ram = o.draft.ram_mib;
                o.selected = 1;
                o.activate();
                o.edit = Some("SS20 Solaris".into());
                app.key(Scancode::Return, Mod::NOMOD, &video, canvas.window_mut())?;
                app.key(Scancode::F9, Mod::NOMOD, &video, canvas.window_mut())?;
                let saved = Settings::load(&app.config)?;
                assert_eq!(saved.model, Model::Sparcstation20);
                assert_eq!(saved.display, Display::Sx);
                assert_eq!(saved.cpus, 2);
                assert_eq!(saved.ram_mib, selected_ram);
                assert_eq!(saved.machines["Original"].cpus, 1);
                assert_eq!(saved.machines["Original"].model, Model::Sparcstation5);
            }
            if count == 14 {
                app.key(Scancode::F9, Mod::NOMOD, &video, canvas.window_mut())?;
                app.key(Scancode::Return, Mod::NOMOD, &video, canvas.window_mut())?;
                assert_eq!(
                    app.overlay.as_ref().unwrap().draft.machine_label.as_deref(),
                    Some("Original")
                );
                assert_eq!(app.settings.machine_label.as_deref(), Some("SS20 Solaris"));
                app.key(Scancode::F9, Mod::NOMOD, &video, canvas.window_mut())?;
                assert_eq!(Settings::load(&app.config)?.model, Model::Sparcstation5);
                assert_eq!(Settings::load(&app.config)?.cpus, 1);
                assert_eq!(state.phase, Phase::Ready);
                app.key(Scancode::F9, Mod::NOMOD, &video, canvas.window_mut())?;
                app.notice = None;
            }
            if count == 16 {
                if let Some(path) = &options.screenshot {
                    canvas.read_pixels(None)?.save_bmp(path)?;
                }
                println!(
                    "SDL smoke test passed: paused launch, overlay editing, unsaved draft dialog, SCSI targets, picker history, named machines and saved settings."
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
        if app.quitting && state.shutdown_complete {
            if let Some(error) = state.error {
                return Err(error.into());
            }
            return Ok(());
        }
        if !redraw {
            let timeout = if state.phase == Phase::Running {
                16
            } else if app.notice.is_some() {
                50
            } else {
                100
            };
            pending_event = events.wait_event_timeout(Duration::from_millis(timeout));
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn options(args: &[&str]) -> Options {
        Options::parse_from(args.iter().map(|arg| (*arg).to_string()))
            .unwrap()
            .unwrap()
    }

    #[test]
    fn ram_cli_parses_order_independently_and_rejects_overflow() {
        for args in [
            &["--ram-mib", "256", "--machine", "ss5"][..],
            &["--machine", "ss5", "--ram-mib", "256"][..],
            &["--ram-mib", "256", "--cpus", "1", "--machine", "ss5"][..],
        ] {
            let mut settings = Settings::default();
            options(args).apply_hardware(&mut settings).unwrap();
            assert_eq!(settings.model, Model::Sparcstation5);
            assert_eq!(settings.ram_mib, 256);
        }
        for args in [
            &["--ram-mib"][..],
            &["--ram-mib", "-1"][..],
            &["--ram-mib", "4294967296"][..],
        ] {
            assert!(
                Options::parse_from(args.iter().map(|arg| (*arg).to_string())).is_err(),
                "{args:?}"
            );
        }
    }

    #[test]
    fn idle_parking_cli_is_explicitly_opt_in() {
        let mut settings = Settings::default();
        assert!(!settings.idle_parking);
        options(&["--idle-parking"])
            .apply_hardware(&mut settings)
            .unwrap();
        assert!(settings.idle_parking);
    }

    #[test]
    fn ram_cli_overrides_named_machine_and_validates_the_final_combination() {
        let mut settings = Settings::default();
        settings.save_as_machine("base").unwrap();
        settings.select_machine("base").unwrap();
        options(&["--machine-label", "base", "--ram-mib", "256"])
            .apply_hardware(&mut settings)
            .unwrap();
        assert_eq!(settings.ram_mib, 256);

        let mut ss20_sx = Settings {
            model: Model::Sparcstation20,
            display: Display::Sx,
            ..Settings::default()
        };
        assert!(
            options(&["--ram-mib", "512"])
                .apply_hardware(&mut ss20_sx)
                .is_err()
        );
        let mut ss20_cgsix = Settings {
            model: Model::Sparcstation20,
            display: Display::Cgsix,
            ..Settings::default()
        };
        options(&[
            "--ram-mib",
            "512",
            "--machine",
            "ss20",
            "--display",
            "cgsix",
        ])
        .apply_hardware(&mut ss20_cgsix)
        .unwrap();
        assert_eq!(ss20_cgsix.ram_mib, 512);

        let mut ss5 = Settings::default();
        assert!(
            options(&["--ram-mib", "240"])
                .apply_hardware(&mut ss5)
                .is_err()
        );
        let mut axil = Settings::default();
        options(&["--machine", "axil320", "--ram-mib", "512"])
            .apply_hardware(&mut axil)
            .unwrap();
        assert_eq!(axil.ram_mib, 512);
    }
}
