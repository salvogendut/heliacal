use super::tests::{dual_cpu_test_settings, wait};
use super::*;

struct Fixture(PathBuf);

impl Fixture {
    fn sample(&self, id: i64) -> toml::Table {
        atomic_write(&self.0.join("request"), id.to_string().as_bytes()).unwrap();
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            if let Ok(raw) = fs::read_to_string(self.0.join("sample.toml")) {
                let value: toml::Table = toml::from_str(&raw).unwrap();
                if value["request"].as_integer() == Some(id) {
                    return value;
                }
            }
            assert!(Instant::now() < until, "no worker reply for request {id}");
            thread::sleep(Duration::from_millis(2));
        }
    }
    fn write_prom(&self, words: &[u32]) -> Settings {
        let path = self.0.join("prom");
        fs::write(
            &path,
            words
                .iter()
                .copied()
                .flat_map(u32::to_be_bytes)
                .collect::<Vec<_>>(),
        )
        .unwrap();
        Settings {
            model: Model::Sparcstation20,
            display: Display::Sx,
            prom: path.display().to_string(),
            execution_jit: true,
            network: false,
            rtc_host_time: false,
            ..Settings::default()
        }
    }
    fn emulator(&self) -> Emulator {
        Emulator::new(self.0.join("state.nvram")).unwrap()
    }
}

// Each subprocess gives its own worker the private sampling directory. Other
// concurrent tests continue to run with their normal environment.
fn isolated(name: &str, body: impl FnOnce(Fixture)) {
    if let Some(dir) = std::env::var_os("HELIACAL_TEST_PHASE_DIR") {
        body(Fixture(dir.into()));
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            &format!("emulator::jit_worker_tests::{name}"),
            "--nocapture",
        ])
        .env("HELIACAL_TEST_PHASE_DIR", dir.path())
        .env("HELIACAL_JIT_PHASE_DIR", dir.path())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(dir.path().join("sample.toml").exists());
}

const ADD: u32 = 0x8200_6001;
const NOP: u32 = 0x0100_0000;
const POWEROFF: [u32; 3] = [
    0x0300_0000 | (0xf1a0_1000u32 >> 10),
    0x8410_2001,
    0xc4a8_45e0,
];

fn constant(code: &mut Vec<u32>, rd: u32, value: u32) {
    code.push(0x0100_0000 | rd << 25 | value >> 10);
    code.push(0x8010_2000 | rd << 25 | rd << 14 | (value & 0x3ff));
}
fn store(code: &mut Vec<u32>, addr: u32, data: u32, asi: u32) {
    constant(code, 1, addr);
    constant(code, 2, data);
    code.push(0xc4a0_4000 | asi << 5); // sta %g2,[%g1] asi
}

// Map VA 0 to writable RAM at 0x8000 and turn on translation on the
// last instruction before 0x400. The 15-word loop cuts across batch ends.
fn loop_prom(observe_break: bool) -> Vec<u32> {
    let mut code = vec![];
    for (addr, data) in [
        (0x1000, 0x201),
        (0x2000, 0x301),
        (0x3000, 0x401),
        (0x4000, 0x88e),
    ] {
        store(&mut code, addr, data, 0x20);
    }
    let mut payload = vec![];
    if observe_break {
        constant(&mut code, 3, 0xf1100004); // TTYA control at PA f:f1100004
        // Enable RX and disable the WR15 external-status latch so RR0
        // reports both live edges without an interrupt acknowledgement.
        for byte in [3, 0xc1, 15, 0] {
            constant(&mut code, 2, byte);
            code.push(0xc4a8_c5e0); // stba %g2,[%g3] ASI 2f, enable receiver
        }
        // Wait until the guest sees BREAK asserted, then until it sees release.
        for branch in [0x02bf_fff1, 0x12bf_fff1] {
            payload.extend([ADD; 13]);
            payload.extend([0xc888_c5e0, 0x8089_2080, branch, NOP]); // lduba; andcc RR0,0x80; be/bne -15
        }
        payload.extend(POWEROFF);
    } else {
        payload.extend([ADD; 13]);
        payload.extend([0x10bf_fff3, NOP]); // ba back 13, delay slot
    }
    for (i, word) in payload.into_iter().enumerate() {
        store(&mut code, 0x8400 + 4 * i as u32, word, 0x20);
    }
    store(&mut code, 0x100, 0x100, 4);
    constant(&mut code, 1, 0);
    constant(&mut code, 2, 1);
    assert!(code.len() < 255);
    code.resize(255, NOP);
    code.push(0xc4a0_4080); // sta %g2,[%g1] ASI 4, enable MMU
    code
}

fn histogram(sample: &toml::Table, id: &str) -> Vec<i64> {
    sample[id]["calls_by_length"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_integer().unwrap())
        .collect()
}
fn outcomes(sample: &toml::Table) -> i64 {
    sample["worker_outcomes"].as_integer().unwrap()
}

#[test]
fn fault_and_poweroff_preserve_progress() {
    isolated("fault_and_poweroff_preserve_progress", |f| {
        let mut words = vec![NOP; 2049];
        words.push(0);
        let emu = f.emulator();
        emu.boot(f.write_prom(&words)).unwrap();
        wait(&emu, |s| s.phase == Phase::Failed);
        let error = f.sample(1);
        assert_eq!(outcomes(&error), 2049);
        assert_eq!(error["cpu0"]["interpreted"].as_integer(), Some(2049));
        assert_eq!(error["cpu0"]["cpu_errors"].as_integer(), Some(1));
        let mut words = vec![NOP; 2047];
        words.extend(POWEROFF);
        emu.boot(f.write_prom(&words)).unwrap();
        let stopped = wait(&emu, |s| s.phase == Phase::Stopped);
        assert!(stopped.serial.contains("System powered off."));
        let power = f.sample(2);
        assert_ne!(error["cpu0"]["epoch"], power["cpu0"]["epoch"]);
        assert_eq!(outcomes(&power), 2049);
        assert_eq!(power["cpu0"]["interpreted"].as_integer(), Some(2049));
        emu.shutdown().unwrap();
    });
}

#[test]
fn translated_spans_respect_budget_pause_and_keyboard_backpressure() {
    isolated(
        "translated_spans_respect_budget_pause_and_keyboard_backpressure",
        |f| {
            let emu = f.emulator();
            emu.boot(f.write_prom(&loop_prom(false))).unwrap();
            wait(&emu, |s| s.phase == Phase::Running && s.steps >= 2048);
            let run = f.sample(1);
            assert_eq!(outcomes(&run) % 2048, 0);
            emu.pause().unwrap();
            wait(&emu, |s| s.phase == Phase::Paused);
            let paused = f.sample(2);
            assert_eq!(outcomes(&paused) % 2048, 0);
            let hist = histogram(&paused, "cpu0");
            let native = hist
                .iter()
                .enumerate()
                .map(|(i, n)| (i as i64 + 1) * n)
                .sum::<i64>();
            assert!(hist[1..].iter().sum::<i64>() > 0);
            assert_eq!(
                outcomes(&paused),
                paused["cpu0"]["interpreted"].as_integer().unwrap() + native
            );
            assert_eq!(f.sample(3)["cpu0"], paused["cpu0"]);
            // The guest never enables the keyboard receiver. The 16-byte queue
            // fills while further keys remain pending at the worker.
            for key in 1..=64 {
                assert!(emu.key(key, true));
            }
            emu.pause().unwrap();
            wait(&emu, |s| {
                s.phase == Phase::Running && s.steps > outcomes(&paused) as u64
            });
            let blocked = f.sample(4);
            let h = histogram(&blocked, "cpu0");
            assert_eq!(&h[1..], &hist[1..]);
            assert!(h[0] > hist[0]);
            assert_eq!(outcomes(&blocked) % 2048, 0);
            emu.shutdown().unwrap();
        },
    );
}

#[test]
fn parking_and_smp_exclude_spans() {
    isolated("parking_and_smp_exclude_spans", |f| {
        let emu = f.emulator();
        let mut s = f.write_prom(&loop_prom(false));
        s.idle_parking = true;
        emu.boot(s).unwrap();
        wait(&emu, |s| s.phase == Phase::Running && s.steps >= 2048);
        let parked = f.sample(1);
        let h = histogram(&parked, "cpu0");
        assert!(h[0] > 0);
        assert!(h[1..].iter().all(|n| *n == 0));
        let mut dual = dual_cpu_test_settings(&f.0, Model::Sparcstation20);
        dual.execution_jit = true;
        emu.boot(dual).unwrap();
        wait(&emu, |s| s.processor_steps[1] > 0);
        let smp = f.sample(2);
        assert_eq!(smp["jit"].as_bool(), Some(false));
        for cpu in ["cpu0", "cpu2"] {
            assert_eq!(histogram(&smp, cpu).iter().sum::<i64>(), 0);
            assert!(smp[cpu]["interpreted"].as_integer().unwrap() > 0);
        }
        emu.shutdown().unwrap();
    });
}

#[test]
fn serial_break_reaches_the_guest_and_releases_after_a_million_outcomes() {
    isolated(
        "serial_break_reaches_the_guest_and_releases_after_a_million_outcomes",
        |f| {
            let emu = f.emulator();
            let mut s = f.write_prom(&loop_prom(true));
            s.headless = true;
            emu.boot(s).unwrap();
            wait(&emu, |s| s.phase == Phase::Running && s.steps > 2048);
            emu.pause().unwrap();
            wait(&emu, |s| s.phase == Phase::Paused);
            let before = f.sample(1);
            assert!(histogram(&before, "cpu0")[1..].iter().sum::<i64>() > 0);
            emu.serial_break().unwrap();
            emu.pause().unwrap();
            let stopped = wait(&emu, |s| s.phase == Phase::Stopped);
            assert!(stopped.serial.contains("System powered off."));
            let elapsed = outcomes(&f.sample(2)) - outcomes(&before);
            assert!(
                (1_000_000..=1_000_000 + 2 * 2048).contains(&elapsed),
                "{elapsed}"
            );
            emu.shutdown().unwrap();
        },
    );
}

#[test]
fn guest_reset_starts_a_new_epoch_mid_batch_and_restores_policy() {
    isolated(
        "guest_reset_starts_a_new_epoch_mid_batch_and_restores_policy",
        |f| {
            // The PROM reads the SS-20 reset cause on each entry. First pass:
            // request software reset at word 2048. Second pass: power off.
            let mut words = vec![
                0x0300_0000 | (0xf1f0_0000u32 >> 10), // sethi reset register address, %g1
                0xc480_45e0,                          // lda [%g1] ASI 2f,%g2
                0x8088_a002,                          // andcc %g2,2,%g0
                0x1280_0000 | (2050 - 3),             // bne poweroff
                NOP,
            ];
            words.resize(2047, NOP);
            words.extend([0x8410_2001, 0xc4a0_45e0, 0]); // or 1,%g2; sta; UNIMP
            words.extend(POWEROFF);
            let emu = f.emulator();
            emu.boot(f.write_prom(&words)).unwrap();
            let state = wait(&emu, |s| matches!(s.phase, Phase::Stopped | Phase::Failed));
            assert_eq!(state.phase, Phase::Stopped, "{:?}", state.error);
            let final_sample = f.sample(1);
            assert_eq!(outcomes(&final_sample), 2056);
            assert_eq!(final_sample["cpu0"]["interpreted"].as_integer(), Some(7));
            assert_eq!(histogram(&final_sample, "cpu0").iter().sum::<i64>(), 0);
            emu.shutdown().unwrap();
        },
    );
}
