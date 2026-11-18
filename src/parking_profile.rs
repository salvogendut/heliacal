use std::{
    fmt::Write as _,
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

const PROFILE_ENV: &str = "HELIACAL_PARKING_PROFILE";
const RESET_ENV: &str = "HELIACAL_PARKING_PROFILE_RESET";
const DUMP_ENV: &str = "HELIACAL_PARKING_PROFILE_DUMP";
const TOGGLE_ENV: &str = "HELIACAL_PARKING_PROFILE_TOGGLE";

pub struct ParkingProfile {
    path: PathBuf,
    reset_path: Option<PathBuf>,
    dump_path: Option<PathBuf>,
    toggle_path: Option<PathBuf>,
    active: bool,
    started: Instant,
    reset_polled: Instant,
    pub outer_loops: u64,
    pub instructions: u64,
    pub candidates: u64,
    pub prepared: u64,
    pub committed: u64,
    pub prep_break: u64,
    pub prep_host_work: u64,
    pub prep_deadline: u64,
    pub prep_token: u64,
    pub post_break: u64,
    pub post_host_work: u64,
    pub post_network: u64,
    pub post_zero_elapsed: u64,
    pub post_validation: u64,
    pub host_checks: u64,
    pub token_calls: u64,
    pub validations: u64,
    pub wait_calls: u64,
    pub wait_requested_ns: u64,
    pub cpu_exec_ns: u64,
    pub host_check_ns: u64,
    pub token_ns: u64,
    pub wait_ns: u64,
    pub validation_ns: u64,
    pub serial_ns: u64,
    pub input_ns: u64,
    pub network_ns: u64,
    pub output_ns: u64,
    pub publish_ns: u64,
    pub control_ns: u64,
    pub committed_virtual_ns: u64,
    pub oversleep_ns: u64,
}

impl ParkingProfile {
    fn new(
        path: PathBuf,
        reset_path: Option<PathBuf>,
        dump_path: Option<PathBuf>,
        toggle_path: Option<PathBuf>,
    ) -> Self {
        helios::parking_profile::reset();
        let now = Instant::now();
        Self {
            path,
            reset_path,
            dump_path,
            toggle_path,
            active: true,
            started: now,
            reset_polled: now,
            outer_loops: 0,
            instructions: 0,
            candidates: 0,
            prepared: 0,
            committed: 0,
            prep_break: 0,
            prep_host_work: 0,
            prep_deadline: 0,
            prep_token: 0,
            post_break: 0,
            post_host_work: 0,
            post_network: 0,
            post_zero_elapsed: 0,
            post_validation: 0,
            host_checks: 0,
            token_calls: 0,
            validations: 0,
            wait_calls: 0,
            wait_requested_ns: 0,
            cpu_exec_ns: 0,
            host_check_ns: 0,
            token_ns: 0,
            wait_ns: 0,
            validation_ns: 0,
            serial_ns: 0,
            input_ns: 0,
            network_ns: 0,
            output_ns: 0,
            publish_ns: 0,
            control_ns: 0,
            committed_virtual_ns: 0,
            oversleep_ns: 0,
        }
    }

    pub fn from_env() -> Option<Self> {
        let path = std::env::var_os(PROFILE_ENV).map(PathBuf::from)?;
        Some(Self::new(
            path,
            std::env::var_os(RESET_ENV).map(PathBuf::from),
            std::env::var_os(DUMP_ENV).map(PathBuf::from),
            std::env::var_os(TOGGLE_ENV).map(PathBuf::from),
        ))
    }

    #[cfg(test)]
    pub(crate) fn for_review() -> Option<Self> {
        Some(Self::new(PathBuf::from("/dev/null"), None, None, None))
    }

    pub fn maybe_reset(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.reset_polled) < Duration::from_millis(100) {
            return;
        }
        self.reset_polled = now;
        if let Some(path) = self.toggle_path.as_deref()
            && fs::remove_file(path).is_ok()
        {
            self.active = !self.active;
        }
        if let Some(path) = self.dump_path.as_deref()
            && fs::remove_file(path).is_ok()
        {
            self.write();
        }
        let Some(path) = self.reset_path.as_deref() else {
            return;
        };
        if fs::remove_file(path).is_ok() {
            helios::parking_profile::reset();
            self.started = now;
            self.outer_loops = 0;
            self.instructions = 0;
            self.candidates = 0;
            self.prepared = 0;
            self.committed = 0;
            self.prep_break = 0;
            self.prep_host_work = 0;
            self.prep_deadline = 0;
            self.prep_token = 0;
            self.post_break = 0;
            self.post_host_work = 0;
            self.post_network = 0;
            self.post_zero_elapsed = 0;
            self.post_validation = 0;
            self.host_checks = 0;
            self.token_calls = 0;
            self.validations = 0;
            self.wait_calls = 0;
            self.wait_requested_ns = 0;
            self.cpu_exec_ns = 0;
            self.host_check_ns = 0;
            self.token_ns = 0;
            self.wait_ns = 0;
            self.validation_ns = 0;
            self.serial_ns = 0;
            self.input_ns = 0;
            self.network_ns = 0;
            self.output_ns = 0;
            self.publish_ns = 0;
            self.control_ns = 0;
            self.committed_virtual_ns = 0;
            self.oversleep_ns = 0;
        }
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn timer(&self) -> Option<Instant> {
        self.active.then(Instant::now)
    }

    pub fn write(&self) {
        let period_ns = nanos(self.started.elapsed());
        let accounted_ns = self
            .cpu_exec_ns
            .saturating_add(self.host_check_ns)
            .saturating_add(self.token_ns)
            .saturating_add(self.wait_ns)
            .saturating_add(self.validation_ns)
            .saturating_add(self.serial_ns)
            .saturating_add(self.input_ns)
            .saturating_add(self.network_ns)
            .saturating_add(self.output_ns)
            .saturating_add(self.publish_ns)
            .saturating_add(self.control_ns);
        let mut text = String::new();
        let _ = writeln!(text, "{{");
        let _ = writeln!(
            text,
            "  \"elapsed_clock\": \"worker Instant since last reset; not CPU time\","
        );
        let _ = writeln!(text, "  \"elapsed_ns\": {period_ns},");
        let _ = writeln!(text, "  \"accounted_elapsed_ns\": {accounted_ns},");
        let _ = writeln!(
            text,
            "  \"unaccounted_elapsed_ns\": {},",
            period_ns.saturating_sub(accounted_ns)
        );
        let _ = writeln!(text, "  \"counters\": {{");
        let _ = writeln!(text, "    \"outer_loops\": {},", self.outer_loops);
        let _ = writeln!(text, "    \"instructions\": {},", self.instructions);
        let _ = writeln!(text, "    \"candidates\": {},", self.candidates);
        let _ = writeln!(text, "    \"prepared\": {},", self.prepared);
        let _ = writeln!(text, "    \"committed\": {},", self.committed);
        let _ = writeln!(text, "    \"prep_break\": {},", self.prep_break);
        let _ = writeln!(text, "    \"prep_host_work\": {},", self.prep_host_work);
        let _ = writeln!(text, "    \"prep_deadline\": {},", self.prep_deadline);
        let _ = writeln!(text, "    \"prep_token\": {},", self.prep_token);
        let _ = writeln!(text, "    \"post_break\": {},", self.post_break);
        let _ = writeln!(text, "    \"post_host_work\": {},", self.post_host_work);
        let _ = writeln!(text, "    \"post_network\": {},", self.post_network);
        let _ = writeln!(
            text,
            "    \"post_zero_elapsed\": {},",
            self.post_zero_elapsed
        );
        let _ = writeln!(text, "    \"post_validation\": {},", self.post_validation);
        let _ = writeln!(text, "    \"host_checks\": {},", self.host_checks);
        let _ = writeln!(text, "    \"token_calls\": {},", self.token_calls);
        let _ = writeln!(text, "    \"validations\": {},", self.validations);
        let _ = writeln!(text, "    \"wait_calls\": {},", self.wait_calls);
        let _ = writeln!(
            text,
            "    \"wait_requested_ns\": {},",
            self.wait_requested_ns
        );
        let _ = writeln!(
            text,
            "    \"committed_virtual_ns\": {},",
            self.committed_virtual_ns
        );
        let _ = writeln!(text, "    \"oversleep_ns\": {}", self.oversleep_ns);
        let _ = writeln!(text, "  }},");
        let _ = writeln!(text, "  \"host_ns\": {{");
        let _ = writeln!(text, "    \"normal_execution\": {},", self.cpu_exec_ns);
        let _ = writeln!(text, "    \"host_checks\": {},", self.host_check_ns);
        let _ = writeln!(text, "    \"token_construction\": {},", self.token_ns);
        let _ = writeln!(text, "    \"commit_validation\": {},", self.validation_ns);
        let _ = writeln!(text, "    \"waiting\": {},", self.wait_ns);
        let _ = writeln!(text, "    \"serial_service\": {},", self.serial_ns);
        let _ = writeln!(text, "    \"input\": {},", self.input_ns);
        let _ = writeln!(text, "    \"network\": {},", self.network_ns);
        let _ = writeln!(text, "    \"console_output\": {},", self.output_ns);
        let _ = writeln!(text, "    \"publish\": {},", self.publish_ns);
        let _ = writeln!(text, "    \"control\": {}", self.control_ns);
        let _ = writeln!(text, "  }},");
        let core = helios::parking_profile::snapshot();
        let _ = writeln!(text, "  \"core\": {{");
        let _ = writeln!(text, "    \"token_success\": {},", core.token_success);
        let _ = writeln!(text, "    \"token_rejected\": {},", core.token_rejected);
        let _ = writeln!(text, "    \"token_success_ns\": {},", core.token_success_ns);
        let _ = writeln!(
            text,
            "    \"token_rejected_ns\": {},",
            core.token_rejected_ns
        );
        let _ = writeln!(text, "    \"token_ns\": {},", core.token_ns);
        let _ = writeln!(text, "    \"validate_success\": {},", core.validate_success);
        let _ = writeln!(
            text,
            "    \"validate_rejected\": {},",
            core.validate_rejected
        );
        let _ = writeln!(
            text,
            "    \"validate_success_ns\": {},",
            core.validate_success_ns
        );
        let _ = writeln!(
            text,
            "    \"validate_rejected_ns\": {},",
            core.validate_rejected_ns
        );
        let _ = writeln!(text, "    \"validate_ns\": {},", core.validate_ns);
        let _ = writeln!(text, "    \"advance_success\": {},", core.advance_success);
        let _ = writeln!(text, "    \"advance_rejected\": {},", core.advance_rejected);
        let _ = writeln!(
            text,
            "    \"advance_success_ns\": {},",
            core.advance_success_ns
        );
        let _ = writeln!(
            text,
            "    \"advance_rejected_ns\": {},",
            core.advance_rejected_ns
        );
        let _ = writeln!(text, "    \"advance_ns\": {},", core.advance_ns);
        let _ = writeln!(text, "    \"proof_ok\": {},", core.proof_ok);
        let _ = writeln!(text, "    \"proof_failed\": {},", core.proof_failed);
        let _ = writeln!(text, "    \"proof_ns\": {},", core.proof_ns);
        let _ = writeln!(text, "    \"readiness_ok\": {},", core.readiness_ok);
        let _ = writeln!(text, "    \"readiness_failed\": {},", core.readiness_failed);
        let _ = writeln!(text, "    \"readiness_ns\": {},", core.readiness_ns);
        let _ = writeln!(text, "    \"digest_calls\": {},", core.digest_calls);
        let _ = writeln!(text, "    \"digest_ns\": {},", core.digest_ns);
        let _ = writeln!(
            text,
            "    \"arbitration_calls\": {},",
            core.arbitration_calls
        );
        let _ = writeln!(text, "    \"arbitration_ns\": {},", core.arbitration_ns);
        let _ = writeln!(text, "    \"snapshot_calls\": {},", core.snapshot_calls);
        let _ = writeln!(text, "    \"snapshot_ns\": {},", core.snapshot_ns);
        let _ = writeln!(text, "    \"snapshot_eq_ok\": {},", core.snapshot_eq_ok);
        let _ = writeln!(
            text,
            "    \"snapshot_eq_failed\": {},",
            core.snapshot_eq_failed
        );
        let _ = writeln!(text, "    \"snapshot_eq_ns\": {},", core.snapshot_eq_ns);
        let _ = writeln!(text, "    \"deadline_calls\": {},", core.deadline_calls);
        let _ = writeln!(text, "    \"deadline_ns\": {},", core.deadline_ns);
        let _ = writeln!(
            text,
            "    \"advancement_calls\": {},",
            core.advancement_calls
        );
        let _ = writeln!(text, "    \"advancement_ns\": {}", core.advancement_ns);
        let _ = writeln!(text, "  }}");
        let _ = writeln!(text, "}}");
        if let Err(error) = fs::write(&self.path, text) {
            eprintln!("parking profile write {}: {error}", self.path.display());
        }
    }
}

pub fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}
