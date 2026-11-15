use helios::machine::InstalledVideo;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Display {
    #[default]
    Cgsix,
    Sx,
}

impl Display {
    pub fn label(self) -> &'static str {
        match self {
            Self::Cgsix => "TurboGX / CGsix",
            Self::Sx => "Native SX / CG14",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Cgsix => Self::Sx,
            Self::Sx => Self::Cgsix,
        }
    }

    pub const fn installed_video(self) -> InstalledVideo {
        match self {
            Self::Cgsix => InstalledVideo::None,
            Self::Sx => InstalledVideo::Sx,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Model {
    #[default]
    Sparcstation5,
    Sparcstation20,
    Axil320,
}

impl Model {
    pub const ALL: [Self; 3] = [Self::Sparcstation5, Self::Sparcstation20, Self::Axil320];

    pub fn label(self) -> &'static str {
        match self {
            Self::Sparcstation5 => "Sun SPARCstation 5",
            Self::Sparcstation20 => "Sun SPARCstation 20",
            Self::Axil320 => "Axil 320",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Sparcstation5 => "SS-5",
            Self::Sparcstation20 => "SS-20",
            Self::Axil320 => "Axil 320",
        }
    }

    pub fn processor(self) -> &'static str {
        match self {
            Self::Sparcstation5 => "microSPARC-II / 85 MHz",
            Self::Sparcstation20 => "SuperSPARC-II / 75 MHz",
            Self::Axil320 => "SuperSPARC-II / 75 MHz",
        }
    }

    pub fn graphics(self, display: Display) -> &'static str {
        match (self, display) {
            (Self::Sparcstation20, Display::Sx) => "SX / CG14 / 1152 x 900",
            (Self::Sparcstation20, Display::Cgsix) => "TurboGX / CGsix / SBus slot 2",
            (_, Display::Cgsix) => "TurboGX / CGsix / 1152 x 900",
            (_, Display::Sx) => "SX / CG14 / 1152 x 900",
        }
    }

    pub fn graphics_short(self, display: Display) -> &'static str {
        match display {
            Display::Cgsix => "CGsix",
            Display::Sx => "SX/CG14",
        }
    }

    pub fn prom_label(self) -> &'static str {
        match self {
            Self::Sparcstation5 => "Sun SS-5 PROM",
            Self::Sparcstation20 => "Sun SS-20 PROM",
            Self::Axil320 => "Axil 320 PROM",
        }
    }

    pub fn profile(self) -> helios::machine::MachineProfile {
        match self {
            Self::Sparcstation5 => helios::machine::MachineProfile::Ss5,
            Self::Sparcstation20 => helios::machine::MachineProfile::Ss20,
            Self::Axil320 => helios::machine::MachineProfile::Axil320,
        }
    }

    pub fn default_nvram(self) -> helios::devices::nvram::Nvram {
        self.profile().default_nvram()
    }

    pub fn supports_sx(self) -> bool {
        matches!(self, Self::Sparcstation20)
    }

    pub fn next(self) -> Self {
        let index = Self::ALL.iter().position(|model| *model == self).unwrap();
        Self::ALL[(index + 1) % Self::ALL.len()]
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_label: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub machines: BTreeMap<String, crate::profiles::NamedMachine>,
    pub model: Model,
    pub display: Display,
    pub headless: bool,
    pub serial_pty_link: String,
    pub cpus: u8,
    pub accurate_cpu_clocks: bool,
    pub ram_mib: u32,
    pub prom: String,
    pub cgsix_rom: String,
    pub scsi: [ScsiTarget; SCSI_TARGETS],
    pub disk_read_only: bool,
    pub network: bool,
    pub smoothing: bool,
    pub scanlines: bool,
    pub show_status: bool,
    pub show_serial_console: bool,
    pub tinker: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            machine_label: None,
            machines: BTreeMap::new(),
            model: Model::Sparcstation5,
            display: Display::Cgsix,
            headless: false,
            serial_pty_link: String::new(),
            cpus: 1,
            accurate_cpu_clocks: false,
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
            show_serial_console: true,
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
                migrate_media(&mut value)?;
                if let Some(machines) = value
                    .get_mut("machines")
                    .and_then(toml::Value::as_table_mut)
                {
                    for (_, machine) in machines.iter_mut() {
                        migrate_media(machine)?;
                    }
                }
                let mut settings: Self = value
                    .try_into()
                    .map_err(|e| format!("{}: {e}", path.display()))?;
                if let Some(label) = settings.machine_label.clone() {
                    settings.select_machine(&label)?;
                }
                settings.validate()?;
                Ok(settings)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        self.validate_hardware()?;
        self.validate_machines()
    }

    pub fn validate_hardware(&self) -> Result<(), String> {
        if self
            .model
            .profile()
            .processor_topology(usize::from(self.cpus))
            .is_none()
        {
            return Err(format!(
                "{} supports {} in this release (requested {})",
                self.model.label(),
                if self.model == Model::Sparcstation5 {
                    "1 CPU"
                } else {
                    "1 or 2 CPUs; dual-CPU MBus modules are not implemented"
                },
                self.cpus,
            ));
        }
        if self.display == Display::Sx && !self.model.supports_sx() {
            return Err(format!(
                "{} does not have a native SX display",
                self.model.label()
            ));
        }
        self.model
            .profile()
            .ram_layout(self.ram_bytes()?, self.installed_video())
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn installed_video(&self) -> InstalledVideo {
        if self.headless {
            InstalledVideo::None
        } else {
            self.display.installed_video()
        }
    }

    pub fn ram_bytes(&self) -> Result<usize, String> {
        usize::try_from(self.ram_mib)
            .ok()
            .and_then(|mib| mib.checked_mul(1024 * 1024))
            .ok_or_else(|| format!("RAM size overflow: {} MiB", self.ram_mib))
    }

    /// Exact memory choices supported by the selected model and display.
    pub fn valid_ram_mib(&self) -> Result<Vec<u32>, String> {
        let totals = self
            .model
            .profile()
            .valid_ram_totals(self.installed_video())
            .map_err(|error| error.to_string())?;
        totals
            .into_iter()
            .map(|bytes| {
                u32::try_from(bytes / (1024 * 1024))
                    .map_err(|_| "RAM size does not fit the settings format".to_string())
            })
            .collect()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let mut saved = self.clone();
        saved.remember_machine();
        saved.validate()?;
        saved.validate_files(path.parent().unwrap_or(Path::new(".")))?;
        atomic_write(
            path,
            toml::to_string_pretty(&saved)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
    }

    fn validate_files(&self, base: &Path) -> Result<(), String> {
        let resolved = self.resolve(base);
        let mut images = vec![(
            "Sun PROM".to_string(),
            &resolved.prom,
            1,
            helios::machine::MAX_PROM_BYTES as u64,
        )];
        if !resolved.headless && resolved.display == Display::Cgsix {
            images.push(("TurboGX ROM".to_string(), &resolved.cgsix_rom, 32768, 32768));
        }
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
        if !resolved.headless
            && resolved.display == Display::Cgsix
            && !resolved.cgsix_rom.is_empty()
        {
            let rom = fs::read(&resolved.cgsix_rom).map_err(|e| format!("TurboGX ROM: {e}"))?;
            helios::devices::cgsix::Cgsix::validate_rom(&rom)
                .map_err(|e| format!("TurboGX ROM: {e:?}"))?;
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
        self.machine_label == other.machine_label
            && self.nvram_path(Path::new("nvram")) == other.nvram_path(Path::new("nvram"))
            && self.model == other.model
            && self.cpus == other.cpus
            && self.accurate_cpu_clocks == other.accurate_cpu_clocks
            && self.display == other.display
            && self.headless == other.headless
            && self.serial_pty_link == other.serial_pty_link
            && self.ram_mib == other.ram_mib
            && self.prom == other.prom
            && self.cgsix_rom == other.cgsix_rom
            && self.scsi == other.scsi
            && self.disk_read_only == other.disk_read_only
            && self.network == other.network
    }

    // Paths in a config are relative to that config, not the launch directory.
    pub fn resolve(&self, base: &Path) -> Self {
        let mut settings = self.clone();
        settings.resolve_paths(base);
        for machine in settings.machines.values_mut() {
            machine.resolve(base);
        }
        settings
    }

    pub fn resolve_paths(&mut self, base: &Path) {
        for path in [
            &mut self.prom,
            &mut self.cgsix_rom,
            &mut self.serial_pty_link,
        ]
        .into_iter()
        .chain(self.scsi.iter_mut().map(|target| &mut target.path))
        {
            if !path.is_empty() && Path::new(path).is_relative() {
                *path = base.join(&*path).to_string_lossy().into_owned();
            }
        }
    }
}

// The same short disk/CD spelling works in old configs and named stanzas.
fn migrate_media(value: &mut toml::Value) -> Result<(), String> {
    let Some(table) = value.as_table_mut() else {
        return Ok(());
    };
    let mut targets = Settings::default().scsi;
    let mut has_media = false;
    for (key, id, kind) in [
        ("disk", 3, TargetKind::Disk),
        ("cdrom", 6, TargetKind::Cdrom),
    ] {
        if let Some(value) = table.remove(key) {
            let image = value
                .as_str()
                .ok_or_else(|| format!("{key} must be a path string"))?;
            if !image.is_empty() {
                targets[id] = ScsiTarget {
                    kind,
                    path: image.into(),
                };
                has_media = true;
            }
        }
    }
    if has_media {
        if table.contains_key("scsi") {
            return Err("Use scsi targets or legacy disk/cdrom paths, not both".into());
        }
        table.insert(
            "scsi".into(),
            toml::Value::try_from(targets).map_err(|e| e.to_string())?,
        );
    }
    Ok(())
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
    fn clock_mode_defaults_fast_and_roundtrips_per_named_machine() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.cfg");
        let mut settings: Settings = toml::from_str("model = 'sparcstation20'").unwrap();
        assert!(!settings.accurate_cpu_clocks);
        settings.save_as_machine("Fast").unwrap();
        settings.save_as_machine("Accurate").unwrap();
        let nvram = settings.nvram_path(&path);
        settings.accurate_cpu_clocks = true;
        settings.save(&path).unwrap();
        let mut loaded = Settings::load(&path).unwrap();
        assert!(loaded.accurate_cpu_clocks);
        assert_eq!(loaded.nvram_path(&path), nvram);
        loaded.select_machine("Fast").unwrap();
        assert!(!loaded.accurate_cpu_clocks);
        loaded.select_machine("Accurate").unwrap();
        assert!(loaded.accurate_cpu_clocks);
        let old: crate::profiles::NamedMachine = toml::from_str("model = 'axil320'").unwrap();
        assert!(!old.accurate_cpu_clocks);
    }

    #[test]
    fn incompatible_turbogx_plus_rom_is_rejected_transactionally() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("machine.cfg");
        let rom = dir.path().join("renamed.rom");
        let mut settings = Settings {
            cgsix_rom: rom.display().to_string(),
            ..Settings::default()
        };
        fs::write(&rom, vec![0; 32768]).unwrap();
        settings.save(&config).unwrap();
        let original = fs::read(&config).unwrap();
        // Synthetic CRC-32 fc7be95e fixture; contains no firmware.
        let mut incompatible = vec![0; 32768];
        incompatible[32764..].copy_from_slice(&[0x2c, 0x1d, 0x32, 0x02]);
        fs::write(&rom, incompatible).unwrap();
        let error = settings.save(&config).unwrap_err();
        assert!(error.contains("TurboGX+"), "{error}");
        assert!(error.contains("turbogx.bin"), "{error}");
        assert_eq!(fs::read(&config).unwrap(), original);
        settings.headless = true;
        settings.save(&config).unwrap(); // No graphics card is constructed.
    }

    #[test]
    fn cpu_counts_default_to_one_validate_and_require_cold_replacement() {
        let old: Settings = toml::from_str("model = 'sparcstation20'").unwrap();
        assert_eq!(old.cpus, 1);
        for model in Model::ALL {
            for cpus in [0, 1, 2, 3, 4, 255] {
                let s = Settings {
                    model,
                    cpus,
                    ..Settings::default()
                };
                assert_eq!(
                    s.validate_hardware().is_ok(),
                    cpus == 1 || (cpus == 2 && model != Model::Sparcstation5),
                    "{model:?}/{cpus}"
                );
            }
        }
        let dual = Settings {
            cpus: 2,
            ..old.clone()
        };
        assert!(!old.hardware_eq(&dual));
        let roundtrip: Settings = toml::from_str(&toml::to_string(&dual).unwrap()).unwrap();
        assert_eq!(dual, roundtrip);
        for text in ["cpus = -1", "cpus = 1.5", "cpus = '2'"] {
            assert!(toml::from_str::<Settings>(text).is_err());
        }
    }

    #[test]
    fn ram_choices_come_from_the_shared_core_model_and_display() {
        let mut settings = Settings::default();
        let choices = settings.valid_ram_mib().unwrap();
        assert!(choices.contains(&8) && choices.contains(&128) && choices.contains(&256));
        assert!(!choices.contains(&240));
        assert!(settings.validate_hardware().is_ok());

        settings.ram_mib = 232;
        assert!(settings.validate_hardware().is_ok());
        assert!(!settings.hardware_eq(&Settings::default()));
        settings.ram_mib = 240;
        assert!(settings.validate_hardware().is_err());
        assert!(!settings.hardware_eq(&Settings::default()));

        settings.model = Model::Sparcstation20;
        settings.display = Display::Sx;
        let sx = settings.valid_ram_mib().unwrap();
        assert!(sx.contains(&448) && !sx.contains(&432) && !sx.contains(&512));
        settings.ram_mib = 448;
        assert!(settings.validate_hardware().is_ok());
        settings.ram_mib = 512;
        assert!(settings.validate_hardware().is_err());

        settings.display = Display::Cgsix;
        let cgsix = settings.valid_ram_mib().unwrap();
        assert!(cgsix.contains(&512) && !cgsix.contains(&496));
        settings.ram_mib = 512;
        assert!(settings.validate_hardware().is_ok());

        settings.model = Model::Axil320;
        assert!(settings.validate_hardware().is_ok());
        let axil = settings.valid_ram_mib().unwrap();
        assert!(axil.contains(&32) && axil.contains(&416) && axil.contains(&512));
        assert!(!axil.contains(&432) && !axil.contains(&480));
        settings.ram_mib = 480;
        assert!(settings.validate_hardware().is_err());
        settings.display = Display::Sx;
        assert!(settings.validate_hardware().is_err());
    }

    #[test]
    fn old_named_records_default_ram_and_inactive_invalid_records_fail() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("heliacal.cfg");
        fs::write(
            &config,
            r#"
[machines.old]
model = "sparcstation20"
display = "sx"
[machines.invalid]
model = "sparcstation5"
ram_mib = 4
"#,
        )
        .unwrap();
        let error = Settings::load(&config).unwrap_err();
        assert!(error.contains("Machine 'invalid'"));
        assert!(error.contains("SPARCstation 5"));

        fs::write(
            &config,
            r#"
[machines.old]
model = "sparcstation20"
display = "sx"
"#,
        )
        .unwrap();
        let settings = Settings::load(&config).unwrap();
        assert_eq!(settings.machines["old"].ram_mib, RAM_MIB);
        assert!(settings.machines["old"].validate().is_ok());
    }

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
    fn machine_models_roundtrip_and_cycle() {
        assert_eq!(Model::default(), Model::Sparcstation5);
        assert_eq!(Model::Sparcstation5.next(), Model::Sparcstation20);
        assert_eq!(Model::Sparcstation20.next(), Model::Axil320);
        assert_eq!(Model::Axil320.next(), Model::Sparcstation5);
        let s: Settings = toml::from_str("model = 'axil320'").unwrap();
        assert_eq!(s.model, Model::Axil320);
        assert_eq!(s.display, Display::Cgsix);
        assert_eq!(
            toml::from_str::<Settings>("display = 'sx'")
                .unwrap()
                .display,
            Display::Sx
        );
        assert!(
            Settings {
                display: Display::Sx,
                ..Settings::default()
            }
            .validate()
            .is_err()
        );
        assert_eq!(s.model.profile(), helios::machine::MachineProfile::Axil320);
        assert!(!s.model.supports_sx());
        assert!(Model::Sparcstation20.supports_sx());
        assert!(!s.hardware_eq(&Settings::default()));
    }
    #[test]
    fn unsupported_hardware_is_rejected() {
        assert!(toml::from_str::<Settings>("model = 'ultra5'").is_err());
        assert!(
            Settings {
                ram_mib: 4,
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
        s.ram_mib = 240;
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
