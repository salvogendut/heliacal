//! Named workstations share desktop preferences but keep their hardware and state.
use crate::settings::{Display, Model, SCSI_TARGETS, ScsiTarget, Settings, companion_path};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NamedMachine {
    /// Naming the existing unnamed workstation keeps its firmware state.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub legacy_state: bool,
    pub model: Model,
    pub display: Display,
    pub ram_mib: u32,
    pub prom: String,
    pub cgsix_rom: String,
    pub scsi: [ScsiTarget; SCSI_TARGETS],
    pub disk_read_only: bool,
    pub network: bool,
}

impl Default for NamedMachine {
    fn default() -> Self {
        Self::capture(&Settings::default(), false)
    }
}

impl NamedMachine {
    fn capture(settings: &Settings, legacy_state: bool) -> Self {
        Self {
            legacy_state,
            model: settings.model,
            display: settings.display,
            ram_mib: settings.ram_mib,
            prom: settings.prom.clone(),
            cgsix_rom: settings.cgsix_rom.clone(),
            scsi: settings.scsi.clone(),
            disk_read_only: settings.disk_read_only,
            network: settings.network,
        }
    }

    fn apply(&self, settings: &mut Settings) {
        settings.model = self.model;
        settings.display = self.display;
        settings.ram_mib = self.ram_mib;
        settings.prom = self.prom.clone();
        settings.cgsix_rom = self.cgsix_rom.clone();
        settings.scsi = self.scsi.clone();
        settings.disk_read_only = self.disk_read_only;
        settings.network = self.network;
    }

    pub fn validate(&self) -> Result<(), String> {
        let mut settings = Settings::default();
        self.apply(&mut settings);
        settings.validate_hardware()
    }

    pub fn resolve(&mut self, base: &Path) {
        let mut settings = Settings::default();
        self.apply(&mut settings);
        settings.resolve_paths(base);
        *self = Self::capture(&settings, self.legacy_state);
    }
}

fn validate_label(label: &str) -> Result<(), String> {
    if label.is_empty()
        || label.len() > 32
        || label.trim() != label
        || !label.bytes().all(|b| b.is_ascii_graphic() || b == b' ')
    {
        return Err("Use 1-32 printable ASCII characters, without leading/trailing spaces".into());
    }
    Ok(())
}

impl Settings {
    pub fn save_as_machine(&mut self, label: &str) -> Result<(), String> {
        validate_label(label)?;
        if self.machines.contains_key(label) {
            return Err("That label already exists; select it to edit its settings".into());
        }
        // Save As copies the draft without overwriting its named source.
        let machine = NamedMachine::capture(self, self.machine_label.is_none());
        self.machines.insert(label.into(), machine);
        self.machine_label = Some(label.into());
        Ok(())
    }

    pub fn remember_machine(&mut self) {
        if let Some(label) = self.machine_label.clone()
            && let Some(saved) = self.machines.get(&label)
        {
            let machine = NamedMachine::capture(self, saved.legacy_state);
            self.machines.insert(label, machine);
        }
    }

    pub fn select_machine(&mut self, label: &str) -> Result<(), String> {
        let saved = self
            .machines
            .get(label)
            .cloned()
            .ok_or_else(|| format!("Unknown machine label: {label}"))?;
        saved.apply(self);
        self.machine_label = Some(label.into());
        Ok(())
    }

    pub fn next_machine(&mut self) -> bool {
        if self.machines.is_empty() {
            return false;
        }
        self.remember_machine();
        let labels: Vec<_> = self.machines.keys().cloned().collect();
        let index = self
            .machine_label
            .as_ref()
            .and_then(|label| labels.iter().position(|item| item == label))
            .map_or(0, |index| (index + 1) % labels.len());
        self.select_machine(&labels[index]).expect("catalog label");
        true
    }

    pub fn validate_machines(&self) -> Result<(), String> {
        for (label, machine) in &self.machines {
            validate_label(label)?;
            machine
                .validate()
                .map_err(|error| format!("Machine '{label}': {error}"))?;
        }
        if let Some(label) = &self.machine_label
            && !self.machines.contains_key(label)
        {
            return Err(format!("Unknown machine label: {label}"));
        }
        Ok(())
    }

    pub fn machine_name(&self) -> String {
        self.machine_label.as_ref().map_or_else(
            || self.model.label().into(),
            |label| format!("{label} / {}", self.model.short_label()),
        )
    }

    pub fn machine_description(&self) -> String {
        format!(
            "{} / {} MiB / {} / NAT {}",
            self.machine_name(),
            self.ram_mib,
            self.model.graphics_short(self.display),
            if self.network { "on" } else { "off" }
        )
    }

    pub fn nvram_path(&self, base: &Path) -> PathBuf {
        let base = if let Some(label) = &self.machine_label
            && self
                .machines
                .get(label)
                .is_some_and(|machine| !machine.legacy_state)
        {
            // Full byte encoding avoids case/path collisions and traversal.
            let encoded: String = label.bytes().map(|b| format!("{b:02x}")).collect();
            companion_path(base, ".machines")
                .join(encoded)
                .join("nvram")
        } else {
            base.to_path_buf()
        };
        match self.model {
            Model::Sparcstation5 => base,
            Model::Sparcstation20 => companion_path(&base, ".ss20"),
            Model::Axil320 => companion_path(&base, ".axil320"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::TargetKind;
    use std::fs;

    #[test]
    fn named_stanzas_override_flat_defaults_and_resolve_every_dependency() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("heliacal.cfg");
        fs::write(dir.path().join("ss20.rom"), [1; 4]).unwrap();
        fs::write(dir.path().join("solaris.raw"), [0; 512]).unwrap();
        fs::write(dir.path().join("install.iso"), [0; 2048]).unwrap();
        fs::write(
            &config,
            r#"
machine_label = "SS20 Solaris"
smoothing = true
[machines."SS20 Solaris"]
model = "sparcstation20"
display = "sx"
prom = "ss20.rom"
disk = "solaris.raw"
cdrom = "install.iso"
network = false
disk_read_only = true
[machines.Axil]
model = "axil320"
prom = "offline.rom"
cgsix_rom = "gx.rom"
disk = "axil.raw"
"#,
        )
        .unwrap();
        let settings = Settings::load(&config).unwrap();
        assert_eq!(settings.model, Model::Sparcstation20);
        assert_eq!(settings.display, Display::Sx);
        assert!(settings.smoothing && settings.disk_read_only && !settings.network);
        assert_eq!(settings.scsi[3].path, "solaris.raw");
        assert_eq!(settings.scsi[6].kind, TargetKind::Cdrom);
        let mut resolved = settings.resolve(dir.path());
        assert_eq!(resolved.prom, dir.path().join("ss20.rom").to_str().unwrap());
        resolved.select_machine("Axil").unwrap();
        assert_eq!(
            resolved.prom,
            dir.path().join("offline.rom").to_str().unwrap()
        );
        assert_eq!(
            resolved.cgsix_rom,
            dir.path().join("gx.rom").to_str().unwrap()
        );
        assert_eq!(
            resolved.scsi[3].path,
            dir.path().join("axil.raw").to_str().unwrap()
        );
        // Offline media belonging to inactive labels do not prevent saving.
        settings.save(&config).unwrap();
        assert_eq!(Settings::load(&config).unwrap(), settings);
        assert!(
            fs::read_to_string(&config)
                .unwrap()
                .contains("[machines.\"SS20 Solaris\"]")
        );
    }

    #[test]
    fn save_as_preserves_source_and_switching_remembers_edits_until_save() {
        let mut settings = Settings {
            prom: "ss5.rom".into(),
            ..Settings::default()
        };
        settings.scsi[0] = ScsiTarget {
            kind: TargetKind::Disk,
            path: "data.raw".into(),
        };
        settings.save_as_machine("Original").unwrap();
        let original = settings.machines["Original"].clone();
        settings.model = Model::Sparcstation20;
        settings.display = Display::Sx;
        settings.prom = "ss20.rom".into();
        settings.scsi[0].path = "other.raw".into();
        settings.disk_read_only = true;
        settings.network = false;
        settings.save_as_machine("SS20").unwrap();
        assert_eq!(settings.machines["Original"], original);
        assert!(!settings.machines["SS20"].legacy_state);
        settings.prom = "updated.rom".into();
        settings.smoothing = true;
        assert!(settings.next_machine());
        assert_eq!(settings.machine_label.as_deref(), Some("Original"));
        assert_eq!(settings.prom, "ss5.rom");
        assert!(settings.smoothing);
        assert!(settings.next_machine());
        assert_eq!(settings.prom, "updated.rom");
        assert_eq!(settings.scsi[0].path, "other.raw");
        assert!(settings.disk_read_only && !settings.network);
        assert_eq!(settings.display, Display::Sx);
    }

    #[test]
    fn save_updates_selected_stanza_and_failed_save_preserves_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.cfg");
        let mut settings = Settings::default();
        settings.save_as_machine("One").unwrap();
        settings.save(&path).unwrap();
        settings.network = false;
        settings.save(&path).unwrap();
        let loaded = Settings::load(&path).unwrap();
        assert!(!loaded.network && !loaded.machines["One"].network);
        let before = fs::read(&path).unwrap();
        settings.prom = "missing.rom".into();
        assert!(settings.save(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
    }

    #[test]
    fn labels_and_selected_names_are_validated_without_mutation() {
        let mut settings = Settings::default();
        settings.save_as_machine("Sun").unwrap();
        let before = settings.clone();
        for label in [
            "",
            "Sun",
            " Sun",
            "Sun ",
            "a\nb",
            "é",
            "012345678901234567890123456789012",
        ] {
            assert!(settings.save_as_machine(label).is_err());
            assert_eq!(settings, before);
        }
        assert!(settings.select_machine("Missing").is_err());
        assert_eq!(settings, before);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.cfg");
        fs::write(&path, "machine_label = 'Missing'\n").unwrap();
        assert!(Settings::load(&path).unwrap_err().contains("Missing"));
        fs::write(
            &path,
            "[machines.Invalid]\nmodel = 'axil320'\ndisplay = 'sx'\n",
        )
        .unwrap();
        assert!(Settings::load(&path).unwrap_err().contains("Invalid"));
    }

    #[test]
    fn first_label_adopts_legacy_nvram_and_other_labels_have_distinct_safe_paths() {
        let base = Path::new("/tmp/heliacal.cfg.nvram");
        let mut settings = Settings {
            model: Model::Sparcstation20,
            ..Settings::default()
        };
        let legacy = settings.nvram_path(base);
        settings.save_as_machine("Original").unwrap();
        assert_eq!(settings.nvram_path(base), legacy);
        let mut paths = std::collections::BTreeSet::new();
        for label in ["SS20", "ss20", "../SS20", "SS/20", "SS_20"] {
            settings.save_as_machine(label).unwrap();
            let path = settings.nvram_path(base);
            assert_ne!(path, legacy);
            assert!(path.starts_with("/tmp/heliacal.cfg.nvram.machines"));
            assert!(
                !path
                    .components()
                    .any(|p| p == std::path::Component::ParentDir)
            );
            assert!(paths.insert(path));
        }
        let previous = settings.clone();
        settings.model = Model::Axil320;
        assert_ne!(settings.nvram_path(base), previous.nvram_path(base));
        settings = previous.clone();
        settings.save_as_machine("Another SS20").unwrap();
        assert!(!settings.hardware_eq(&previous));
    }
}
