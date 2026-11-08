use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub const RAM_MIB: u32 = 128;
pub const SCSI_TARGETS: usize = 7; // Narrow SCSI: ID 7 belongs to the host adapter.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    #[default]
    Disconnected,
    Disk,
    Cdrom,
}

impl TargetKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Disconnected => "Disconnected",
            Self::Disk => "Hard disk",
            Self::Cdrom => "CD-ROM",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Disconnected => Self::Disk,
            Self::Disk => Self::Cdrom,
            Self::Cdrom => Self::Disconnected,
        }
    }
    pub fn media_kind(self) -> Option<helios::devices::scsi::MediaKind> {
        match self {
            Self::Disconnected => None,
            Self::Disk => Some(helios::devices::scsi::MediaKind::Disk),
            Self::Cdrom => Some(helios::devices::scsi::MediaKind::Cdrom),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ScsiTarget {
    pub kind: TargetKind,
    pub path: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Model {
    #[default]
    Sparcstation5,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub model: Model,
    pub ram_mib: u32,
    pub prom: String,
    pub cgsix_rom: String,
    pub scsi: [ScsiTarget; SCSI_TARGETS],
    pub disk_read_only: bool,
    pub network: bool,
    pub smoothing: bool,
    pub scanlines: bool,
    pub show_status: bool,
    pub tinker: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            model: Model::Sparcstation5,
            ram_mib: RAM_MIB,
            prom: String::new(),
            cgsix_rom: String::new(),
            scsi: std::array::from_fn(|id| ScsiTarget {
                kind: match id {
                    3 => TargetKind::Disk,
                    6 => TargetKind::Cdrom,
                    _ => TargetKind::Disconnected,
                },
                path: String::new(),
            }),
            disk_read_only: false,
            network: true,
            smoothing: false,
            scanlines: false,
            show_status: true,
            tinker: false,
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Result<Self, String> {
        match fs::read_to_string(path) {
            Ok(text) => {
                // Migrate the early two-slot configuration without losing its
                // disk/CD selections. New saves always use all seven targets.
                let mut value: toml::Value =
                    toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
                let mut legacy = Vec::new();
                if let Some(table) = value.as_table_mut() {
                    for (key, id, kind) in [
                        ("disk", 3, TargetKind::Disk),
                        ("cdrom", 6, TargetKind::Cdrom),
                    ] {
                        if let Some(value) = table.remove(key) {
                            let image = value
                                .as_str()
                                .ok_or_else(|| format!("{key} must be a path string"))?;
                            legacy.push((id, kind, image.to_string()));
                        }
                    }
                    if table.contains_key("scsi")
                        && legacy.iter().any(|(_, _, image)| !image.is_empty())
                    {
                        return Err("Use scsi targets or legacy disk/cdrom paths, not both".into());
                    }
                }
                let mut settings: Self = value
                    .try_into()
                    .map_err(|e| format!("{}: {e}", path.display()))?;
                for (id, kind, image) in legacy {
                    if !image.is_empty() {
                        settings.scsi[id] = ScsiTarget { kind, path: image };
                    }
                }
                settings.validate()?;
                Ok(settings)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.ram_mib != RAM_MIB {
            return Err("This release supports only SPARCstation 5 with 128 MiB RAM".into());
        }
        Ok(())
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        self.validate_files(path.parent().unwrap_or(Path::new(".")))?;
        atomic_write(
            path,
            toml::to_string_pretty(self)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
    }

    fn validate_files(&self, base: &Path) -> Result<(), String> {
        let resolved = self.resolve(base);
        let mut images = vec![
            (
                "Sun PROM".to_string(),
                &resolved.prom,
                1,
                helios::machine::MAX_PROM_BYTES as u64,
            ),
            ("TurboGX ROM".to_string(), &resolved.cgsix_rom, 32768, 32768),
        ];
        for (id, target) in resolved.scsi.iter().enumerate() {
            if let Some(kind) = target.kind.media_kind() {
                let block = u64::from(kind.block_size());
                images.push((
                    format!("SCSI {id} {}", target.kind.label()),
                    &target.path,
                    block,
                    block * (1u64 << 32),
                ));
            }
        }
        for (label, path, block, maximum) in images {
            if path.is_empty() {
                continue;
            }
            let metadata = fs::metadata(path).map_err(|e| format!("{label}: {path}: {e}"))?;
            if !metadata.is_file()
                || metadata.len() == 0
                || metadata.len() > maximum
                || metadata.len() % block != 0
            {
                return Err(format!(
                    "{label}: expected a regular file in {block}-byte blocks, at most {maximum} bytes"
                ));
            }
            fs::File::open(path).map_err(|e| format!("{label}: {path}: {e}"))?;
        }
        if !self.disk_read_only {
            for (id, target) in resolved.scsi.iter().enumerate() {
                if target.kind == TargetKind::Disk && !target.path.is_empty() {
                    fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(&target.path)
                        .map_err(|e| format!("SCSI {id} Hard disk: {}: {e}", target.path))?;
                }
            }
        }
        Ok(())
    }

    pub fn hardware_eq(&self, other: &Self) -> bool {
        self.prom == other.prom
            && self.cgsix_rom == other.cgsix_rom
            && self.scsi == other.scsi
            && self.disk_read_only == other.disk_read_only
            && self.network == other.network
    }

    // Paths in a config are relative to that config, not the launch directory.
    pub fn resolve(&self, base: &Path) -> Self {
        let mut settings = self.clone();
        for path in [&mut settings.prom, &mut settings.cgsix_rom]
            .into_iter()
            .chain(settings.scsi.iter_mut().map(|target| &mut target.path))
        {
            if !path.is_empty() && Path::new(path).is_relative() {
                *path = base.join(&*path).to_string_lossy().into_owned();
            }
        }
        settings
    }
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let write = || -> std::io::Result<()> {
        fs::create_dir_all(parent)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        file.write_all(bytes)?;
        file.as_file().sync_all()?;
        file.persist(path).map_err(|e| e.error)?;
        Ok(())
    };
    write().map_err(|e| format!("Saving {}: {e}", path.display()))
}

pub fn companion_path(config: &Path, suffix: &str) -> PathBuf {
    let mut path = config.as_os_str().to_os_string();
    path.push(suffix);
    path.into()
}

/// The OS releases this lock even after a crash. Keep the inode in place so a
/// second instance cannot race an unlink and write the same NVRAM/configuration.
pub fn lock_config(config: &Path) -> Result<fs::File, String> {
    let path = companion_path(config, ".lock");
    fs::create_dir_all(config.parent().unwrap_or(Path::new("."))).map_err(|e| e.to_string())?;
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.try_lock().map_err(|e| {
        format!(
            "Cannot lock {} (another Heliacal instance may be using it): {e}",
            config.display()
        )
    })?;
    Ok(file)
}

pub fn default_path() -> Result<PathBuf, String> {
    if let Some(root) = std::env::var_os("XDG_CONFIG_HOME").filter(|p| Path::new(p).is_absolute()) {
        return Ok(PathBuf::from(root).join("heliacal/heliacal.cfg"));
    }
    if cfg!(target_os = "windows")
        && let Some(root) = std::env::var_os("APPDATA")
    {
        return Ok(PathBuf::from(root).join("heliacal/heliacal.cfg"));
    }
    let home = std::env::var_os("HOME").ok_or("Set HOME or pass --config PATH")?;
    let root = if cfg!(target_os = "macos") {
        "Library/Application Support"
    } else {
        ".config"
    };
    Ok(PathBuf::from(home).join(root).join("heliacal/heliacal.cfg"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disk_access_defaults_to_read_write_and_roundtrips_read_only() {
        assert!(!Settings::default().disk_read_only);
        assert!(Settings::default().network);
        let missing: Settings = toml::from_str("ram_mib = 128").unwrap();
        assert!(!missing.disk_read_only);
        let read_only: Settings = toml::from_str("disk_read_only = true").unwrap();
        assert!(read_only.disk_read_only);
    }
    #[test]
    fn unsupported_hardware_is_rejected() {
        assert!(toml::from_str::<Settings>("model = 'ultra5'").is_err());
        assert!(
            Settings {
                ram_mib: 64,
                ..Settings::default()
            }
            .validate()
            .is_err()
        );
        assert!(toml::from_str::<Settings>("ram_mb = 64").is_err());
    }
    #[test]
    fn save_roundtrip_and_failure_preserve_existing_settings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.cfg");
        fs::create_dir(dir.path().join("firmware")).unwrap();
        fs::write(dir.path().join("firmware/ss5.bin"), [1, 2, 3, 4]).unwrap();
        let mut s = Settings {
            prom: "firmware/ss5.bin".into(),
            ..Settings::default()
        };
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path).unwrap(), s);
        s.ram_mib = 256;
        assert!(s.save(&path).is_err());
        assert_eq!(Settings::load(&path).unwrap().ram_mib, 128);
        s.ram_mib = 128;
        assert!(s.save(&path.join("blocked")).is_err());
        assert_eq!(Settings::load(&path).unwrap().prom, "firmware/ss5.bin");
    }
    #[test]
    fn missing_images_and_concurrent_instances_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("machine.cfg");
        let lock = lock_config(&config).unwrap();
        assert!(lock_config(&config).is_err());
        drop(lock);
        assert!(lock_config(&config).is_ok());
        let s = Settings {
            prom: "missing.bin".into(),
            ..Settings::default()
        };
        assert!(s.save(&config).is_err());
        assert!(!config.exists());
        assert_ne!(
            companion_path(Path::new("state.nvram"), ".nvram"),
            Path::new("state.nvram")
        );
    }
    #[test]
    fn paths_follow_config_directory() {
        let s = Settings {
            prom: "rom.bin".into(),
            ..Settings::default()
        };
        assert_eq!(
            s.resolve(Path::new("/tmp/machine")).prom,
            "/tmp/machine/rom.bin"
        );
        assert!(s.resolve(Path::new("/tmp/machine")).scsi[3].path.is_empty());
    }
    #[test]
    fn scsi_targets_roundtrip_validate_media_and_migrate_legacy_slots() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("machine.cfg");
        fs::write(dir.path().join("disk.raw"), vec![0; 512]).unwrap();
        fs::write(dir.path().join("cd.iso"), vec![0; 2048]).unwrap();
        let mut s = Settings::default();
        for (id, target) in s.scsi.iter_mut().enumerate() {
            *target = if id % 2 == 0 {
                ScsiTarget {
                    kind: TargetKind::Disk,
                    path: "disk.raw".into(),
                }
            } else {
                ScsiTarget {
                    kind: TargetKind::Cdrom,
                    path: "cd.iso".into(),
                }
            };
        }
        s.save(&config).unwrap();
        assert_eq!(Settings::load(&config).unwrap(), s);
        s.scsi[0].kind = TargetKind::Cdrom;
        assert!(s.save(&config).unwrap_err().contains("SCSI 0"));
        assert_eq!(
            Settings::load(&config).unwrap().scsi[0].kind,
            TargetKind::Disk
        );
        s.scsi[0] = ScsiTarget {
            kind: TargetKind::Disconnected,
            path: "unavailable.raw".into(),
        };
        s.save(&config).unwrap(); // Disabled devices do not require their image.
        fs::write(&config, "disk = 'disk.raw'\ncdrom = 'cd.iso'\n").unwrap();
        let legacy = Settings::load(&config).unwrap();
        assert_eq!(legacy.scsi[3].path, "disk.raw");
        assert_eq!(legacy.scsi[6].kind, TargetKind::Cdrom);
        legacy.save(&config).unwrap();
        assert!(!fs::read_to_string(&config).unwrap().contains("\ndisk ="));
        assert!(toml::from_str::<Settings>("scsi = []").is_err());
    }
}
