use super::*;
use std::{fs, thread};

#[test]
fn mode_and_alias_persist_per_machine_without_changing_nvram_identity() {
    let mut settings = Settings {
        model: Model::Sparcstation20,
        display: Display::Sx,
        ..Settings::default()
    };
    settings.save_as_machine("workstation").unwrap();
    let before = settings.clone();
    let identity = settings.nvram_path(Path::new("state"));
    settings.headless = true;
    settings.ram_mib = 512;
    settings.serial_pty_link = "serial port".into();
    settings.remember_machine();
    settings.validate().unwrap();
    assert!(!settings.hardware_eq(&before));
    assert_eq!(settings.nvram_path(Path::new("state")), identity);
    let copy: Settings = toml::from_str(&toml::to_string(&settings).unwrap()).unwrap();
    assert_eq!(copy, settings);
    assert_eq!(
        copy.resolve(Path::new("/tmp/test")).serial_pty_link,
        "/tmp/test/serial port"
    );
    let old: Settings = toml::from_str("[machines.old]\nmodel = 'sparcstation5'").unwrap();
    assert!(!old.headless && !old.machines["old"].headless);
    assert!(old.serial_pty_link.is_empty());
}
#[test]
fn tinker_gates_controls_and_headless_to_sx_adjusts_only_the_draft() {
    let settings = Settings {
        model: Model::Sparcstation20,
        display: Display::Sx,
        headless: true,
        ram_mib: 512,
        ..Settings::default()
    };
    let mut overlay = Overlay::new(&settings);
    overlay.tab = overlay::Tab::Advanced;
    assert!(!overlay.rows().iter().any(|r| r.field == Field::Headless));
    assert!(overlay.draft.headless);
    overlay.draft.tinker = true;
    overlay.selected = overlay
        .rows()
        .iter()
        .position(|r| r.field == Field::Headless)
        .unwrap();
    overlay.activate();
    assert!(!overlay.draft.headless);
    assert_eq!(overlay.draft.ram_mib, 448);
    assert!(overlay.notice.as_deref().unwrap().contains("512 to 448"));
    assert!(settings.headless);
    assert_eq!(settings.ram_mib, 512);
    overlay.selected = overlay
        .rows()
        .iter()
        .position(|r| r.field == Field::SerialPtyLink)
        .unwrap();
    overlay.activate();
    assert_eq!(overlay.edit, Some(String::new()));
}
#[test]
fn serial_view_is_a_tinker_gated_saved_preference_without_hardware_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("heliacal.cfg");
    let settings = Settings {
        headless: true,
        ..Settings::default()
    };
    let mut overlay = Overlay::new(&settings);
    overlay.tab = overlay::Tab::Advanced;
    assert!(
        !overlay
            .rows()
            .iter()
            .any(|row| row.field == Field::SerialConsole)
    );
    overlay.draft.tinker = true;
    overlay.selected = overlay
        .rows()
        .iter()
        .position(|row| row.field == Field::SerialConsole)
        .unwrap();
    overlay.activate();
    assert!(!overlay.draft.show_serial_console);
    assert!(settings.show_serial_console); // Unsaved draft leaves live presentation alone.
    assert!(settings.hardware_eq(&overlay.draft));
    overlay.draft.save(&config).unwrap();
    let saved = Settings::load(&config).unwrap();
    assert!(!saved.show_serial_console);
    assert!(saved.headless);
    assert!(toml::from_str::<Settings>("").unwrap().show_serial_console);
}

#[test]
fn headless_cli_validates_final_topology_independent_of_argument_order() {
    for args in [
        vec![
            "--headless",
            "--ram-mib",
            "512",
            "--machine",
            "ss20",
            "--display",
            "sx",
        ],
        vec![
            "--display",
            "sx",
            "--machine",
            "ss20",
            "--ram-mib",
            "512",
            "--headless",
        ],
    ] {
        let options = Options::parse_from(args.into_iter().map(str::to_string))
            .unwrap()
            .unwrap();
        let mut settings = Settings::default();
        options.apply_hardware(&mut settings).unwrap();
        assert!(settings.headless);
        assert_eq!(settings.ram_mib, 512);
    }
}
#[cfg(unix)]
#[test]
fn headless_worker_transports_raw_serial_and_retains_guest_after_alias_collision() {
    use std::io::{Read, Write};
    use std::os::unix::fs::OpenOptionsExt;
    let dir = tempfile::tempdir().unwrap();
    let prom = dir.path().join("prom");
    fs::write(&prom, helios::machine::serial_demo_image()).unwrap();
    let alias = dir.path().join("console");
    let settings = Settings {
        prom: prom.display().to_string(),
        headless: true,
        network: false,
        serial_pty_link: alias.display().to_string(),
        ..Settings::default()
    };
    let emulator = Emulator::new(dir.path().join("state")).unwrap();
    let wait = |predicate: &dyn Fn(&emulator::Snapshot) -> bool| {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let state = emulator.snapshot();
            if predicate(&state) {
                break state;
            }
            assert!(
                Instant::now() < deadline,
                "timeout {:?} {:?}",
                state.phase,
                state.error
            );
            thread::sleep(Duration::from_millis(5));
        }
    };
    emulator.boot(settings.clone()).unwrap();
    let running = wait(&|s| s.serial.contains("HELIOS"));
    assert!(running.headless);
    assert_eq!(running.serial_screen.contents(), "HELIOS");
    assert!(running.error.is_none());
    let target = fs::read_link(&alias).unwrap();
    let mut client = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(nix::libc::O_NONBLOCK)
        .open(&alias)
        .unwrap();
    emulator.pause().unwrap();
    let paused = wait(&|s| s.phase == Phase::Paused);
    client.write_all(b"Z").unwrap();
    thread::sleep(Duration::from_millis(40));
    assert_eq!(emulator.snapshot().steps, paused.steps);
    assert!(!emulator.snapshot().serial.ends_with("Z\n"));
    let collision = dir.path().join("foreign");
    fs::write(&collision, "keep").unwrap();
    emulator
        .boot(Settings {
            serial_pty_link: collision.display().to_string(),
            ..settings.clone()
        })
        .unwrap();
    let rejected = wait(&|s| s.error.is_some());
    assert_eq!(rejected.phase, Phase::Paused);
    assert_eq!(rejected.steps, paused.steps);
    assert_eq!(fs::read_link(&alias).unwrap(), target);
    emulator.pause().unwrap();
    let echoed = wait(&|s| s.serial.ends_with("Z\n"));
    assert_eq!(echoed.serial_screen.contents(), "HELIOSZ");
    let mut raw = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(2);
    while !raw.ends_with(b"Z\n") {
        let mut buf = [0; 256];
        match client.read(&mut buf) {
            Ok(n) => raw.extend(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("{e}"),
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(raw, b"HELIOSZ\n");
    emulator.boot(settings.clone()).unwrap();
    let reset = wait(&|s| s.serial == "HELIOS" && s.error.is_none());
    assert_eq!(reset.serial_screen.contents(), "HELIOS");
    assert!(reset.serial_frame > echoed.serial_frame);
    assert_eq!(fs::read_link(&alias).unwrap(), target); // F5 keeps attached minicom alive.
    emulator
        .serial_input(reset.serial_input_generation, b"G")
        .unwrap();
    let gui_echoed = wait(&|s| s.serial.ends_with("G\n"));
    assert_eq!(gui_echoed.serial_screen.contents(), "HELIOSG");
    raw.clear();
    let deadline = Instant::now() + Duration::from_secs(2);
    while !raw.ends_with(b"G\n") {
        let mut buf = [0; 256];
        match client.read(&mut buf) {
            Ok(n) => raw.extend(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("{e}"),
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(raw, b"HELIOSG\n"); // Embedded input is echoed to the external PTY too.
    emulator.boot(settings).unwrap();
    let new = wait(&|s| s.serial == "HELIOS");
    assert_ne!(new.serial_input_generation, reset.serial_input_generation);
    assert!(
        emulator
            .serial_input(reset.serial_input_generation, b"stale")
            .is_err()
    );
    emulator.shutdown().unwrap();
    wait(&|s| s.phase == Phase::Stopped);
    assert!(fs::symlink_metadata(&alias).is_err());
    assert_eq!(fs::read_to_string(collision).unwrap(), "keep");
}
