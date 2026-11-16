use super::*;
use std::{collections::BTreeSet, fs, thread};

fn oracle(model: Model, display: Display) -> Vec<u32> {
    let (slots, modules): (usize, &[u32]) = match model {
        Model::Sparcstation5 => (8, &[0, 8, 32]),
        Model::Sparcstation20 if display == Display::Sx => (7, &[0, 16, 32, 64]),
        Model::Axil320 => (8, &[0, 16, 64]),
        _ => (8, &[0, 16, 32, 64]),
    };
    let mut totals = BTreeSet::from([0]);
    for _ in 0..slots {
        totals = totals
            .iter()
            .flat_map(|a| modules.iter().map(move |b| a + b))
            .collect();
    }
    totals.into_iter().filter(|&v| v != 0).collect()
}

fn field(overlay: &mut Overlay, field: Field) {
    overlay.selected = overlay
        .rows()
        .iter()
        .position(|r| r.field == field)
        .unwrap();
}

const CONFIGS: [(Model, Display); 4] = [
    (Model::Sparcstation5, Display::Cgsix),
    (Model::Sparcstation20, Display::Sx),
    (Model::Sparcstation20, Display::Cgsix),
    (Model::Axil320, Display::Cgsix),
];

#[test]
fn every_f9_choice_matches_independent_socket_enumeration_and_wraps() {
    for (model, display) in CONFIGS {
        let choices = oracle(model, display);
        let live = Settings {
            model,
            display,
            ram_mib: choices[0],
            ..Settings::default()
        };
        let mut overlay = Overlay::new(&live);
        field(&mut overlay, Field::Ram);
        for &expected in &choices {
            assert_eq!(overlay.draft.ram_mib, expected);
            overlay.activate();
        }
        assert_eq!(overlay.draft.ram_mib, choices[0]);
        assert_eq!(live.ram_mib, choices[0]);
        assert!(!overlay.dirty(&live));
    }
}

#[test]
fn every_model_display_transition_keeps_or_rounds_down_only_the_draft() {
    for (model, display) in CONFIGS {
        for old in oracle(model, display) {
            for action in [Field::Model, Field::Display] {
                let live = Settings {
                    model,
                    display,
                    ram_mib: old,
                    ..Settings::default()
                };
                let mut overlay = Overlay::new(&live);
                field(&mut overlay, action);
                overlay.activate();
                let choices = oracle(overlay.draft.model, overlay.draft.display);
                let expected = choices
                    .iter()
                    .copied()
                    .filter(|&v| v <= old)
                    .max()
                    .unwrap_or(choices[0]);
                assert_eq!(
                    overlay.draft.ram_mib, expected,
                    "{model:?}/{display:?}/{old}/{action:?}"
                );
                assert_eq!(live.ram_mib, old);
                assert_eq!(overlay.notice.is_some(), old != expected);
                if old != expected {
                    assert!(
                        overlay
                            .notice
                            .as_ref()
                            .unwrap()
                            .contains(&format!("{old} to {expected}"))
                    );
                }
                overlay.draft.validate_hardware().unwrap();
            }
        }
    }
}

fn app(settings: Settings, config: PathBuf) -> App {
    let nvram = settings::companion_path(&config, ".nvram");
    App {
        settings,
        config,
        emulator: Emulator::new(nvram).unwrap(),
        overlay: None,
        confirm: None,
        picker: None,
        creation: None,
        held: input::HeldKeys::default(),
        captured: false,
        quitting: false,
        notice: None,
    }
}

fn wait(emu: &Emulator, predicate: impl Fn(&emulator::Snapshot) -> bool) -> emulator::Snapshot {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let state = emu.snapshot();
        if predicate(&state) {
            return state;
        }
        assert!(
            Instant::now() < deadline,
            "phase={:?} error={:?} pc={:08x} description={:?}",
            state.phase,
            state.error,
            state.pc,
            state.machine_description
        );
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn ram_draft_cancel_discard_and_failed_save_preserve_catalog_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("heliacal.cfg");
    let mut settings = Settings::default();
    settings.save_as_machine("Review RAM").unwrap();
    settings.save(&config).unwrap();
    let original = fs::read(&config).unwrap();
    let mut app = app(settings.clone(), config.clone());
    app.overlay = Some(Overlay::new(&settings));
    app.overlay.as_mut().unwrap().draft.ram_mib = 256;
    app.close_overlay();
    assert!(matches!(app.confirm, Some((Confirm::Unsaved, 2))));
    app.confirm(); // Default Cancel keeps the draft, without saving it.
    assert_eq!(app.overlay.as_ref().unwrap().draft.ram_mib, 256);
    assert_eq!(app.settings, settings);
    assert_eq!(fs::read(&config).unwrap(), original);
    app.close_overlay();
    app.confirm = Some((Confirm::Unsaved, 1));
    app.confirm();
    assert!(app.overlay.is_none());
    assert_eq!(app.settings, settings);
    assert_eq!(fs::read(&config).unwrap(), original);
    app.overlay = Some(Overlay::new(&settings));
    app.overlay.as_mut().unwrap().draft.ram_mib = 240;
    app.save();
    assert!(app.overlay.is_some());
    assert_eq!(app.settings, settings);
    assert_eq!(fs::read(&config).unwrap(), original);
    app.emulator.shutdown().unwrap();
}

#[test]
fn confirmed_replacement_changes_actual_ram_and_failed_build_keeps_old_guest() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("heliacal.cfg");
    let prom = dir.path().join("prom");
    let gx = dir.path().join("gx");
    // Write/read physical 192 MiB, an empty bank at 128 MiB installed.
    // A real 256 MiB machine reads back 'A' and takes the success loop.
    let words: [u32; 11] = [
        0x0303_0000, // sethi %hi(0x0c000000), %g1
        0x8410_2041, // or %g0, 65, %g2
        0xc420_4000, // st %g2, [%g1]
        0xc600_4000, // ld [%g1], %g3
        0x80a0_c002, // subcc %g3, %g2, %g0
        0x0280_0004, // be success (word 9)
        0x0100_0000, // nop
        0x1080_0000, // missing-bank loop
        0x0100_0000,
        0x1080_0000, // installed-bank loop
        0x0100_0000,
    ];
    fs::write(
        &prom,
        words
            .into_iter()
            .flat_map(u32::to_be_bytes)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    fs::write(&gx, vec![0; 32768]).unwrap();
    let mut settings = Settings {
        prom: prom.display().to_string(),
        cgsix_rom: gx.display().to_string(),
        network: false,
        ..Settings::default()
    };
    settings.save_as_machine("Review RAM").unwrap();
    settings.save(&config).unwrap();
    let nvram_path = settings.nvram_path(&settings::companion_path(&config, ".nvram"));
    let mut nvram = settings.model.default_nvram().image();
    nvram[0x100] = 0x5a;
    fs::write(&nvram_path, nvram).unwrap();
    let mut app = app(settings.clone(), config.clone());
    app.emulator.boot(settings).unwrap();
    wait(&app.emulator, |s| s.phase == Phase::Running && s.steps > 10);
    app.emulator.pause().unwrap();
    let initial = wait(&app.emulator, |s| s.phase == Phase::Paused);
    assert!(matches!(initial.pc & 0xff, 28 | 32), "{:08x}", initial.pc);
    app.overlay = Some(Overlay::new(&app.settings));
    app.overlay.as_mut().unwrap().draft.ram_mib = 256;
    app.save();
    assert!(app.overlay.is_none());
    assert_eq!(Settings::load(&config).unwrap().ram_mib, 256);
    assert!(app.notice.as_ref().unwrap().0.contains("F5"));
    assert_eq!(app.emulator.snapshot().steps, initial.steps);
    assert!(
        app.emulator
            .snapshot()
            .machine_description
            .unwrap()
            .contains("128 MiB")
    );
    assert_eq!(
        app.settings
            .nvram_path(&settings::companion_path(&config, ".nvram")),
        nvram_path
    );
    app.ask(Confirm::Reset);
    app.confirm(); // Default cancellation.
    assert_eq!(app.emulator.snapshot().steps, initial.steps);
    assert_eq!(app.emulator.snapshot().phase, Phase::Paused);
    app.confirm = Some((Confirm::Reset, 0));
    app.confirm();
    wait(&app.emulator, |s| {
        s.phase == Phase::Running
            && s.steps > 10
            && s.machine_description
                .as_ref()
                .is_some_and(|d| d.contains("256 MiB"))
    });
    app.emulator.pause().unwrap();
    let replacement = wait(&app.emulator, |s| s.phase == Phase::Paused);
    assert!(
        matches!(replacement.pc & 0xff, 36 | 40),
        "{:08x}",
        replacement.pc
    );
    app.overlay = Some(Overlay::new(&app.settings));
    app.overlay.as_mut().unwrap().draft.ram_mib = 128;
    app.save();
    assert_eq!(Settings::load(&config).unwrap().ram_mib, 128);
    // Media disappeared after Save, before F5: construction must be atomic.
    fs::remove_file(&prom).unwrap();
    app.confirm = Some((Confirm::Reset, 0));
    app.confirm();
    let failed = wait(&app.emulator, |s| s.error.is_some());
    assert_eq!(failed.phase, Phase::Paused);
    assert_eq!(failed.steps, replacement.steps);
    assert_eq!(failed.pc, replacement.pc);
    assert_eq!(failed.machine_description, replacement.machine_description);
    app.emulator.shutdown().unwrap();
    wait(&app.emulator, |s| s.phase == Phase::Stopped);
    assert_eq!(fs::read(&nvram_path).unwrap()[0x100], 0x5a);
}

#[test]
fn named_cli_override_order_is_final_model_and_display_sensitive() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("heliacal.cfg");
    fs::write(&config, "machine_label='old'\n[machines.old]\n[machines.big]\nmodel='sparcstation20'\ndisplay='sx'\nram_mib=448\n").unwrap();
    for args in [
        [
            "--ram-mib",
            "512",
            "--machine-label",
            "big",
            "--display",
            "cgsix",
        ],
        [
            "--machine-label",
            "big",
            "--display",
            "cgsix",
            "--ram-mib",
            "512",
        ],
    ] {
        let options = Options::parse_from(args.into_iter().map(str::to_owned))
            .unwrap()
            .unwrap();
        let mut selected = Settings::load(&config).unwrap();
        selected
            .select_machine(options.machine_label.as_ref().unwrap())
            .unwrap();
        options.apply_hardware(&mut selected).unwrap();
        assert_eq!(selected.machine_label.as_deref(), Some("big"));
        assert_eq!(selected.ram_mib, 512);
        assert_eq!(selected.display, Display::Cgsix);
        assert_eq!(selected.machines["old"].ram_mib, 128);
        assert_eq!(selected.machines["big"].ram_mib, 448);
        assert_eq!(Settings::load(&config).unwrap().ram_mib, 128);
    }
}

#[test]
fn invalid_cli_or_inactive_record_stops_before_sdl_worker_and_media() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("heliacal.cfg");
    fs::write(&config, "").unwrap();
    for (model, display, ram) in [
        ("ss5", "cgsix", "240"),
        ("ss20", "sx", "512"),
        ("ss20", "cgsix", "496"),
        ("axil320", "cgsix", "496"),
        ("ss5", "cgsix", "0"),
        ("ss5", "cgsix", "4294967295"),
    ] {
        let options = Options::parse_from(
            [
                "--config",
                config.to_str().unwrap(),
                "--machine",
                model,
                "--display",
                display,
                "--ram-mib",
                ram,
                "--prom",
                "/missing/firmware",
                "--disk",
                "/missing/disk",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap()
        .unwrap();
        let error = run(options).unwrap_err().to_string();
        assert!(
            !error.contains("SDL") && !error.contains("/missing/"),
            "{error}"
        );
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
    fs::write(&config, "machine_label='active'\n[machines.active]\n[machines.inactive]\nmodel='sparcstation20'\ndisplay='sx'\nram_mib=512\n").unwrap();
    let error = Settings::load(&config).unwrap_err();
    assert!(error.contains("inactive"));
}
