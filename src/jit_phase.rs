//! Optional worker snapshots for the local Solaris measurement harness.
use crate::{emulator::Snapshot, settings::atomic_write};
use helios::{jit_phase::CacheCounts, machine::Machine};
use std::{
    fmt::Write,
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

pub(crate) struct Probe {
    request: PathBuf,
    reply: PathBuf,
    last: Option<u64>,
    next_poll: Instant,
}

impl Probe {
    pub(crate) fn new(dir: PathBuf) -> Self {
        Self {
            request: dir.join("request"),
            reply: dir.join("sample.toml"),
            last: None,
            next_poll: Instant::now(),
        }
    }

    // The harness owns a fresh directory per process and has one outstanding request.
    pub(crate) fn poll(&mut self, machine: &Machine, state: &Snapshot) {
        let now = Instant::now();
        if now < self.next_poll {
            return;
        }
        self.next_poll = now + Duration::from_millis(2);
        let Some(request) = fs::read_to_string(&self.request)
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
        else {
            return;
        };
        if self.last == Some(request) {
            return;
        }
        let mut reply = format!(
            "request = {request}\npid = {}\nworker_phase = {:?}\nworker_outcomes = {}\nvirtual_ns = {}\njit = {}\n",
            std::process::id(),
            state.phase.label(),
            state.steps,
            machine.virtual_ns(),
            machine.native_execution_enabled()
        );
        for (id, cpu) in std::iter::once((0, &machine.cpu))
            .chain(machine.secondary_cpu().map(|module| (2, &module.cpu)))
        {
            let sample = cpu.jit_phase();
            let c = sample.counts;
            writeln!(reply,"[cpu{id}]\nepoch = {}\ncalls_by_length = {:?}\ninterpreted = {}\nfallbacks = {}\ncontrol_fallbacks = {}\nmemory_fallbacks = {}\ntraps = {}\ninterrupts = {}\ncpu_errors = {}",sample.epoch,c.calls_by_length,c.interpreted,c.fallbacks,c.control_fallbacks,c.memory_fallbacks,c.traps,c.interrupts,c.cpu_errors).unwrap();
            for (
                name,
                CacheCounts {
                    hits,
                    misses,
                    compiled,
                    evicted_entries,
                    compile_ns,
                },
            ) in [("words", c.words), ("spans", c.spans)]
            {
                writeln!(reply,"[cpu{id}.{name}]\nhits = {hits}\nmisses = {misses}\ncompiled = {compiled}\nevicted_entries = {evicted_entries}\ncompile_ns = {compile_ns}").unwrap();
            }
        }
        match atomic_write(&self.reply, reply.as_bytes()) {
            Ok(()) => self.last = Some(request),
            Err(error) => {
                eprintln!("JIT phase sample {request} failed: {error}");
                self.next_poll = now + Duration::from_secs(1);
            }
        }
    }
}
