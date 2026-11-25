use super::*;
use helios::ultra5::{BootFlash, PowerOnReset, ResetNvram, Ultra5ResetBoard};
use helios::v9::V9Cpu;

pub(super) struct Ultra5 {
    board: Ultra5ResetBoard,
    cpu: V9Cpu,
}

impl Ultra5 {
    pub(super) fn new(settings: &Settings, flash: Vec<u8>, nvram: Vec<u8>) -> Result<Self, String> {
        let mut board = Ultra5ResetBoard::new(BootFlash::new(flash)?, ResetNvram::new(nvram)?);
        board.attach_keyboard();
        board.set_ethernet_link(settings.network);
        if !settings.ide_disk.is_empty() {
            board
                .attach_disk(&settings.ide_disk, !settings.disk_read_only)
                .map_err(|e| e.to_string())?;
        }
        let cd = &settings.scsi[6];
        if cd.kind == TargetKind::Cdrom && !cd.path.is_empty() {
            board.attach_cdrom(&cd.path).map_err(|e| e.to_string())?;
        }
        Ok(Self {
            board,
            cpu: V9Cpu::from(PowerOnReset::ultra_sparc_ii()),
        })
    }

    pub(super) fn service_network(&mut self, network: &mut Option<NatEngine>, now: Instant) -> u64 {
        super::service_network(&mut self.board, network, now)
    }

    pub(super) fn mouse_event(&mut self, event: MouseEvent) {
        match event {
            MouseEvent::Motion(dx, dy) => self.board.mouse_motion(dx, dy),
            MouseEvent::Button(button, down) => self.board.mouse_button(button, down),
        }
    }
    pub(super) fn batch(
        &mut self,
        keys: &mut VecDeque<(u8, bool)>,
        pressed: &mut [bool; 128],
        state: &mut Snapshot,
    ) {
        for _ in 0..2048 {
            if let Some(&(code, down)) = keys.front()
                && self.board.key(code, down)
            {
                pressed[usize::from(code)] = down;
                keys.pop_front();
            }
            let steps = match self.board.step_or_repeat(&mut self.cpu, 100_000) {
                Ok(steps) => steps,
                Err(error) => {
                    state.phase = Phase::Failed;
                    state.error = Some(format!(
                        "Ultra 5 stopped at PC={:#018x}: {error}",
                        self.cpu.pc()
                    ));
                    eprintln!("{}", state.error.as_ref().unwrap());
                    break;
                }
            };
            state.steps += steps;
            state.processor_steps[0] += steps;
            if steps > 1 {
                break;
            }
        }
        let [bytes, _] = self.board.take_serial_output();
        if !bytes.is_empty() {
            state.serial.push_str(&String::from_utf8_lossy(&bytes));
            if state.serial.len() > 8192 {
                let at = state.serial.ceil_char_boundary(state.serial.len() - 4096);
                state.serial.drain(..at);
            }
        }
    }
    pub(super) fn publish(
        &self,
        state: &mut Snapshot,
        shared: &Mutex<Snapshot>,
        frame: &mut FrameCache,
    ) {
        let activity = self.board.ide_activity();
        state.storage_activity[..activity.len()].copy_from_slice(&activity);
        state.pc = self.cpu.pc();
        state.secondary_pc = None;
        if let Some(size) = self.board.framebuffer_size() {
            state.video_size = size;
            blit_pixels(
                state,
                frame,
                self.board.framebuffer_generation(),
                |pixels| self.board.framebuffer_pixels(pixels),
            );
        } else if frame.scanout.take().is_some() {
            Arc::make_mut(&mut state.pixels).fill(0);
            state.frame += 1;
        }
        *shared.lock().unwrap() = state.clone();
    }
}

pub(super) enum Guest {
    Sun4m(Box<Machine>),
    Ultra5(Box<Ultra5>),
}
impl Guest {
    pub(super) fn nvram(&self) -> Vec<u8> {
        match self {
            Self::Sun4m(m) => m.bus.devices().unwrap().nvram.image(),
            Self::Ultra5(m) => m.board.nvram_image(),
        }
    }
    pub(super) fn publish(
        &self,
        state: &mut Snapshot,
        shared: &Mutex<Snapshot>,
        frame: &mut FrameCache,
    ) {
        match self {
            Self::Sun4m(m) => publish(m, state, shared, frame),
            Self::Ultra5(m) => m.publish(state, shared, frame),
        }
    }
}
