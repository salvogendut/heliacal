use super::*;
use std::cell::RefCell;

use helios::bus::Bus;
use helios::cpu::{PSR_ET, PSR_PS, PSR_S, Step};
use helios::machine::{
    InstalledVideo, Machine, MachineProfile, SOLARIS_IDLE_NPC, SOLARIS_IDLE_PC, SOLARIS_IDLE_PSR,
};
use helios::mmu::{PCR_EN, Width};

const LOOP: [u8; 8] = [0x10, 0x80, 0, 0, 1, 0, 0, 0];

// This deliberately reproduces the submitted tests' admission fixture. It has
// no instruction or data mapping at the supposedly recognized Solaris PC.
fn address_only_machine() -> Machine {
    let mut m = Machine::with_profile_processors_video(
        MachineProfile::Ss20,
        16 * 1024 * 1024,
        &LOOP,
        MachineProfile::Ss20.default_nvram(),
        1,
        InstalledVideo::None,
    )
    .unwrap();
    let words = [0x8188_6000u32, 0x80a0_0000u32];
    let code: Vec<_> = words.iter().flat_map(|word| word.to_be_bytes()).collect();
    m.load_image(0x1000, &code).unwrap();
    assert!(m.set_processor_entry(0, 0x1000, 0x1004));
    m.cpu.write_reg(1, PSR_S | PSR_PS | PSR_ET);
    m.cpu.step(&mut m.bus).unwrap();
    m.cpu.step(&mut m.bus).unwrap();
    m.cpu.set_pc_npc(SOLARIS_IDLE_PC, SOLARIS_IDLE_NPC);
    assert_eq!(m.cpu.psr(), SOLARIS_IDLE_PSR);
    m
}

// Reconstruct the actual captured loop and its three load dependencies. An L1
// 16 MiB mapping translates f0xxxxxx to physical 00xxxxxx, with R/M pre-set.
// Eight complete warmup iterations make the MMU diagnostic view steady too.
pub fn idle_machine() -> Machine {
    let mut m = address_only_machine();
    for (pa, word) in [
        (0x1000, 0x201),
        (0x23c0, 0xee),
        (0x06fa24, 0xd00220ec),
        (0x06fa28, 0x80a22001),
        (0x06fa2c, 0x3280000f),
        (0x06fa30, 0x01000000), // annulled on the captured path
        (0x06fa34, 0xd0062058),
        (0x06fa38, 0x80a22000),
        (0x06fa3c, 0x02800006),
        (0x06fa40, 0x213c097e),
        (0x06fa54, 0xe00421e8),
        (0x06fa58, 0x9fc40000),
        (0x06fa5c, 0x01000000),
        (0x071708, 0x81c3e008),
        (0x07170c, 0x01000000),
        (0x06fa60, 0x10bffff1),
        (0x06fa64, 0x113c097e),
        (0x25f8ec, 1),
        (0x26dea0, 0),
        (0x25f9e8, 0xf0071708),
    ] {
        m.bus.write32(pa, word).unwrap();
    }
    m.cpu
        .mmu
        .store(&mut m.bus, 0x100, 4, Width::Word, 0x100)
        .unwrap();
    m.cpu
        .mmu
        .store(&mut m.bus, 0, 4, Width::Word, PCR_EN)
        .unwrap();
    m.cpu.write_reg(8, 0xf025f800);
    m.cpu.write_reg(24, 0xf026de48);
    m.cpu.write_reg(16, 0xf0071708);
    m.cpu.write_reg(15, 0xf006fa58);
    for _ in 0..8 {
        iteration(&mut m);
    }
    m
}

pub fn iteration(m: &mut Machine) {
    for expected in [
        0xf006fa24, 0xf006fa28, 0xf006fa2c, 0xf006fa34, 0xf006fa38, 0xf006fa3c, 0xf006fa40,
        0xf006fa54, 0xf006fa58, 0xf006fa5c, 0xf0071708, 0xf007170c, 0xf006fa60, 0xf006fa64,
    ] {
        assert_eq!(m.cpu.pc(), expected);
        assert!(matches!(
            m.cpu.step(&mut m.bus).unwrap(),
            Step::Executed { .. }
        ));
    }
    assert_eq!(m.cpu.pc(), SOLARIS_IDLE_PC);
    assert_eq!(m.cpu.npc(), SOLARIS_IDLE_NPC);
    assert_eq!(m.cpu.psr(), SOLARIS_IDLE_PSR);
}

fn context<'a>(
    input: &'a Mutex<Input>,
    receive: &'a mpsc::Receiver<Command>,
    pending_command: &'a mut Option<Command>,
    serial: &'a mut SerialBridge,
    network: &'a mut Option<NatEngine>,
    abort: &'a AtomicBool,
) -> ParkContext<'a> {
    ParkContext {
        input,
        receive,
        pending_command,
        serial,
        pending_keys: false,
        network,
        abort,
    }
}

#[test]
fn parking_waits_before_committing_virtual_time() {
    let mut machine = idle_machine();
    let input = Mutex::new(Input::default());
    let (send, receive) = mpsc::sync_channel(4);
    drop(send);
    let mut pending_command = None;
    let mut serial = SerialBridge::default();
    let mut network = None;
    let abort = AtomicBool::new(false);
    let before = machine.virtual_ns();
    let plan = {
        let mut cx = context(
            &input,
            &receive,
            &mut pending_command,
            &mut serial,
            &mut network,
            &abort,
        );
        prepare_park(&machine, &mut cx).expect("idle plan")
    };
    assert!(plan.wait_ns > 0);
    let waits = RefCell::new(Vec::new());
    {
        let mut cx = context(
            &input,
            &receive,
            &mut pending_command,
            &mut serial,
            &mut network,
            &abort,
        );
        commit_park_with(
            &mut machine,
            plan,
            &mut cx,
            |duration| waits.borrow_mut().push(duration),
            || Duration::from_nanos(500_000),
        )
        .expect("committed wait");
    }
    assert_eq!(waits.borrow().len(), 1);
    assert_eq!(
        machine.virtual_ns(),
        before + 500_000,
        "virtual time was committed before the host wait elapsed"
    );
}

#[test]
fn short_deadline_limits_the_host_wait_budget() {
    let mut machine = idle_machine();
    // Timer count starts at one; limit 2001 gives the 1 ms fixture event.
    machine
        .bus
        .write32(machine.bus.layout().system_timer, 2001 << 9)
        .unwrap();
    let input = Mutex::new(Input::default());
    let (_send, receive) = mpsc::sync_channel(4);
    let mut pending_command = None;
    let mut serial = SerialBridge::default();
    let mut network = None;
    let abort = AtomicBool::new(false);
    let mut cx = context(
        &input,
        &receive,
        &mut pending_command,
        &mut serial,
        &mut network,
        &abort,
    );
    let plan = prepare_park(&machine, &mut cx).expect("idle plan");
    assert_eq!(plan.wait_ns, 1_000_000 - (machine.virtual_ns() % 1_000_000));
    assert!(plan.wait_ns < PARK_BUDGET_NS);
}

#[test]
fn queued_command_and_gui_input_reject_parking() {
    let machine = idle_machine();
    let input = Mutex::new(Input::default());
    let (send, receive) = mpsc::sync_channel(4);
    let mut pending_command = None;
    let mut serial = SerialBridge::default();
    let mut network = None;
    let abort = AtomicBool::new(false);
    send.try_send(Command::Pause).unwrap();
    {
        let mut cx = context(
            &input,
            &receive,
            &mut pending_command,
            &mut serial,
            &mut network,
            &abort,
        );
        assert!(prepare_park(&machine, &mut cx).is_none());
    }
    assert!(pending_command.is_some());

    let mut pending_command = None;
    input.lock().unwrap().queue.push_back((1, true));
    let mut cx = context(
        &input,
        &receive,
        &mut pending_command,
        &mut serial,
        &mut network,
        &abort,
    );
    assert!(prepare_park(&machine, &mut cx).is_none());
}

#[test]
fn pending_or_active_break_rejects_parking() {
    let machine = idle_machine();
    let input = Mutex::new(Input::default());
    let (_send, receive) = mpsc::sync_channel(4);
    let mut pending_command = None;
    let mut network = None;
    let abort = AtomicBool::new(false);
    for (pending, active) in [(true, false), (false, true)] {
        let mut serial = SerialBridge {
            break_pending: pending,
            break_active: active,
            ..SerialBridge::default()
        };
        let mut cx = context(
            &input,
            &receive,
            &mut pending_command,
            &mut serial,
            &mut network,
            &abort,
        );
        assert!(prepare_park(&machine, &mut cx).is_none());
    }
}

#[test]
fn console_output_is_drained_before_parking_can_wait() {
    let mut machine = idle_machine();
    {
        let serial = &mut machine.bus.devices_mut().unwrap().serial;
        serial.write(4, 14).unwrap();
        serial.write(4, 3).unwrap();
        serial.write(6, b'Z').unwrap();
        serial.write(4, 5).unwrap();
        serial.write(4, 0x68).unwrap();
    }
    let mut state = Snapshot::default();
    let mut serial = SerialBridge::default();
    let mut frame_cache = FrameCache::default();
    drain_console(&mut machine, &mut state, &mut serial, &mut frame_cache);
    assert!(state.serial.contains('Z'));
}

#[cfg(feature = "parking-profile")]
struct ProfileHarness {
    use_profiled: bool,
    machine: Machine,
    input: Mutex<Input>,
    receive: mpsc::Receiver<Command>,
    send: mpsc::SyncSender<Command>,
    command: Option<Command>,
    serial: SerialBridge,
    network: Option<NatEngine>,
    profile: Option<ParkingProfile>,
    abort: AtomicBool,
}

#[cfg(feature = "parking-profile")]
impl ProfileHarness {
    fn new() -> Self {
        let (send, receive) = mpsc::sync_channel(8);
        Self {
            use_profiled: true,
            machine: idle_machine(),
            input: Mutex::new(Input::default()),
            receive,
            send,
            command: None,
            serial: SerialBridge::default(),
            network: None,
            profile: ParkingProfile::for_review(),
            abort: AtomicBool::new(false),
        }
    }

    fn prepare(&mut self) -> Option<ParkPlan> {
        let mut context = ParkContext {
            input: &self.input,
            receive: &self.receive,
            pending_command: &mut self.command,
            serial: &mut self.serial,
            pending_keys: false,
            network: &mut self.network,
            abort: &self.abort,
        };
        if self.use_profiled {
            prepare_park_profiled(&self.machine, &mut context, &mut self.profile)
        } else {
            prepare_park(&self.machine, &mut context)
        }
    }

    fn commit(
        &mut self,
        plan: ParkPlan,
        elapsed: Duration,
        mut wait: impl FnMut(Duration, &Mutex<Input>, &mpsc::SyncSender<Command>),
    ) -> Option<helios::machine::ParkedAdvance> {
        let mut context = ParkContext {
            input: &self.input,
            receive: &self.receive,
            pending_command: &mut self.command,
            serial: &mut self.serial,
            pending_keys: false,
            network: &mut self.network,
            abort: &self.abort,
        };
        if !self.use_profiled {
            return commit_park_with(
                &mut self.machine,
                plan,
                &mut context,
                |duration| wait(duration, &self.input, &self.send),
                || elapsed,
            );
        }
        commit_park_with_profiled(
            &mut self.machine,
            plan,
            &mut context,
            &mut self.profile,
            |duration| wait(duration, &self.input, &self.send),
            || elapsed,
        )
    }
}

#[cfg(feature = "parking-profile")]
#[test]
fn profiling_does_not_change_successful_parked_state() {
    let mut off = ProfileHarness::new();
    off.profile = None;
    off.use_profiled = false;
    let mut on = ProfileHarness::new();
    let before = off
        .machine
        .diagnostic_snapshot()
        .differences(&on.machine.diagnostic_snapshot());
    assert!(before.is_empty(), "{before:?}");
    let a = off.prepare().unwrap();
    let b = on.prepare().unwrap();
    let off_result = off
        .commit(a, Duration::from_millis(1), |_, _, _| {})
        .unwrap();
    let on_result = on
        .commit(b, Duration::from_millis(1), |_, _, _| {})
        .unwrap();
    assert_eq!(off_result, on_result);
    let after = off
        .machine
        .diagnostic_snapshot()
        .differences(&on.machine.diagnostic_snapshot());
    assert!(after.is_empty(), "{after:?}");
    let profile = on.profile.as_ref().unwrap();
    assert_eq!(
        (
            profile.token_calls,
            profile.prepared,
            profile.wait_calls,
            profile.validations,
            profile.committed,
        ),
        (1, 1, 1, 1, 1)
    );
    assert_eq!(profile.host_checks, 3);
    assert_eq!(profile.wait_requested_ns, 2_000_000);
    assert_eq!(profile.committed_virtual_ns, 1_000_000);
    assert_eq!(profile.oversleep_ns, 0);
    assert!(profile.token_ns > 0 && profile.validation_ns > 0);
}

#[cfg(feature = "parking-profile")]
#[test]
fn profiling_does_not_change_input_cancellation() {
    let mut off = ProfileHarness::new();
    off.profile = None;
    off.use_profiled = false;
    let mut on = ProfileHarness::new();
    for harness in [&mut off, &mut on] {
        let plan = harness.prepare().unwrap();
        let result = harness.commit(plan, Duration::from_millis(1), |_, input, _| {
            let mut keys = input.lock().unwrap();
            keys.serial.enable(true);
            keys.serial.push(0, b"x").unwrap();
        });
        assert!(result.is_none());
        assert_eq!(harness.machine.virtual_ns(), 0);
        assert_eq!(harness.input.lock().unwrap().serial.front(), Some(b'x'));
    }
    let differences = off
        .machine
        .diagnostic_snapshot()
        .differences(&on.machine.diagnostic_snapshot());
    assert!(differences.is_empty(), "{differences:?}");
    let profile = on.profile.as_ref().unwrap();
    assert_eq!(
        (
            profile.post_host_work,
            profile.wait_calls,
            profile.validations,
            profile.committed,
        ),
        (1, 1, 0, 0)
    );
}

#[cfg(feature = "parking-profile")]
#[test]
fn profiling_does_not_change_stale_token_rejection() {
    let mut off = ProfileHarness::new();
    off.profile = None;
    off.use_profiled = false;
    let mut on = ProfileHarness::new();
    for harness in [&mut off, &mut on] {
        let plan = harness.prepare().unwrap();
        harness.machine.cpu.write_reg(1, 0x1234);
        let result = harness.commit(plan, Duration::from_millis(1), |_, _, _| {});
        assert!(result.is_none());
        assert_eq!(harness.machine.virtual_ns(), 0);
    }
    let differences = off
        .machine
        .diagnostic_snapshot()
        .differences(&on.machine.diagnostic_snapshot());
    assert!(differences.is_empty(), "{differences:?}");
    let profile = on.profile.as_ref().unwrap();
    assert_eq!(
        (
            profile.post_validation,
            profile.wait_calls,
            profile.validations,
            profile.committed,
        ),
        (1, 1, 1, 0)
    );
}
