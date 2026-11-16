use crate::{
    disk_creation::Creation,
    overlay::{Field, Overlay, Tab},
    settings::{Settings, TargetKind},
};
use helios::media::{ImageFormat, create_disk};

fn select(o: &mut Overlay, field: Field) {
    o.selected = o.rows().iter().position(|r| r.field == field).unwrap();
}

#[test]
fn creation_form_cancel_and_invalid_capacity_leave_settings_untouched() {
    let live = Settings::default();
    let mut o = Overlay::new(&live);
    o.tab = Tab::Media;
    select(&mut o, Field::NewDisk);
    o.activate();
    assert_eq!(o.new_disk.as_ref().unwrap().target, 3);
    assert_eq!(o.new_disk.as_ref().unwrap().size_mib, 2048);
    select(&mut o, Field::NewDiskSize);
    for value in ["0", "-1", "1.5", "2097153", "18446744073709551615"] {
        o.edit = Some(value.into());
        assert!(o.finish_edit().is_err());
        assert_eq!(o.new_disk.as_ref().unwrap().size_mib, 2048);
    }
    o.edit = Some("256".into());
    o.finish_edit().unwrap();
    assert_eq!(o.new_disk.as_ref().unwrap().size_mib, 256);
    o.set_path(Field::NewDiskPath, "unused.chd".into());
    o.cancel_new_disk();
    assert_eq!(o.draft, live);
    assert!(!o.dirty(&live));
}

#[test]
fn creation_uses_only_disconnected_targets() {
    let mut live = Settings::default();
    live.scsi[3].kind = TargetKind::Disk;
    live.scsi[3].path = "valuable.raw".into();
    let mut o = Overlay::new(&live);
    o.tab = Tab::Media;
    select(&mut o, Field::NewDisk);
    o.activate();
    assert_eq!(o.new_disk.as_ref().unwrap().target, 0);
    select(&mut o, Field::NewDiskTarget);
    for _ in 0..14 {
        o.activate();
        assert_ne!(o.new_disk.as_ref().unwrap().target, 3);
    }
    o.cancel_new_disk();
    assert_eq!(o.draft, live);
    for target in &mut o.draft.scsi {
        target.kind = TargetKind::Disk;
        target.path = "occupied.chd".into();
    }
    select(&mut o, Field::NewDisk);
    o.activate();
    assert!(o.new_disk.is_none());
    assert!(o.notice.as_ref().unwrap().contains("Detach"));
}

#[test]
fn created_chd_is_kept_on_discard_and_saved_named_machine_reuses_its_path() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("new.chd");
    let config = directory.path().join("test.cfg");
    let live = Settings::default();
    let mut o = Overlay::new(&live);
    let (created, target) = Creation::start(path.clone(), 1, 3)
        .unwrap()
        .finish()
        .unwrap();
    o.disk_created(target, created.display().to_string());
    assert!(live.scsi[3].path.is_empty());
    assert!(o.dirty(&live));
    drop(o);
    assert!(path.is_file());
    let mut saved = live.clone();
    saved.scsi[3] = crate::settings::ScsiTarget {
        kind: TargetKind::Disk,
        path: path.display().to_string(),
    };
    saved.save_as_machine("CHD test").unwrap();
    saved.save(&config).unwrap();
    let reloaded = Settings::load(&config).unwrap();
    assert_eq!(reloaded.scsi[3], saved.scsi[3]);
    assert_eq!(reloaded.machines["CHD test"].scsi[3], saved.scsi[3]);
    assert!(!helios::media::companion_path(&path).exists());
}

#[test]
fn settings_reject_wrong_chd_kind_and_corrupt_child_without_rewriting_anything() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("new.chd");
    let config = directory.path().join("test.cfg");
    create_disk(&path, ImageFormat::Chd, 1024 * 1024).unwrap();
    let mut settings = Settings::default();
    settings.scsi[2] = crate::settings::ScsiTarget {
        kind: TargetKind::Disk,
        path: path.display().to_string(),
    };
    settings.save(&config).unwrap();
    let original = std::fs::read(&config).unwrap();
    settings.scsi[2].kind = TargetKind::Cdrom;
    assert!(settings.save(&config).unwrap_err().contains("SCSI 2"));
    assert_eq!(std::fs::read(&config).unwrap(), original);
    settings.scsi[2].kind = TargetKind::Disk;
    let child = helios::media::companion_path(&path);
    std::fs::write(&child, b"valuable bad child").unwrap();
    assert!(settings.save(&config).is_err());
    assert_eq!(std::fs::read(&child).unwrap(), b"valuable bad child");
    assert_eq!(std::fs::read(&config).unwrap(), original);
}
