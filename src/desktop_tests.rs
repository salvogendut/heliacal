use super::*;
use std::{fs, thread};

fn app(dir: &Path, power_off: bool) -> App {
    let prom = dir.join("test-prom.bin");
    let words: &[u32] = if power_off {
        // SS-20 AUXIO2: write 1 to PA 0xff1a01000 through ASI 0x2f.
        &[0x033c_6804, 0x8210_6000, 0x8410_2001, 0xc4a8_45e0]
    } else {
        &[0x3080_0000, 0x0100_0000] // BA,a self-loop.
    };
    fs::write(
        &prom,
        words
            .iter()
            .flat_map(|w| w.to_be_bytes())
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let config = dir.join("heliacal.cfg");
    App {
        settings: Settings {
            model: Model::Sparcstation20,
            headless: true,
            ram_mib: 16,
            prom: prom.display().to_string(),
            network: false,
            ..Settings::default()
        },
        emulator: Emulator::new(settings::companion_path(&config, ".nvram")).unwrap(),
        config,
        overlay: None,
        confirm: None,
        picker: None,
        picker_history: file_picker::History::default(),
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
    }
}

fn wait(emu: &Emulator, predicate: impl Fn(&emulator::Snapshot) -> bool) -> emulator::Snapshot {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let state = emu.snapshot();
        if predicate(&state) {
            return state;
        }
        assert!(
            Instant::now() < until,
            "timeout: {:?} {:?}",
            state.phase,
            state.error
        );
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn stopped_guest_skips_quit_confirmation_but_waits_for_persistence_and_cleanup() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path(), true);
    app.emulator.boot(app.settings.clone()).unwrap();
    let stopped = wait(&app.emulator, |s| s.phase == Phase::Stopped);
    assert!(stopped.has_machine && !stopped.shutdown_complete);
    assert!(stopped.serial_endpoint.is_some());
    let nvram = app
        .settings
        .nvram_path(&settings::companion_path(&app.config, ".nvram"));
    assert!(!nvram.exists());
    app.request_quit();
    assert!(app.quitting && app.confirm.is_none());
    let closed = wait(&app.emulator, |s| s.shutdown_complete);
    assert_eq!(closed.phase, Phase::Stopped);
    assert!(closed.error.is_none() && closed.serial_endpoint.is_none());
    assert_eq!(fs::read(nvram).unwrap().len(), 8192);
}

#[test]
fn faulted_guest_also_skips_quit_confirmation_and_saves_nvram() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path(), false);
    // UNIMP with traps disabled stops the core with an error.
    fs::write(&app.settings.prom, [0; 4]).unwrap();
    app.emulator.boot(app.settings.clone()).unwrap();
    let failed = wait(&app.emulator, |s| s.phase == Phase::Failed);
    assert!(failed.error.is_some() && !failed.shutdown_complete);
    app.request_quit();
    assert!(app.quitting && app.confirm.is_none());
    let closed = wait(&app.emulator, |s| s.shutdown_complete);
    assert!(closed.error.is_none() && closed.serial_endpoint.is_none());
    let nvram = app
        .settings
        .nvram_path(&settings::companion_path(&app.config, ".nvram"));
    assert_eq!(fs::read(nvram).unwrap().len(), 8192);
}

#[test]
fn stopped_guest_save_failure_remains_recoverable() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path(), true);
    app.emulator.boot(app.settings.clone()).unwrap();
    wait(&app.emulator, |s| s.phase == Phase::Stopped);
    let nvram = app
        .settings
        .nvram_path(&settings::companion_path(&app.config, ".nvram"));
    fs::create_dir(&nvram).unwrap();
    app.request_quit();
    let failed = wait(&app.emulator, |s| s.shutdown_failed);
    assert_eq!(failed.phase, Phase::Stopped);
    assert!(!failed.shutdown_complete && failed.has_machine);
    assert!(failed.serial_endpoint.is_some());
    fs::remove_dir(&nvram).unwrap();
    app.quitting = false; // The event loop re-enables Quit after a save failure.
    app.request_quit();
    assert!(app.confirm.is_none());
    assert!(wait(&app.emulator, |s| s.shutdown_complete).error.is_none());
    assert_eq!(fs::read(nvram).unwrap().len(), 8192);
}

#[test]
fn running_and_paused_guests_keep_quit_confirmation() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path(), false);
    app.emulator.boot(app.settings.clone()).unwrap();
    wait(&app.emulator, |s| s.phase == Phase::Running && s.steps > 0);
    app.request_quit();
    assert!(matches!(app.confirm, Some((Confirm::Quit, 1))));
    assert!(!app.quitting);
    app.confirm(); // Default Cancel.
    app.emulator.pause().unwrap();
    wait(&app.emulator, |s| s.phase == Phase::Paused);
    app.request_quit();
    assert!(matches!(app.confirm, Some((Confirm::Quit, 1))));
    assert!(!app.quitting);
    app.confirm = Some((Confirm::Quit, 0));
    app.confirm();
    assert!(app.quitting);
    wait(&app.emulator, |s| s.shutdown_complete);
}
