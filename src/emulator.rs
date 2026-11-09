use crate::settings::{
    Model, RAM_MIB, SCSI_TARGETS, Settings, TargetKind, atomic_write, companion_path,
};
use helios::{
    devices::{
        cgsix::{Cgsix, HEIGHT, WIDTH},
        keyboard::Keyboard,
        nvram::Nvram,
        serial::Channel,
    },
    machine::Machine,
    net::{NatConfig, NatEngine},
};
use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Ready,
    Loading,
    Running,
    Paused,
    Failed,
    Stopped,
}

impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "PAUSED / NOT BOOTED",
            Self::Loading => "LOADING",
            Self::Running => "RUNNING",
            Self::Paused => "PAUSED",
            Self::Failed => "STOPPED / CORE ERROR",
            Self::Stopped => "STOPPED",
        }
    }
}

#[derive(Clone)]
pub struct Snapshot {
    pub phase: Phase,
    pub pixels: Arc<Vec<u8>>,
    pub frame: u64,
    pub steps: u64,
    pub pc: u32,
    pub serial: String,
    pub error: Option<String>,
    pub has_machine: bool,
    pub shutdown_failed: bool,
    pub scsi_activity: [(u64, u64); SCSI_TARGETS],
    pub network_activity: u64,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            phase: Phase::Ready,
            pixels: Arc::new(vec![0; WIDTH * HEIGHT * 4]),
            frame: 0,
            steps: 0,
            pc: 0,
            serial: String::new(),
            error: None,
            has_machine: false,
            shutdown_failed: false,
            scsi_activity: [(0, 0); SCSI_TARGETS],
            network_activity: 0,
        }
    }
}

enum Command {
    Boot(Box<Settings>),
    Pause,
    Shutdown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MouseEvent {
    Motion(i32, i32),
    Button(u8, bool),
}

#[derive(Default)]
struct Input {
    queue: VecDeque<(u8, bool)>,
    mouse: VecDeque<MouseEvent>,
    mouse_buttons: u8,
    release: bool,
}

#[derive(Default)]
struct FrameCache {
    pixels: Vec<u32>,
    scanout: Option<u64>,
}

pub struct Emulator {
    command: SyncSender<Command>,
    snapshot: Arc<Mutex<Snapshot>>,
    input: Arc<Mutex<Input>>,
    abort: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Emulator {
    pub fn new(nvram: PathBuf) -> Result<Self, String> {
        let (command, receive) = mpsc::sync_channel(8);
        let snapshot = Arc::new(Mutex::new(Snapshot::default()));
        let input = Arc::new(Mutex::new(Input::default()));
        let abort = Arc::new(AtomicBool::new(false));
        let (out, keys, stop) = (snapshot.clone(), input.clone(), abort.clone());
        let worker = thread::Builder::new()
            .name("helios-machine".into())
            .spawn(move || run(receive, out, keys, stop, &nvram))
            .map_err(|e| e.to_string())?;
        Ok(Self {
            command,
            snapshot,
            input,
            abort,
            worker: Some(worker),
        })
    }

    pub fn snapshot(&self) -> Snapshot {
        self.snapshot.lock().unwrap().clone()
    }
    pub fn boot(&self, settings: Settings) -> Result<(), String> {
        self.send(Command::Boot(Box::new(settings)))
    }
    pub fn pause(&self) -> Result<(), String> {
        self.send(Command::Pause)
    }
    pub fn shutdown(&self) -> Result<(), String> {
        // Hold the publication lock until the previous failure is cleared.
        let mut snapshot = self.snapshot.lock().unwrap();
        self.send(Command::Shutdown)?;
        snapshot.shutdown_failed = false;
        Ok(())
    }
    fn send(&self, command: Command) -> Result<(), String> {
        self.command
            .try_send(command)
            .map_err(|e| format!("Machine command: {e}"))
    }
    pub fn key(&self, code: u8, down: bool) -> bool {
        let mut input = self.input.lock().unwrap();
        if input.queue.len() >= 256 {
            input.queue.clear();
            input.release = true;
            return false;
        }
        input.queue.push_back((code, down));
        true
    }
    pub fn release_keys(&self) {
        let mut input = self.input.lock().unwrap();
        input.queue.clear();
        input.mouse.clear();
        for button in 0..3 {
            if input.mouse_buttons & (1 << (2 - button)) != 0 {
                input.mouse.push_back(MouseEvent::Button(button, false));
            }
        }
        input.mouse_buttons = 0;
        input.release = true;
    }

    pub fn mouse_motion(&self, dx: f32, dy: f32) {
        let dx = dx.round() as i32;
        let dy = dy.round() as i32;
        if dx == 0 && dy == 0 {
            return;
        }
        let mut input = self.input.lock().unwrap();
        if input.mouse.len() < 256 {
            input.mouse.push_back(MouseEvent::Motion(dx, dy));
        }
    }

    pub fn mouse_button(&self, button: u8, down: bool) {
        if button > 2 {
            return;
        }
        let mut input = self.input.lock().unwrap();
        let mask = 1 << (2 - button);
        let old = input.mouse_buttons;
        input.mouse_buttons = if down {
            input.mouse_buttons | mask
        } else {
            input.mouse_buttons & !mask
        };
        if input.mouse_buttons != old && input.mouse.len() < 256 {
            input.mouse.push_back(MouseEvent::Button(button, down));
        }
    }
}

impl Drop for Emulator {
    fn drop(&mut self) {
        self.abort.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn read_rom(path: &str, label: &str, max: usize) -> Result<Vec<u8>, String> {
    if path.is_empty() {
        return Err(format!("Select {label} in F9 > General before booting"));
    }
    let metadata = fs::metadata(path).map_err(|e| format!("{label}: {path}: {e}"))?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > max as u64 {
        return Err(format!(
            "{label}: expected a nonempty regular file of at most {max} bytes"
        ));
    }
    fs::read(path).map_err(|e| format!("{label}: {e}"))
}

fn build(settings: &Settings, nvram: Nvram) -> Result<Machine, String> {
    settings.validate()?;
    let profile = settings.model.profile();
    let prom = read_rom(
        &settings.prom,
        settings.model.prom_label(),
        helios::machine::MAX_PROM_BYTES,
    )?;
    let mut machine = Machine::with_profile(profile, RAM_MIB as usize * 1024 * 1024, &prom, nvram)
        .map_err(|e| e.to_string())?;
    if settings.cgsix_rom.is_empty() {
        if settings.model.requires_cgsix() {
            return Err(format!(
                "Select a TurboGX FCode ROM for {} in F9 > General before booting",
                settings.model.label()
            ));
        }
    } else {
        let rom = read_rom(&settings.cgsix_rom, "TurboGX ROM (32 KiB)", 32768)?;
        let graphics = Cgsix::new(&rom).map_err(|e| format!("TurboGX ROM: {e:?}"))?;
        machine.install_cgsix(graphics).map_err(|e| e.to_string())?;
    }
    let devices = machine.bus.devices_mut().unwrap();
    devices.keyboard = Some(Keyboard::default());
    for (id, target) in settings.scsi.iter().enumerate() {
        if let Some(kind) = target.kind.media_kind()
            && !target.path.is_empty()
        {
            if target.kind == TargetKind::Disk && !settings.disk_read_only {
                devices
                    .scsi
                    .attach_disk_rw(id as u8, &target.path)
                    .map_err(|e| format!("SCSI {id}: {}: {e}", target.path))?;
            } else {
                devices
                    .scsi
                    .attach(id as u8, kind, &target.path)
                    .map_err(|e| format!("SCSI {id}: {}: {e}", target.path))?;
            }
        }
    }
    Ok(machine)
}

fn service_network(machine: &mut Machine, network: &mut Option<NatEngine>, now: Instant) -> u64 {
    let Some(network) = network.as_mut() else {
        return 0;
    };
    loop {
        let frame = machine.bus.devices_mut().unwrap().ethernet.take_frame();
        let Some(frame) = frame else {
            break;
        };
        network.input(&frame, now);
    }
    network.poll(now);
    while let Some(frame) = network.output() {
        if !machine
            .bus
            .devices_mut()
            .unwrap()
            .ethernet
            .receive_frame(&frame)
        {
            break;
        }
    }
    network.stats().guest_frames + network.stats().host_frames
}

fn load_nvram(path: &Path, model: Model) -> Result<Nvram, String> {
    match fs::read(path) {
        Ok(bytes) => Nvram::from_image(&bytes).map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(model.default_nvram()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

fn nvram_path_for(base: &Path, model: Model) -> PathBuf {
    match model {
        Model::Sparcstation5 => base.to_path_buf(),
        Model::Sparcstation20 => companion_path(base, ".ss20"),
        Model::Axil320 => companion_path(base, ".axil320"),
    }
}

fn blit_pixels(
    state: &mut Snapshot,
    frame_cache: &mut FrameCache,
    scanout: u64,
    draw: impl FnOnce(&mut [u32]),
) {
    if frame_cache.scanout == Some(scanout) {
        return;
    }
    if frame_cache.pixels.len() != WIDTH * HEIGHT {
        frame_cache.pixels.resize(WIDTH * HEIGHT, 0);
    }
    draw(&mut frame_cache.pixels);
    let rgba = Arc::make_mut(&mut state.pixels);
    if rgba.len() != WIDTH * HEIGHT * 4 {
        rgba.resize(WIDTH * HEIGHT * 4, 0);
    }
    for (chunk, pixel) in rgba
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(&frame_cache.pixels)
    {
        chunk.copy_from_slice(&[(pixel >> 16) as u8, (pixel >> 8) as u8, *pixel as u8, 255]);
    }
    frame_cache.scanout = Some(scanout);
    state.frame += 1;
}

fn publish(
    machine: &Machine,
    state: &mut Snapshot,
    shared: &Mutex<Snapshot>,
    frame_cache: &mut FrameCache,
) {
    if let Some(cgsix) = machine.bus.cgsix() {
        blit_pixels(state, frame_cache, cgsix.scanout_key(), |pixels| {
            cgsix.pixels(pixels)
        });
    } else if let Some(sx) = machine.bus.sx() {
        blit_pixels(state, frame_cache, sx.generation() ^ (1 << 63), |pixels| {
            sx.pixels(pixels)
        });
    }
    state.pc = machine.cpu.pc();
    state.scsi_activity = std::array::from_fn(|id| {
        machine
            .bus
            .devices()
            .unwrap()
            .scsi
            .target(id as u8)
            .map_or((0, 0), |target| target.activity())
    });
    *shared.lock().unwrap() = state.clone();
}

fn run(
    receive: mpsc::Receiver<Command>,
    shared: Arc<Mutex<Snapshot>>,
    input: Arc<Mutex<Input>>,
    abort: Arc<AtomicBool>,
    nvram_base: &Path,
) {
    let mut machine: Option<Machine> = None;
    let mut current_model: Option<Model> = None;
    let mut nvram_path = nvram_base.to_path_buf();
    let mut network: Option<NatEngine> = None;
    let mut state = Snapshot::default();
    let mut frame_at = Instant::now();
    let mut frame_cache = FrameCache::default();
    let mut pressed = [false; 128];
    let mut pending_keys: VecDeque<(u8, bool)> = VecDeque::new();
    let mut pending_mouse: VecDeque<MouseEvent> = VecDeque::new();
    let mut saved = false;
    loop {
        if abort.load(Ordering::Relaxed) {
            break;
        }
        let command = if state.phase == Phase::Running {
            receive.try_recv().ok()
        } else {
            receive.recv_timeout(Duration::from_millis(10)).ok()
        };
        if let Some(command) = command {
            match command {
                Command::Shutdown => {
                    if let Some(m) = &machine
                        && let Err(error) =
                            atomic_write(&nvram_path, &m.bus.devices().unwrap().nvram.image())
                    {
                        state.error = Some(error);
                        state.shutdown_failed = true;
                        if state.phase == Phase::Running {
                            state.phase = Phase::Paused;
                        }
                        publish(m, &mut state, &shared, &mut frame_cache);
                        continue;
                    }
                    state.error = None;
                    state.shutdown_failed = false;
                    saved = true;
                    break;
                }
                Command::Pause => {
                    state.phase = match state.phase {
                        Phase::Running => Phase::Paused,
                        Phase::Paused => Phase::Running,
                        other => other,
                    };
                }
                Command::Boot(settings) => {
                    let old_phase = state.phase;
                    shared.lock().unwrap().phase = Phase::Loading;
                    let target_path = nvram_path_for(nvram_base, settings.model);
                    let nvram = (|| {
                        if let (Some(m), Some(old_model)) = (&machine, current_model) {
                            if old_model == settings.model {
                                return Ok(m.bus.devices().unwrap().nvram.clone());
                            }
                            atomic_write(
                                &nvram_path_for(nvram_base, old_model),
                                &m.bus.devices().unwrap().nvram.image(),
                            )?;
                        }
                        load_nvram(&target_path, settings.model)
                    })();
                    match nvram.and_then(|n| build(&settings, n)) {
                        Ok(new) => {
                            machine = Some(new);
                            current_model = Some(settings.model);
                            nvram_path = target_path;
                            network = settings
                                .network
                                .then(|| NatEngine::new(NatConfig::default()));
                            state.phase = Phase::Running;
                            state.has_machine = true;
                            state.steps = 0;
                            state.network_activity = 0;
                            state.serial.clear();
                            state.error = None;
                            pressed.fill(false);
                            pending_keys.clear();
                            pending_mouse.clear();
                            let mut keys = input.lock().unwrap();
                            keys.queue.clear();
                            keys.mouse.clear();
                            keys.mouse_buttons = 0;
                            keys.release = false;
                        }
                        Err(error) => {
                            state.phase = old_phase;
                            state.error = Some(error);
                        }
                    }
                }
            }
            if let Some(m) = &machine {
                publish(m, &mut state, &shared, &mut frame_cache);
            } else {
                *shared.lock().unwrap() = state.clone();
            }
        }
        if state.phase != Phase::Running {
            continue;
        }
        let m = machine.as_mut().unwrap();
        {
            let mut keys = input.lock().unwrap();
            if keys.release {
                keys.release = false;
                keys.queue.clear();
                pending_keys.clear();
                pending_mouse.clear();
                pending_keys.extend((1..127).filter(|&i| pressed[i]).map(|i| (i as u8, false)));
                pending_mouse.append(&mut keys.mouse);
            } else {
                pending_keys.append(&mut keys.queue);
                pending_mouse.append(&mut keys.mouse);
            }
        }
        while let Some(event) = pending_mouse.pop_front() {
            match event {
                MouseEvent::Motion(dx, dy) => m.mouse_motion(dx, dy),
                MouseEvent::Button(button, down) => m.mouse_button(button, down),
            }
        }
        state.network_activity = service_network(m, &mut network, Instant::now());
        for _ in 0..2048 {
            if let Some(&(code, down)) = pending_keys.front()
                && m.bus
                    .devices_mut()
                    .unwrap()
                    .keyboard
                    .as_mut()
                    .unwrap()
                    .key(code, down)
            {
                pressed[code as usize] = down;
                pending_keys.pop_front();
            }
            match m.step() {
                Ok(_) => state.steps += 1,
                Err(error) if error.is_power_off() => {
                    state.phase = Phase::Stopped;
                    state.error = None;
                    state.serial.push_str("\nSystem powered off.\n");
                    break;
                }
                Err(error) => {
                    state.phase = Phase::Failed;
                    state.error = Some(format!("Helios stopped at PC={:08x}: {error:?}", error.pc));
                    eprintln!("{}", state.error.as_ref().unwrap());
                    break;
                }
            }
        }
        let bytes = m.bus.devices_mut().unwrap().serial.take_output(Channel::A);
        for byte in bytes {
            match byte {
                b'\n' => state.serial.push('\n'),
                b'\r' => {}
                0x20..=0x7e => state.serial.push(char::from(byte)),
                _ => {}
            }
        }
        if state.serial.len() > 8192 {
            state.serial.drain(..state.serial.len() - 8192);
        }
        if frame_at.elapsed() >= Duration::from_millis(33) || state.phase != Phase::Running {
            publish(m, &mut state, &shared, &mut frame_cache);
            frame_at = Instant::now();
        }
    }
    if let Some(m) = machine {
        if !saved
            && let Err(error) = atomic_write(&nvram_path, &m.bus.devices().unwrap().nvram.image())
        {
            eprintln!("{error}");
            state.error = Some(error);
        } else {
            // A prior core fault is already visible; it is not a state-save failure.
            state.error = None;
        }
        publish(&m, &mut state, &shared, &mut frame_cache);
    }
    state.phase = Phase::Stopped;
    *shared.lock().unwrap() = state;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn per_machine_nvram_paths_and_defaults_are_distinct() {
        use crate::settings::Model;
        let base = Path::new("/tmp/heliacal.cfg.nvram");
        assert_eq!(nvram_path_for(base, Model::Sparcstation5), base);
        assert_eq!(
            nvram_path_for(base, Model::Sparcstation20),
            Path::new("/tmp/heliacal.cfg.nvram.ss20")
        );
        assert_eq!(
            nvram_path_for(base, Model::Axil320),
            Path::new("/tmp/heliacal.cfg.nvram.axil320")
        );
        let ss5 = Model::Sparcstation5.default_nvram().image();
        let axil = Model::Axil320.default_nvram().image();
        assert_ne!(&ss5[0x1fd8..0x1fe8], &axil[0x1fd8..0x1fe8]);
    }
    #[test]
    fn machine_models_choose_the_expected_firmware_and_graphics() {
        use crate::settings::Model;
        let dir = tempfile::tempdir().unwrap();
        let prom = dir.path().join("prom");
        let rom = dir.path().join("rom");
        fs::write(&prom, helios::machine::serial_demo_image()).unwrap();
        fs::write(&rom, vec![0; 32768]).unwrap();
        let base = Settings {
            prom: prom.display().to_string(),
            cgsix_rom: rom.display().to_string(),
            ..Settings::default()
        };
        let ss20 = Settings {
            model: Model::Sparcstation20,
            cgsix_rom: String::new(),
            ..base.clone()
        };
        let machine = build(&ss20, Nvram::default()).unwrap();
        assert!(machine.bus.sx().is_some());
        assert!(machine.bus.cgsix().is_none());
        let ss20_gx = Settings {
            model: Model::Sparcstation20,
            ..base.clone()
        };
        let machine = build(&ss20_gx, Nvram::default()).unwrap();
        assert!(machine.bus.sx().is_some() && machine.bus.cgsix().is_some());
        let axil = Settings {
            model: Model::Axil320,
            ..base
        };
        let machine = build(&axil, Nvram::default()).unwrap();
        assert!(machine.bus.sx().is_none() && machine.bus.cgsix().is_some());
    }
    #[test]
    fn all_seven_scsi_ids_honor_the_selected_access_mode() {
        use crate::settings::{ScsiTarget, TargetKind};
        let dir = tempfile::tempdir().unwrap();
        let prom = dir.path().join("prom");
        let rom = dir.path().join("rom");
        let disk = dir.path().join("disk.raw");
        let cd = dir.path().join("cd.iso");
        fs::write(&prom, helios::machine::serial_demo_image()).unwrap();
        fs::write(&rom, vec![0; 32768]).unwrap();
        fs::write(&disk, vec![0; 512]).unwrap();
        fs::write(&cd, vec![0; 2048]).unwrap();
        let mut settings = Settings {
            prom: prom.display().to_string(),
            cgsix_rom: rom.display().to_string(),
            ..Settings::default()
        };
        for (id, target) in settings.scsi.iter_mut().enumerate() {
            *target = ScsiTarget {
                kind: if id % 2 == 0 {
                    TargetKind::Disk
                } else {
                    TargetKind::Cdrom
                },
                path: if id % 2 == 0 {
                    disk.display().to_string()
                } else {
                    cd.display().to_string()
                },
            };
        }
        let machine = build(&settings, Nvram::default()).unwrap();
        let bus = &machine.bus.devices().unwrap().scsi;
        for id in 0..7 {
            let target = bus.target(id).unwrap();
            let expected = if id % 2 == 0 {
                helios::devices::scsi::MediaKind::Disk
            } else {
                helios::devices::scsi::MediaKind::Cdrom
            };
            assert_eq!(target.kind(), expected);
            assert_eq!(
                target.writable(),
                expected == helios::devices::scsi::MediaKind::Disk
            );
            assert_eq!(target.blocks(), 1);
        }
        assert!(bus.target(7).is_none());
        settings.disk_read_only = true;
        let read_only = build(&settings, Nvram::default()).unwrap();
        for id in 0..7 {
            let target = read_only.bus.devices().unwrap().scsi.target(id).unwrap();
            assert!(!target.writable());
        }
        settings.scsi[0].kind = TargetKind::Disconnected;
        settings.scsi[0].path = "missing.raw".into();
        let replacement = build(&settings, Nvram::default()).unwrap();
        assert!(replacement.bus.devices().unwrap().scsi.target(0).is_none());
    }
    fn wait(emulator: &Emulator, check: impl Fn(&Snapshot) -> bool) -> Snapshot {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let state = emulator.snapshot();
            if check(&state) {
                return state;
            }
            assert!(
                Instant::now() < deadline,
                "worker timeout: {:?} {:?}",
                state.phase,
                state.error
            );
            thread::sleep(Duration::from_millis(5));
        }
    }
    #[test]
    fn real_core_boot_pause_failed_replacement_and_nvram_save() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("prom"),
            helios::machine::serial_demo_image(),
        )
        .unwrap();
        fs::write(dir.path().join("gx"), vec![0; 32768]).unwrap();
        let settings = Settings {
            prom: dir.path().join("prom").display().to_string(),
            cgsix_rom: dir.path().join("gx").display().to_string(),
            ..Settings::default()
        };
        let state_path = dir.path().join("state.nvram");
        let emu = Emulator::new(state_path.clone()).unwrap();
        assert_eq!(emu.snapshot().phase, Phase::Ready);
        emu.boot(settings).unwrap();
        wait(&emu, |s| s.serial.contains("HELIOS"));
        emu.pause().unwrap();
        let paused = wait(&emu, |s| s.phase == Phase::Paused);
        thread::sleep(Duration::from_millis(50));
        assert_eq!(emu.snapshot().steps, paused.steps);
        emu.boot(Settings::default()).unwrap();
        let failed = wait(&emu, |s| s.error.is_some());
        assert_eq!(failed.phase, Phase::Paused);
        assert_eq!(failed.steps, paused.steps);
        emu.pause().unwrap();
        wait(&emu, |s| s.steps > paused.steps);
        emu.shutdown().unwrap();
        assert!(wait(&emu, |s| s.phase == Phase::Stopped).error.is_none());
        assert_eq!(fs::read(state_path).unwrap().len(), 8192);
    }
    #[test]
    fn missing_firmware_is_recoverable_and_input_is_bounded() {
        let dir = tempfile::tempdir().unwrap();
        let emu = Emulator::new(dir.path().join("nvram")).unwrap();
        emu.boot(Settings::default()).unwrap();
        let state = wait(&emu, |s| s.error.is_some());
        assert_eq!(state.phase, Phase::Ready);
        assert!(!state.has_machine);
        for _ in 0..256 {
            assert!(emu.key(0x4d, true));
        }
        assert!(!emu.key(0x4d, false));
        assert!(emu.input.lock().unwrap().release);
        assert!(emu.input.lock().unwrap().queue.is_empty());
    }

    #[test]
    fn failed_nvram_save_retains_machine_for_retry() {
        let dir = tempfile::tempdir().unwrap();
        let prom = dir.path().join("prom");
        let rom = dir.path().join("rom");
        let state = dir.path().join("nvram");
        fs::write(&prom, helios::machine::serial_demo_image()).unwrap();
        fs::write(&rom, vec![0; 32768]).unwrap();
        let emu = Emulator::new(state.clone()).unwrap();
        emu.boot(Settings {
            prom: prom.display().to_string(),
            cgsix_rom: rom.display().to_string(),
            ..Settings::default()
        })
        .unwrap();
        wait(&emu, |s| s.steps > 0);
        fs::create_dir(&state).unwrap();
        emu.shutdown().unwrap();
        let rejected = wait(&emu, |s| s.shutdown_failed);
        assert_eq!(rejected.phase, Phase::Paused);
        assert!(rejected.has_machine);
        fs::remove_dir(&state).unwrap();
        emu.shutdown().unwrap();
        assert!(wait(&emu, |s| s.phase == Phase::Stopped).error.is_none());
        assert_eq!(fs::read(state).unwrap().len(), 8192);
    }
}
