use crate::serial_pty::SerialPty;
use crate::settings::{Display, Model, SCSI_TARGETS, Settings, TargetKind, atomic_write};
use helios::{
    devices::{
        cgsix::{Cgsix, HEIGHT, WIDTH},
        keyboard::Keyboard,
        nvram::Nvram,
        serial::Channel,
    },
    machine::{Machine, SchedulerError},
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
    pub secondary_pc: Option<u32>,
    /// Scheduler outcomes by populated module (hardware processors 0 and 2).
    pub processor_steps: [u64; 2],
    pub serial: String,
    pub serial_screen: Arc<vt100::Screen>,
    pub serial_frame: u64,
    pub headless: bool,
    pub serial_endpoint: Option<String>,
    pub serial_error: Option<String>,
    pub serial_dropped: u64,
    pub error: Option<String>,
    pub has_machine: bool,
    pub machine_name: Option<String>,
    pub machine_description: Option<String>,
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
            secondary_pc: None,
            processor_steps: [0; 2],
            serial: String::new(),
            serial_screen: Arc::new(vt100::Parser::default().screen().clone()),
            serial_frame: 0,
            headless: false,
            serial_endpoint: None,
            serial_error: None,
            serial_dropped: 0,
            error: None,
            has_machine: false,
            machine_name: None,
            machine_description: None,
            shutdown_failed: false,
            scsi_activity: [(0, 0); SCSI_TARGETS],
            network_activity: 0,
        }
    }
}

enum Command {
    Boot(Box<Settings>),
    Pause,
    SerialBreak,
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
    console: crate::serial_screen::SerialScreen,
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
    pub fn serial_break(&self) -> Result<(), String> {
        self.send(Command::SerialBreak)
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
    let nvram = crate::obp_console::select(settings.model, &prom, nvram, settings.headless)?;
    let graphics = if settings.headless {
        None
    } else {
        match settings.display {
            Display::Sx => {
                if !settings.model.supports_sx() {
                    return Err(format!(
                        "{} does not have a native SX display",
                        settings.model.label()
                    ));
                }
                None
            }
            Display::Cgsix => {
                if settings.cgsix_rom.is_empty() {
                    return Err(format!(
                        "Select a TurboGX FCode ROM for {} in F9 > General before booting",
                        settings.model.label()
                    ));
                }
                let rom = read_rom(&settings.cgsix_rom, "TurboGX ROM (32 KiB)", 32768)?;
                Some(Cgsix::new(&rom).map_err(|e| format!("TurboGX ROM: {e:?}"))?)
            }
        }
    };
    let mut machine = Machine::with_profile_processors_video(
        profile,
        settings.ram_bytes()?,
        &prom,
        nvram,
        usize::from(settings.cpus),
        settings.installed_video(),
    )
    .map_err(|e| e.to_string())?;
    if settings.accurate_cpu_clocks {
        machine.reset_with_clock_mode(helios::machine::CpuClockMode::Hardware);
    }
    if let Some(graphics) = graphics {
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
    if let Some(screen) = frame_cache.console.publish() {
        state.serial_screen = screen;
        state.serial_frame = state.serial_frame.wrapping_add(1);
    }
    state.pc = machine.cpu.pc();
    state.secondary_pc = machine.secondary_cpu().map(|module| module.cpu.pc());
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

// Choose once per machine topology; never mix legacy and scheduled stepping
// on a running dual-CPU machine. Single-CPU machines keep the accepted path.
fn step_machine(machine: &mut Machine) -> Result<u8, SchedulerError> {
    if machine.secondary_cpu().is_some() {
        machine.step_scheduled().map(|outcome| outcome.processor)
    } else {
        machine
            .step()
            .map(|_| 0)
            .map_err(|fault| SchedulerError::Cpu {
                processor: 0,
                fault,
            })
    }
}

#[derive(Default)]
struct SerialBridge {
    pty: Option<SerialPty>,
    epoch: u64,
    break_pending: bool,
    break_active: bool,
    break_until_step: u64,
}
impl SerialBridge {
    fn service(&mut self, machine: &mut Machine, running: bool, state: &mut Snapshot) {
        let Some(pty) = &mut self.pty else {
            return;
        };
        let serial = &mut machine.bus.devices_mut().unwrap().serial;
        let epoch = serial.receiver_epoch(Channel::A);
        if epoch != self.epoch {
            // A receiver reset discards input belonging to the previous session.
            pty.clear_input();
            self.epoch = epoch;
            self.break_active = false;
            self.break_pending = false;
        }
        if let Err(error) = pty.poll() {
            state.serial_error = Some(error);
        }
        state.serial_dropped = pty.dropped;
        if !running {
            return;
        }
        if self.break_active {
            // Hold across at least 1M execution outcomes, as in the accepted
            // original-OBP BREAK fixture. One batch can miss firmware polling.
            // This is an input pulse, not an advance of guest device time.
            if state.steps < self.break_until_step {
                return;
            }
            match serial.set_break(Channel::A, false) {
                Ok(()) => self.break_active = false,
                Err(helios::bus::BusErrorKind::WouldBlock) => {}
                Err(e) => {
                    state.serial_error = Some(format!("Serial BREAK release: {e:?}"));
                    self.break_active = false;
                }
            }
        } else if self.break_pending && serial.receiver_enabled(Channel::A) {
            self.break_pending = false;
            match serial.set_break(Channel::A, true) {
                Ok(()) => {
                    self.break_active = true;
                    self.break_until_step = state.steps.saturating_add(1_000_000);
                }
                Err(e) => state.serial_error = Some(format!("Serial BREAK: {e:?}")),
            }
        } else if let Some(&byte) = pty.input.front()
            && serial.receive(Channel::A, byte)
        {
            pty.input.pop_front();
        }
    }
}

fn run(
    receive: mpsc::Receiver<Command>,
    shared: Arc<Mutex<Snapshot>>,
    input: Arc<Mutex<Input>>,
    abort: Arc<AtomicBool>,
    nvram_base: &Path,
) {
    let mut machine: Option<Machine> = None;
    let mut nvram_path = nvram_base.to_path_buf();
    let mut network: Option<NatEngine> = None;
    let mut state = Snapshot::default();
    let mut frame_at = Instant::now();
    let mut frame_cache = FrameCache::default();
    let mut pressed = [false; 128];
    let mut pending_keys: VecDeque<(u8, bool)> = VecDeque::new();
    let mut pending_mouse: VecDeque<MouseEvent> = VecDeque::new();
    let mut saved = false;
    let mut serial = SerialBridge::default();
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
                Command::SerialBreak => {
                    if serial.pty.is_some() {
                        serial.break_pending = true;
                    }
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
                    let target_path = settings.nvram_path(nvram_base);
                    let nvram = (|| {
                        if let Some(m) = &machine {
                            if nvram_path == target_path {
                                return Ok(m.bus.devices().unwrap().nvram.clone());
                            }
                            atomic_write(&nvram_path, &m.bus.devices().unwrap().nvram.image())?;
                        }
                        load_nvram(&target_path, settings.model)
                    })();
                    let replacement = nvram.and_then(|n| build(&settings, n)).and_then(|new| {
                        // Reserve a different endpoint before replacing the running machine.
                        let reuse = settings.headless
                            && serial
                                .pty
                                .as_ref()
                                .is_some_and(|p| p.alias_matches(&settings.serial_pty_link));
                        let pty = if settings.headless && !reuse {
                            Some(SerialPty::open(&settings.serial_pty_link)?)
                        } else {
                            None
                        };
                        Ok((new, pty, reuse))
                    });
                    match replacement {
                        Ok((new, pty, reuse)) => {
                            if !reuse {
                                serial.pty = pty;
                            }
                            if let Some(p) = &mut serial.pty {
                                p.reset();
                            }
                            serial.epoch =
                                new.bus.devices().unwrap().serial.receiver_epoch(Channel::A);
                            serial.break_pending = false;
                            serial.break_active = false;
                            state.headless = settings.headless;
                            state.serial_error = None;
                            state.serial_dropped = 0;
                            state.serial_endpoint = serial
                                .pty
                                .as_ref()
                                .map(|p| p.endpoint().display().to_string());
                            if let Some(p) = &serial.pty {
                                eprintln!(
                                    "TTYA serial console: {} -> {} (9600 8N1)",
                                    p.endpoint().display(),
                                    p.path().display()
                                );
                            }
                            machine = Some(new);
                            nvram_path = target_path;
                            network = settings
                                .network
                                .then(|| NatEngine::new(NatConfig::default()));
                            state.phase = Phase::Running;
                            state.has_machine = true;
                            state.machine_name = Some(settings.machine_name());
                            state.machine_description = Some(settings.machine_description());
                            state.steps = 0;
                            state.processor_steps = [0; 2];
                            state.network_activity = 0;
                            state.serial.clear();
                            state.error = None;
                            state.pixels = Arc::new(vec![0; WIDTH * HEIGHT * 4]);
                            state.frame = state.frame.wrapping_add(1);
                            frame_cache = FrameCache::default();
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
        if let Some(m) = machine.as_mut() {
            serial.service(m, state.phase == Phase::Running, &mut state);
            if state.phase != Phase::Running {
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
            match step_machine(m) {
                Ok(processor) => {
                    state.steps += 1;
                    state.processor_steps[usize::from(processor / 2)] += 1;
                }
                Err(SchedulerError::Cpu { fault, .. }) if fault.is_power_off() => {
                    state.phase = Phase::Stopped;
                    state.error = None;
                    state.serial.push_str("\nSystem powered off.\n");
                    break;
                }
                Err(error) => {
                    state.phase = Phase::Failed;
                    state.error = Some(format!("Helios stopped: {error}"));
                    eprintln!("{}", state.error.as_ref().unwrap());
                    break;
                }
            }
        }
        let bytes = m.bus.devices_mut().unwrap().serial.take_output(Channel::A);
        if let Some(p) = &mut serial.pty {
            p.send(&bytes);
        }
        frame_cache.console.process(&bytes);
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
    drop(serial); // Remove only our alias before publishing final shutdown.
    state.serial_endpoint = None;
    state.phase = Phase::Stopped;
    *shared.lock().unwrap() = state;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dual_cpu_test_settings(dir: &Path, model: Model) -> Settings {
        // Both CPUs execute reset firmware. CPU0 releases CPU2 through a
        // guest STA to AER; no host-set entry point or arbitration override.
        let program: [u32; 7] = [
            0x0338_0000, // sethi %hi(0xe0000000), %g1
            0x8200_6fff, // add %g1, 0xfff, %g1
            0x8200_6009, // add %g1, 9, %g1 -> AER low word
            0x8410_2004, // or %g0, 4, %g2 (enable hardware CPU2)
            0xc4a0_45e0, // sta %g2, [%g1] ASI 0x2f
            0x1080_0000, // ba .
            0x0100_0000, // nop
        ];
        fs::write(
            dir.join("prom"),
            program
                .into_iter()
                .flat_map(u32::to_be_bytes)
                .collect::<Vec<_>>(),
        )
        .unwrap();
        fs::write(dir.join("gx"), vec![0; 32768]).unwrap();
        Settings {
            model,
            cpus: 2,
            network: false,
            display: if model == Model::Sparcstation20 {
                Display::Sx
            } else {
                Display::Cgsix
            },
            prom: dir.join("prom").display().to_string(),
            cgsix_rom: dir.join("gx").display().to_string(),
            ..Settings::default()
        }
    }

    #[test]
    fn dual_worker_runs_both_cpus_pauses_and_replaces_with_one() {
        for model in [Model::Sparcstation20, Model::Axil320] {
            let dir = tempfile::tempdir().unwrap();
            let settings = dual_cpu_test_settings(dir.path(), model);
            let base = dir.path().join("state.nvram");
            let emu = Emulator::new(base.clone()).unwrap();
            emu.boot(settings.clone()).unwrap();
            let running = wait(&emu, |s| s.processor_steps.iter().all(|&n| n > 10));
            assert_eq!(running.phase, Phase::Running);
            assert!(running.secondary_pc.is_some());
            assert!(
                running
                    .machine_description
                    .as_ref()
                    .unwrap()
                    .contains("2 CPUs")
            );
            assert_eq!(running.steps, running.processor_steps.iter().sum::<u64>());
            emu.pause().unwrap();
            let paused = wait(&emu, |s| s.phase == Phase::Paused);
            thread::sleep(Duration::from_millis(40));
            assert_eq!(paused.processor_steps, emu.snapshot().processor_steps);
            // Invalid replacements must preserve both processors and paused state.
            emu.boot(Settings {
                cpus: 4,
                ..settings.clone()
            })
            .unwrap();
            let rejected = wait(&emu, |s| s.error.is_some());
            assert_eq!(rejected.phase, Phase::Paused);
            assert_eq!(rejected.processor_steps, paused.processor_steps);
            emu.pause().unwrap();
            wait(&emu, |s| s.processor_steps[1] > paused.processor_steps[1]);
            emu.boot(Settings {
                cpus: 1,
                ..settings.clone()
            })
            .unwrap();
            let single = wait(&emu, |s| {
                s.secondary_pc.is_none() && s.processor_steps[0] > 10
            });
            assert_eq!(single.processor_steps[1], 0);
            assert!(
                single
                    .machine_description
                    .as_ref()
                    .unwrap()
                    .contains("1 CPU /")
            );
            emu.shutdown().unwrap();
            assert!(wait(&emu, |s| s.phase == Phase::Stopped).error.is_none());
            assert_eq!(fs::read(settings.nvram_path(&base)).unwrap().len(), 8192);
        }
    }

    #[test]
    fn secondary_execution_errors_keep_the_hardware_processor_tag() {
        let dir = tempfile::tempdir().unwrap();
        let settings = dual_cpu_test_settings(dir.path(), Model::Sparcstation20);
        let mut m = build(&settings, settings.model.default_nvram()).unwrap();
        // Error attribution fixture: deliberately invalid secondary instruction.
        m.load_image(0x1000, &[0; 4]).unwrap();
        m.set_processor_entry(2, 0x1000, 0x1004);
        let error = (0..32).find_map(|_| step_machine(&mut m).err()).unwrap();
        assert!(matches!(error, SchedulerError::Cpu { processor: 2, fault } if fault.pc == 0x1000));
        assert!(error.to_string().contains("processor 2"));
    }

    #[test]
    fn per_machine_nvram_paths_and_defaults_are_distinct() {
        use crate::settings::Model;
        let base = Path::new("/tmp/heliacal.cfg.nvram");
        assert_eq!(Settings::default().nvram_path(base), base);
        assert_eq!(
            Settings {
                model: Model::Sparcstation20,
                ..Settings::default()
            }
            .nvram_path(base),
            Path::new("/tmp/heliacal.cfg.nvram.ss20")
        );
        assert_eq!(
            Settings {
                model: Model::Axil320,
                ..Settings::default()
            }
            .nvram_path(base),
            Path::new("/tmp/heliacal.cfg.nvram.axil320")
        );
        let ss5 = Model::Sparcstation5.default_nvram().image();
        let axil = Model::Axil320.default_nvram().image();
        assert_ne!(&ss5[0x1fd8..0x1fe8], &axil[0x1fd8..0x1fe8]);
    }
    #[cfg(unix)]
    #[test]
    fn serial_break_holds_across_batches_retries_full_fifo_and_flushes_receiver_epoch() {
        let mut machine = Machine::ss5(8 * 1024 * 1024, &[0; 4]).unwrap();
        let serial = &mut machine.bus.devices_mut().unwrap().serial;
        serial.write(4, 4).unwrap();
        serial.write(4, 0x44).unwrap();
        serial.write(4, 3).unwrap();
        serial.write(4, 0xc1).unwrap();
        for b in [1, 2, 3] {
            assert!(serial.receive(Channel::A, b));
        }
        let mut bridge = SerialBridge {
            pty: Some(SerialPty::open("").unwrap()),
            epoch: serial.receiver_epoch(Channel::A),
            break_pending: true,
            ..SerialBridge::default()
        };
        let mut state = Snapshot::default();
        bridge.service(&mut machine, false, &mut state);
        assert!(!bridge.break_active); // Paused commands do not mutate the SCC.
        bridge.service(&mut machine, true, &mut state);
        assert!(bridge.break_active);
        state.steps = 999_999;
        bridge.service(&mut machine, true, &mut state);
        assert_ne!(
            machine.bus.devices_mut().unwrap().serial.read(4).unwrap() & 0x80,
            0
        );
        state.steps = 1_000_000;
        bridge.service(&mut machine, true, &mut state);
        assert!(bridge.break_active); // NUL termination cannot overrun the full FIFO.
        assert!(state.serial_error.is_none());
        assert_eq!(
            machine.bus.devices_mut().unwrap().serial.read(6).unwrap(),
            1
        );
        bridge.service(&mut machine, true, &mut state);
        assert!(!bridge.break_active);
        for expected in [2, 3, 0] {
            assert_eq!(
                machine.bus.devices_mut().unwrap().serial.read(6).unwrap(),
                expected
            );
        }
        bridge.pty.as_mut().unwrap().input.extend(b"old input");
        let serial = &mut machine.bus.devices_mut().unwrap().serial;
        serial.write(4, 3).unwrap();
        serial.write(4, 0xc1).unwrap();
        bridge.service(&mut machine, false, &mut state);
        assert!(bridge.pty.as_ref().unwrap().input.is_empty());
    }
    #[test]
    fn headless_profiles_allocate_no_framebuffer_or_vsimm_and_keep_keyboard_presence() {
        let dir = tempfile::tempdir().unwrap();
        let prom = dir.path().join("prom");
        fs::write(&prom, [0; 4]).unwrap();
        for (model, ram) in [
            (Model::Sparcstation5, 256),
            (Model::Sparcstation20, 512),
            (Model::Axil320, 512),
        ] {
            let settings = Settings {
                model,
                headless: true,
                ram_mib: ram,
                display: if model == Model::Sparcstation20 {
                    Display::Sx
                } else {
                    Display::Cgsix
                },
                prom: prom.display().to_string(),
                cgsix_rom: "does-not-exist".into(),
                ..Settings::default()
            };
            let m = build(&settings, model.default_nvram()).unwrap();
            assert!(m.bus.sx().is_none() && m.bus.cgsix().is_none());
            assert!(m.bus.devices().unwrap().keyboard.is_some());
            assert_eq!(
                m.bus.dram().unwrap().bank_sizes().iter().sum::<usize>(),
                ram as usize * 1024 * 1024
            );
        }
    }
    #[test]
    fn worker_build_applies_both_clock_modes_to_each_profile() {
        use crate::settings::Model;
        use helios::machine::CpuClockMode;
        let dir = tempfile::tempdir().unwrap();
        let prom = dir.path().join("prom");
        fs::write(&prom, helios::machine::serial_demo_image()).unwrap();
        for model in Model::ALL {
            for accurate_cpu_clocks in [false, true] {
                let settings = Settings {
                    model,
                    headless: true,
                    ram_mib: 16,
                    cpus: if model == Model::Sparcstation5 { 1 } else { 2 },
                    prom: prom.display().to_string(),
                    accurate_cpu_clocks,
                    ..Settings::default()
                };
                let machine = build(&settings, model.default_nvram()).unwrap();
                assert_eq!(
                    machine.clock_mode(),
                    if accurate_cpu_clocks {
                        CpuClockMode::Hardware
                    } else {
                        CpuClockMode::Fast
                    }
                );
                assert_eq!(
                    machine.cpu_clock_hz(),
                    if accurate_cpu_clocks {
                        model.profile().cpu_clock_hz()
                    } else {
                        10_000_000
                    }
                );
            }
        }
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
            display: Display::Sx,
            cgsix_rom: String::new(),
            ..base.clone()
        };
        let machine = build(&ss20, Nvram::default()).unwrap();
        assert!(machine.bus.sx().is_some() && machine.bus.cgsix().is_none());
        let ss20_gx = Settings {
            model: Model::Sparcstation20,
            display: Display::Cgsix,
            ..base.clone()
        };
        let machine = build(&ss20_gx, Nvram::default()).unwrap();
        assert!(machine.bus.sx().is_none() && machine.bus.cgsix().is_some());
        let axil = Settings {
            model: Model::Axil320,
            ..base
        };
        let machine = build(&axil, Nvram::default()).unwrap();
        assert!(machine.bus.sx().is_none() && machine.bus.cgsix().is_some());
    }
    #[test]
    fn worker_build_uses_selected_ram_and_video_without_a_transient_sx() {
        use crate::settings::Model;
        const MIB: u64 = 1024 * 1024;
        let dir = tempfile::tempdir().unwrap();
        let prom = dir.path().join("prom");
        let rom = dir.path().join("rom");
        fs::write(&prom, helios::machine::serial_demo_image()).unwrap();
        fs::write(&rom, vec![0; 32768]).unwrap();
        for (model, display, ram_mib, sx, cgsix) in [
            (Model::Sparcstation5, Display::Cgsix, 256, false, true),
            (Model::Sparcstation20, Display::Sx, 448, true, false),
            (Model::Sparcstation20, Display::Cgsix, 512, false, true),
            (Model::Axil320, Display::Cgsix, 512, false, true),
        ] {
            let settings = Settings {
                model,
                display,
                ram_mib,
                prom: prom.display().to_string(),
                cgsix_rom: rom.display().to_string(),
                ..Settings::default()
            };
            let machine = build(&settings, Nvram::default()).unwrap();
            let dram = machine.bus.dram().unwrap();
            let aperture = if model == Model::Sparcstation20 && display == Display::Sx {
                512
            } else {
                ram_mib
            };
            assert_eq!(dram.aperture_bytes(), u64::from(aperture) * MIB);
            assert_eq!(
                dram.bank_sizes().iter().sum::<usize>() as u64,
                u64::from(ram_mib) * MIB,
                "{model:?}/{display:?}"
            );
            assert_eq!(machine.bus.sx().is_some(), sx, "{model:?}/{display:?}");
            assert_eq!(
                machine.bus.cgsix().is_some(),
                cgsix,
                "{model:?}/{display:?}"
            );
        }

        let invalid = Settings {
            ram_mib: 240,
            prom: prom.display().to_string(),
            cgsix_rom: rom.display().to_string(),
            ..Settings::default()
        };
        assert!(build(&invalid, Nvram::default()).is_err());
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
            ram_mib: 256,
            prom: dir.path().join("prom").display().to_string(),
            cgsix_rom: dir.path().join("gx").display().to_string(),
            ..Settings::default()
        };
        let state_path = dir.path().join("state.nvram");
        let emu = Emulator::new(state_path.clone()).unwrap();
        assert_eq!(emu.snapshot().phase, Phase::Ready);
        emu.boot(settings.clone()).unwrap();
        wait(&emu, |s| s.serial.contains("HELIOS"));
        emu.pause().unwrap();
        let paused = wait(&emu, |s| s.phase == Phase::Paused);
        assert!(
            paused
                .machine_description
                .as_deref()
                .is_some_and(|description| description.contains("256 MiB"))
        );
        thread::sleep(Duration::from_millis(50));
        assert_eq!(emu.snapshot().steps, paused.steps);
        emu.boot(Settings {
            ram_mib: 240,
            ..settings
        })
        .unwrap();
        let failed_ram = wait(&emu, |s| {
            s.error.as_deref().is_some_and(|e| e.contains("RAM"))
        });
        assert_eq!(failed_ram.phase, Phase::Paused);
        assert_eq!(failed_ram.steps, paused.steps);
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
    fn named_workstations_of_the_same_model_keep_nvram_and_failed_switches_separate() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("state.nvram");
        fs::write(
            dir.path().join("prom"),
            helios::machine::serial_demo_image(),
        )
        .unwrap();
        fs::write(dir.path().join("gx"), vec![0; 32768]).unwrap();
        let mut settings = Settings {
            prom: dir.path().join("prom").display().to_string(),
            cgsix_rom: dir.path().join("gx").display().to_string(),
            ..Settings::default()
        };
        settings.save_as_machine("A").unwrap();
        settings.save_as_machine("B").unwrap();
        let path_b = settings.nvram_path(&base);
        settings.select_machine("A").unwrap();
        let path_a = settings.nvram_path(&base);
        for (path, marker) in [(&path_a, 0x11), (&path_b, 0x22)] {
            let mut bytes = Nvram::default().image();
            bytes[0x100] = marker;
            atomic_write(path, &bytes).unwrap();
        }
        let emu = Emulator::new(base).unwrap();
        emu.boot(settings.clone()).unwrap();
        wait(&emu, |s| s.serial.contains("HELIOS"));
        // A reset of the same workstation must retain its live NVRAM.
        let mut external = Nvram::default().image();
        external[0x100] = 0x33;
        fs::write(&path_a, external).unwrap();
        emu.boot(settings.clone()).unwrap();
        emu.pause().unwrap(); // FIFO ensures the reset has completed.
        wait(&emu, |s| s.phase == Phase::Paused);
        settings.select_machine("B").unwrap();
        emu.boot(settings.clone()).unwrap();
        wait(&emu, |s| s.machine_name.as_deref() == Some("B / SS-5"));
        assert_eq!(fs::read(&path_a).unwrap()[0x100], 0x11);
        emu.pause().unwrap();
        let paused = wait(&emu, |s| s.phase == Phase::Paused);
        settings.save_as_machine("Bad").unwrap();
        settings.prom = dir.path().join("missing").display().to_string();
        let bad_path = settings.nvram_path(&dir.path().join("state.nvram"));
        emu.boot(settings).unwrap();
        let rejected = wait(&emu, |s| s.error.is_some());
        assert_eq!(rejected.phase, Phase::Paused);
        assert_eq!(rejected.machine_name.as_deref(), Some("B / SS-5"));
        assert_eq!(rejected.steps, paused.steps);
        assert!(!bad_path.exists());
        emu.shutdown().unwrap();
        assert!(wait(&emu, |s| s.phase == Phase::Stopped).error.is_none());
        assert_eq!(fs::read(&path_b).unwrap()[0x100], 0x22);
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
