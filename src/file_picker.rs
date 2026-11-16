use crate::{
    overlay::{Field, Overlay},
    settings::{self, Settings, TargetKind},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Navigation preferences are independent of the machine settings draft.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct History {
    #[serde(skip_serializing_if = "Option::is_none")]
    firmware: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disk: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cdrom: Option<PathBuf>,
}

enum Kind {
    Firmware,
    Disk,
    Cdrom,
}

impl Kind {
    fn for_field(field: Field, settings: &Settings) -> Option<Self> {
        match field {
            Field::Prom | Field::Rom => Some(Self::Firmware),
            Field::NewDiskPath => Some(Self::Disk),
            Field::Scsi(id) => Some(match settings.scsi.get(id)?.kind {
                TargetKind::Cdrom => Self::Cdrom,
                TargetKind::Disk | TargetKind::Disconnected => Self::Disk,
            }),
            _ => None,
        }
    }
}

impl History {
    pub fn load(config: &Path) -> Result<Self, String> {
        let path = settings::companion_path(config, ".picker.toml");
        match fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }

    pub fn save(&self, config: &Path) -> Result<(), String> {
        settings::atomic_write(
            &settings::companion_path(config, ".picker.toml"),
            toml::to_string_pretty(self)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
    }

    pub fn remember(&mut self, field: Field, settings: &Settings, path: &Path, base: &Path) {
        let Some(kind) = Kind::for_field(field, settings) else {
            return;
        };
        let path = resolve(path, base);
        match kind {
            Kind::Firmware => self.firmware = Some(path),
            Kind::Disk => self.disk = Some(path),
            Kind::Cdrom => self.cdrom = Some(path),
        }
    }

    pub fn location(&self, overlay: &Overlay, field: Field, base: &Path) -> Option<PathBuf> {
        let remembered = match Kind::for_field(field, &overlay.draft)? {
            Kind::Firmware => self.firmware.as_deref(),
            Kind::Disk => self.disk.as_deref(),
            Kind::Cdrom => self.cdrom.as_deref(),
        };
        let current = overlay.path(field).filter(|path| !path.is_empty());
        let current = current.map(|path| resolve(Path::new(path), base));

        if field == Field::NewDiskPath {
            // Preserve an explicitly chosen destination, even before it exists.
            if let Some(path) = current.as_ref()
                && usable(path)
                && path.parent().is_some_and(Path::is_dir)
                && !path.is_dir()
            {
                return current;
            }
        }

        let location = current
            .as_deref()
            .and_then(existing_location)
            .or_else(|| remembered.and_then(|path| existing_location(&resolve(path, base))));
        if field == Field::NewDiskPath {
            let target = overlay.new_disk.as_ref()?.target;
            let directory = location
                .as_deref()
                .map(|path| {
                    if path.is_dir() {
                        path
                    } else {
                        path.parent().unwrap_or(base)
                    }
                })
                .unwrap_or(base);
            // Reuse the directory, never the last existing disk's filename.
            Some(directory.join(format!("new-disk-scsi{target}.chd")))
        } else {
            location
        }
    }
}

fn resolve(path: &Path, base: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

fn usable(path: &Path) -> bool {
    // SDL's Rust dialog wrapper requires UTF-8 paths without embedded NULs.
    path.to_str().is_some_and(|text| !text.contains('\0'))
}

fn existing_location(path: &Path) -> Option<PathBuf> {
    if !usable(path) {
        return None;
    }
    if path.is_file() {
        return Some(path.to_path_buf());
    }
    // Removed media or directories should still open a surviving parent.
    path.ancestors().find(|path| path.is_dir()).map(|path| {
        let mut directory = path.to_path_buf();
        directory.push(""); // Native pickers distinguish a directory from a filename.
        directory
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlay::{DiskDraft, Tab};

    #[test]
    fn history_survives_restart_and_discard_without_saving_machine_settings() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        let config = base.join("settings.cfg");
        let saved = Settings::default();
        saved.save(&config).unwrap();
        let original = fs::read(&config).unwrap();
        let mut history = History::load(&config).unwrap();
        let mut overlay = Overlay::new(&saved);
        for (field, name) in [
            (Field::Prom, "firmware/ss5.bin"),
            (Field::Scsi(3), "disks/solaris-ss5-scsi3.raw"),
            (Field::Scsi(6), "cd images/solaris.iso"),
        ] {
            let path = base.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, b"picker fixture").unwrap();
            overlay.set_path(field, path.display().to_string());
            history.remember(field, &overlay.draft, &path, base);
        }
        history.save(&config).unwrap();
        drop(overlay); // Discard the hardware/media edits.
        let history = History::load(&config).unwrap();
        let overlay = Overlay::new(&saved);
        for (field, name) in [
            (Field::Prom, "firmware/ss5.bin"),
            (Field::Rom, "firmware/ss5.bin"),
            (Field::Scsi(0), "disks/solaris-ss5-scsi3.raw"),
            (Field::Scsi(6), "cd images/solaris.iso"),
        ] {
            assert_eq!(
                history.location(&overlay, field, base),
                Some(base.join(name))
            );
        }
        assert_eq!(fs::read(&config).unwrap(), original);
        assert_eq!(Settings::load(&config).unwrap(), saved);
        assert!(
            History::load(&base.join("other.cfg"))
                .unwrap()
                .firmware
                .is_none()
        );
    }

    #[test]
    fn current_relative_image_wins_and_missing_paths_use_a_surviving_directory() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        let firmware = base.join("firmware");
        fs::create_dir(&firmware).unwrap();
        let first = firmware.join("first.bin");
        let second = firmware.join("second.bin");
        fs::write(&first, []).unwrap();
        fs::write(&second, []).unwrap();
        let mut overlay = Overlay::new(&Settings::default());
        let mut history = History::default();
        history.remember(Field::Prom, &overlay.draft, &first, base);
        overlay.set_path(Field::Prom, "firmware/second.bin".into());
        assert_eq!(
            history.location(&overlay, Field::Prom, base),
            Some(second.clone())
        );
        fs::remove_file(second).unwrap();
        assert_eq!(
            history.location(&overlay, Field::Prom, base),
            Some(firmware.clone())
        );
        overlay.set_path(Field::Prom, String::new());
        fs::remove_file(first).unwrap();
        fs::remove_dir(firmware).unwrap();
        assert_eq!(
            history.location(&overlay, Field::Prom, base),
            Some(base.to_path_buf())
        );
    }

    #[test]
    fn new_disk_uses_last_disk_directory_with_a_new_scsi_filename() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        let disks = base.join("disks");
        fs::create_dir(&disks).unwrap();
        let existing = disks.join("valuable-scsi3.chd");
        fs::write(&existing, b"preserve").unwrap();
        let mut history = History::default();
        let mut overlay = Overlay::new(&Settings::default());
        history.remember(Field::Scsi(3), &overlay.draft, &existing, base);
        overlay.tab = Tab::Media;
        overlay.new_disk = Some(DiskDraft {
            path: String::new(),
            size_mib: 1,
            target: 2,
        });
        assert_eq!(
            history.location(&overlay, Field::NewDiskPath, base),
            Some(disks.join("new-disk-scsi2.chd"))
        );
        overlay.set_path(Field::NewDiskPath, "disks/chosen-scsi2.chd".into());
        assert_eq!(
            history.location(&overlay, Field::NewDiskPath, base),
            Some(disks.join("chosen-scsi2.chd"))
        );
        assert_eq!(fs::read(existing).unwrap(), b"preserve");
    }

    #[test]
    fn invalid_history_reports_errors_and_unsafe_locations_are_not_passed_to_sdl() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("settings.cfg");
        let cache = settings::companion_path(&config, ".picker.toml");
        fs::write(&cache, "not valid TOML").unwrap();
        assert!(History::load(&config).is_err());
        fs::remove_file(&cache).unwrap();
        fs::create_dir(&cache).unwrap();
        assert!(History::default().save(&config).is_err());
        assert!(!config.exists());
        let history = History {
            firmware: Some("bad\0path".into()),
            ..History::default()
        };
        let overlay = Overlay::new(&Settings::default());
        assert!(
            history
                .location(&overlay, Field::Prom, dir.path())
                .is_none()
        );
    }
}
