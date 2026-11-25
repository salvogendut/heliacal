use crate::settings::{Display, Settings, TargetKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    General,
    Display,
    Media,
    Network,
    Advanced,
    About,
}
impl Tab {
    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Display => "Display",
            Self::Media => "Media",
            Self::Network => "Network",
            Self::Advanced => "Advanced",
            Self::About => "About",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Fixed,
    MachineLabel,
    SaveMachineAs,
    Model,
    CpuCount,
    Ram,
    Display,
    Prom,
    Rom,
    IdeDisk,
    Scsi(usize),
    NewDisk,
    NewDiskPath,
    NewDiskSize,
    NewDiskTarget,
    CreateDisk,
    CancelDisk,
    DiskAccess,
    Headless,
    SerialPtyLink,
    OpenSerialTerminal,
    SerialBreak,
    SerialConsole,
    AccurateClocks,
    RtcHostTime,
    Network,
    Smoothing,
    Scanlines,
    Status,
    Tinker,
}
impl Field {
    pub fn is_path(self) -> bool {
        matches!(
            self,
            Self::Prom
                | Self::Rom
                | Self::IdeDisk
                | Self::Scsi(_)
                | Self::SerialPtyLink
                | Self::NewDiskPath
        )
    }
}
pub struct Row {
    pub label: &'static str,
    pub value: String,
    pub field: Field,
}
#[derive(Clone, Debug)]
pub struct DiskDraft {
    pub path: String,
    pub size_mib: u64,
    pub target: usize,
}
pub struct Overlay {
    pub draft: Settings,
    pub tab: Tab,
    pub selected: usize,
    pub edit: Option<String>,
    pub notice: Option<String>,
    pub new_disk: Option<DiskDraft>,
}
impl Overlay {
    pub fn new(settings: &Settings) -> Self {
        Self {
            draft: settings.clone(),
            tab: Tab::General,
            selected: 0,
            edit: None,
            notice: None,
            new_disk: None,
        }
    }
    pub fn tabs(&self) -> Vec<Tab> {
        vec![
            Tab::General,
            Tab::Display,
            Tab::Media,
            Tab::Network,
            Tab::Advanced,
            Tab::About,
        ]
    }
    pub fn rows(&self) -> Vec<Row> {
        if let Some(disk) = &self.new_disk {
            return vec![
                Row {
                    label: "New CHD disk",
                    value: path_label(&disk.path),
                    field: Field::NewDiskPath,
                },
                Row {
                    label: "Capacity (MiB)",
                    value: disk.size_mib.to_string(),
                    field: Field::NewDiskSize,
                },
                Row {
                    label: "Attach at",
                    value: format!("SCSI ID {}", disk.target),
                    field: Field::NewDiskTarget,
                },
                Row {
                    label: "Create blank disk",
                    value: "Create file and attach to draft".into(),
                    field: Field::CreateDisk,
                },
                Row {
                    label: "Cancel",
                    value: "Return to media settings".into(),
                    field: Field::CancelDisk,
                },
            ];
        }
        match self.tab {
            Tab::General => vec![
                (
                    "Machine label",
                    self.draft
                        .machine_label
                        .clone()
                        .unwrap_or_else(|| "Unnamed".into()),
                    Field::MachineLabel,
                ),
                (
                    "Save machine as...",
                    "Enter a new label".into(),
                    Field::SaveMachineAs,
                ),
                ("Machine", self.draft.model.label().into(), Field::Model),
                (
                    "Processor",
                    self.draft.model.processor().into(),
                    Field::Fixed,
                ),
                (
                    "CPUs",
                    if self.draft.cpus == 2 {
                        "2 (experimental SMP)".into()
                    } else {
                        "1".into()
                    },
                    if matches!(
                        self.draft.model,
                        crate::settings::Model::Sparcstation5 | crate::settings::Model::Ultra5
                    ) {
                        Field::Fixed
                    } else {
                        Field::CpuCount
                    },
                ),
                (
                    "Memory",
                    format!("{} MiB", self.draft.ram_mib),
                    if self.draft.model == crate::settings::Model::Ultra5 {
                        Field::Fixed
                    } else {
                        Field::Ram
                    },
                ),
                (
                    "Graphics",
                    if self.draft.headless {
                        "Headless / serial TTYA".into()
                    } else {
                        self.draft.model.graphics(self.draft.display).into()
                    },
                    Field::Fixed,
                ),
                (
                    "Display",
                    if self.draft.headless {
                        "Headless".into()
                    } else {
                        self.draft.display.label().into()
                    },
                    if self.draft.headless || self.draft.model == crate::settings::Model::Ultra5 {
                        Field::Fixed
                    } else {
                        Field::Display
                    },
                ),
                (
                    self.draft.model.prom_label(),
                    if self.draft.prom.is_empty()
                        && self.draft.model == crate::settings::Model::Ultra5
                    {
                        "Select original 1 MiB flash".into()
                    } else if self.draft.prom.is_empty() {
                        "OpenBIOS (automatic)".into()
                    } else {
                        path_label(&self.draft.prom)
                    },
                    Field::Prom,
                ),
                (
                    "TurboGX FCode ROM",
                    path_label(&self.draft.cgsix_rom),
                    Field::Rom,
                ),
                ("Tinker", on_off(self.draft.tinker), Field::Tinker),
            ],
            Tab::Display => vec![
                ("Smoothing", on_off(self.draft.smoothing), Field::Smoothing),
                ("Scanlines", on_off(self.draft.scanlines), Field::Scanlines),
                (
                    "Shortcut hints",
                    on_off(self.draft.show_status),
                    Field::Status,
                ),
            ],
            Tab::Media if self.draft.model == crate::settings::Model::Ultra5 => vec![
                (
                    "IDE disk (primary master)",
                    path_label(&self.draft.ide_disk),
                    Field::IdeDisk,
                ),
                (
                    "IDE CD-ROM (secondary master)",
                    if self.draft.scsi[6].kind == TargetKind::Cdrom {
                        path_label(&self.draft.scsi[6].path)
                    } else {
                        path_label("")
                    },
                    Field::Scsi(6),
                ),
            ],
            Tab::Media => [
                "SCSI ID 0",
                "SCSI ID 1",
                "SCSI ID 2",
                "SCSI ID 3",
                "SCSI ID 4",
                "SCSI ID 5",
                "SCSI ID 6",
            ]
            .into_iter()
            .enumerate()
            .map(|(id, label)| {
                (
                    label,
                    path_label(&self.draft.scsi[id].path),
                    Field::Scsi(id),
                )
            })
            .chain(std::iter::once((
                "New blank CHD disk...",
                "Choose capacity and destination".into(),
                Field::NewDisk,
            )))
            .collect(),
            Tab::Advanced if self.draft.model == crate::settings::Model::Ultra5 => vec![
                (
                    "Disk access",
                    if self.draft.disk_read_only {
                        "Read-only".into()
                    } else {
                        "Read/write".into()
                    },
                    Field::DiskAccess,
                ),
                (
                    "Console",
                    "Onboard Mach64 and Sun Type 5 keyboard".into(),
                    Field::Fixed,
                ),
            ],
            Tab::Network => vec![
                (
                    "Host networking",
                    on_off(self.draft.network),
                    Field::Network,
                ),
                (
                    "NAT guest",
                    "192.168.0.2 / 255.255.255.0".into(),
                    Field::Fixed,
                ),
                ("NAT gateway", "192.168.0.1".into(), Field::Fixed),
                (
                    "Guest setup",
                    if self.draft.model == crate::settings::Model::Ultra5 {
                        "DHCP or static hme0"
                    } else {
                        "DHCP or static le0"
                    }
                    .into(),
                    Field::Fixed,
                ),
            ],
            Tab::Advanced => {
                let mut rows = vec![(
                    "Disk access",
                    if self.draft.disk_read_only {
                        "Read-only".into()
                    } else {
                        "Read/write".into()
                    },
                    Field::DiskAccess,
                )];
                if self.draft.tinker {
                    rows.extend([
                        (
                            "Headless guest",
                            on_off(self.draft.headless),
                            Field::Headless,
                        ),
                        (
                            "Serial PTY symlink",
                            if self.draft.serial_pty_link.is_empty() {
                                "Automatic /dev/pts endpoint".into()
                            } else {
                                self.draft.serial_pty_link.clone()
                            },
                            Field::SerialPtyLink,
                        ),
                        (
                            "Open serial terminal",
                            "Launch minicom for the running guest".into(),
                            Field::OpenSerialTerminal,
                        ),
                        (
                            "Send serial BREAK",
                            "Interrupt the running guest console".into(),
                            Field::SerialBreak,
                        ),
                        (
                            "Serial console view",
                            on_off(self.draft.show_serial_console),
                            Field::SerialConsole,
                        ),
                        (
                            "Accurate CPU clocks",
                            if self.draft.accurate_cpu_clocks {
                                format!(
                                    "On / {} MHz",
                                    self.draft
                                        .model
                                        .profile()
                                        .map_or(440, |p| p.cpu_clock_hz() / 1_000_000)
                                )
                            } else {
                                "Off / faster boots".into()
                            },
                            Field::AccurateClocks,
                        ),
                        (
                            "RTC at startup",
                            if self.draft.rtc_host_time {
                                "Host UTC".into()
                            } else {
                                "Saved guest time".into()
                            },
                            Field::RtcHostTime,
                        ),
                    ]);
                }
                rows
            }
            Tab::About => vec![],
        }
        .into_iter()
        .filter(|(_, _, field)| {
            self.draft.model != crate::settings::Model::Ultra5
                || !matches!(field, Field::Rom | Field::NewDisk)
        })
        .map(|(label, value, field)| Row {
            label,
            value,
            field,
        })
        .collect()
    }
    pub fn field(&self) -> Field {
        self.rows()
            .get(self.selected)
            .map_or(Field::Fixed, |row| row.field)
    }
    pub fn move_row(&mut self, delta: isize) {
        let rows = self.rows().len();
        if rows != 0 {
            self.selected = (self.selected as isize + delta).rem_euclid(rows as isize) as usize;
        }
    }
    pub fn dirty(&self, live: &Settings) -> bool {
        self.draft != *live
    }
    pub fn switch_tab(&mut self, delta: isize) {
        if self.new_disk.is_some() {
            return;
        }
        let tabs = self.tabs();
        let index = tabs.iter().position(|t| *t == self.tab).unwrap_or(0);
        self.tab = tabs[(index as isize + delta).rem_euclid(tabs.len() as isize) as usize];
        self.selected = 0;
    }
    pub fn path(&self, field: Field) -> Option<&String> {
        match field {
            Field::NewDiskPath => self.new_disk.as_ref().map(|d| &d.path),
            Field::SerialPtyLink => Some(&self.draft.serial_pty_link),
            Field::Prom => Some(&self.draft.prom),
            Field::Rom => Some(&self.draft.cgsix_rom),
            Field::IdeDisk => Some(&self.draft.ide_disk),
            Field::Scsi(id) => Some(&self.draft.scsi[id].path),
            _ => None,
        }
    }
    pub fn path_mut(&mut self, field: Field) -> Option<&mut String> {
        match field {
            Field::NewDiskPath => self.new_disk.as_mut().map(|d| &mut d.path),
            Field::SerialPtyLink => Some(&mut self.draft.serial_pty_link),
            Field::Prom => Some(&mut self.draft.prom),
            Field::Rom => Some(&mut self.draft.cgsix_rom),
            Field::IdeDisk => Some(&mut self.draft.ide_disk),
            Field::Scsi(id) => Some(&mut self.draft.scsi[id].path),
            _ => None,
        }
    }
    pub fn activate(&mut self) {
        match self.field() {
            Field::NewDisk => {
                let target = std::iter::once(3).chain(0..7).find(|&id| {
                    self.draft.scsi[id].kind == TargetKind::Disconnected
                        || self.draft.scsi[id].path.is_empty()
                });
                if let Some(target) = target {
                    self.new_disk = Some(DiskDraft {
                        path: String::new(),
                        size_mib: 2048,
                        target,
                    });
                    self.selected = 0;
                } else {
                    self.notice =
                        Some("Detach a SCSI target in the draft before creating a disk.".into());
                }
            }
            Field::NewDiskSize => {
                self.edit = self.new_disk.as_ref().map(|d| d.size_mib.to_string())
            }
            Field::NewDiskTarget => {
                if let Some(disk) = &mut self.new_disk
                    && let Some(id) = (1..=7).map(|i| (disk.target + i) % 7).find(|&id| {
                        self.draft.scsi[id].kind == TargetKind::Disconnected
                            || self.draft.scsi[id].path.is_empty()
                    })
                {
                    disk.target = id;
                }
            }
            Field::CancelDisk => self.cancel_new_disk(),
            Field::MachineLabel => {
                if !self.draft.next_machine() {
                    self.selected = 1;
                    self.edit = Some(String::new());
                }
            }
            Field::SaveMachineAs => self.edit = Some(String::new()),
            Field::Model => {
                self.draft.model = self.draft.model.next();
                if self
                    .draft
                    .model
                    .profile()
                    .and_then(|p| p.processor_topology(usize::from(self.draft.cpus)))
                    .is_none()
                {
                    self.draft.cpus = 1;
                }
                if self.draft.model == crate::settings::Model::Ultra5 {
                    self.draft.display = Display::Mach64;
                    self.draft.headless = false;
                } else if !self.draft.model.supports_sx() || self.draft.display == Display::Mach64 {
                    self.draft.display = Display::Cgsix;
                }
                self.adjust_ram_after_hardware_change();
            }
            Field::CpuCount => self.draft.cpus = if self.draft.cpus == 1 { 2 } else { 1 },
            Field::Ram => {
                if let Ok(choices) = self.draft.valid_ram_mib()
                    && !choices.is_empty()
                {
                    let index = choices
                        .iter()
                        .position(|&mib| mib == self.draft.ram_mib)
                        .unwrap_or(0);
                    self.draft.ram_mib = choices[(index + 1) % choices.len()];
                }
            }
            Field::Display => {
                if self.draft.model.supports_sx() {
                    self.draft.display = self.draft.display.next();
                    self.adjust_ram_after_hardware_change();
                }
            }
            Field::Headless => {
                self.draft.headless ^= true;
                self.adjust_ram_after_hardware_change();
            }
            Field::SerialConsole => self.draft.show_serial_console ^= true,
            Field::AccurateClocks => self.draft.accurate_cpu_clocks ^= true,
            Field::RtcHostTime => self.draft.rtc_host_time ^= true,
            Field::DiskAccess => self.draft.disk_read_only ^= true,
            Field::Network => self.draft.network ^= true,
            Field::Smoothing => self.draft.smoothing ^= true,
            Field::Scanlines => self.draft.scanlines ^= true,
            Field::Status => self.draft.show_status ^= true,
            Field::Tinker => self.draft.tinker ^= true,
            field => {
                self.edit = self.path_mut(field).cloned();
            }
        }
    }
    fn adjust_ram_after_hardware_change(&mut self) {
        let old = self.draft.ram_mib;
        let Ok(choices) = self.draft.valid_ram_mib() else {
            return;
        };
        if choices.is_empty() || choices.contains(&old) {
            return;
        }
        let replacement = choices
            .iter()
            .rev()
            .find(|&&mib| mib <= old)
            .copied()
            .or_else(|| choices.first().copied());
        if let Some(replacement) = replacement {
            self.draft.ram_mib = replacement;
            self.notice = Some(format!(
                "Memory adjusted from {old} to {replacement} MiB for {} / {}.",
                self.draft.model.short_label(),
                self.draft.display.label()
            ));
        }
    }

    pub fn finish_edit(&mut self) -> Result<(), String> {
        if let Some(value) = self.edit.as_ref() {
            if self.field() == Field::NewDiskSize {
                if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                    return Err("Capacity must be a positive number of MiB.".into());
                }
                let mib: u64 = value.parse().map_err(|_| "Invalid disk capacity.")?;
                if !(1..=2_097_152).contains(&mib) {
                    return Err("Capacity must be 1..2097152 MiB (SCSI limit: 2 TiB).".into());
                }
                self.new_disk.as_mut().unwrap().size_mib = mib;
            } else if self.field() == Field::SaveMachineAs {
                self.draft.save_as_machine(value)?;
            } else {
                self.set_path(self.field(), value.clone());
            }
            self.edit = None;
        }
        Ok(())
    }
    pub fn set_path(&mut self, field: Field, value: String) {
        if let Field::Scsi(id) = field
            && !value.is_empty()
            && (self.draft.scsi[id].kind == TargetKind::Disconnected
                || (id == 6 && self.draft.model == crate::settings::Model::Ultra5))
        {
            self.draft.scsi[id].kind = if id == 6 {
                TargetKind::Cdrom
            } else {
                TargetKind::Disk
            };
        }
        if let Some(path) = self.path_mut(field) {
            *path = value;
        }
    }
    pub fn cancel_new_disk(&mut self) {
        self.new_disk = None;
        self.edit = None;
        self.selected = 7;
    }
    pub fn disk_created(&mut self, target: usize, path: String) {
        self.draft.scsi[target] = crate::settings::ScsiTarget {
            kind: TargetKind::Disk,
            path,
        };
        self.new_disk = None;
        self.edit = None;
        self.selected = target;
    }
    pub fn cycle_target(&mut self) {
        if let Field::Scsi(id) = self.field() {
            self.draft.scsi[id].kind = if self.draft.model == crate::settings::Model::Ultra5 {
                if self.draft.scsi[id].kind == TargetKind::Cdrom {
                    TargetKind::Disconnected
                } else {
                    TargetKind::Cdrom
                }
            } else {
                self.draft.scsi[id].kind.next()
            };
        }
    }
    pub fn detach(&mut self) {
        if let Field::Scsi(id) = self.field() {
            self.draft.scsi[id].kind = TargetKind::Disconnected;
        }
        if let Some(path) = self.path_mut(self.field()) {
            path.clear();
        }
    }
}
fn path_label(value: &str) -> String {
    if value.is_empty() {
        "Not selected".into()
    } else {
        value.into()
    }
}
fn on_off(value: bool) -> String {
    if value { "On" } else { "Off" }.into()
}

#[derive(Clone, Copy)]
pub enum Confirm {
    Quit,
    Reset,
    Unsaved,
}
impl Confirm {
    pub fn text(self) -> &'static str {
        match self {
            Self::Quit => "Quit Heliacal? NVRAM will be saved.",
            Self::Reset => "Cold reset with saved settings? Guest RAM will be lost.",
            Self::Unsaved => "Save changes before closing?",
        }
    }
    pub fn choices(self) -> &'static [&'static str] {
        match self {
            Self::Quit | Self::Reset => &["Yes", "Cancel"],
            Self::Unsaved => &["Save", "Discard", "Cancel"],
        }
    }
    pub fn default_choice(self) -> usize {
        match self {
            Self::Unsaved => 2,
            Self::Quit | Self::Reset => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_selection_is_a_draft_and_model_changes_enforce_topology() {
        use crate::settings::Model;
        let saved = Settings {
            model: Model::Sparcstation20,
            ..Settings::default()
        };
        let mut overlay = Overlay::new(&saved);
        overlay.selected = overlay
            .rows()
            .iter()
            .position(|r| r.field == Field::CpuCount)
            .unwrap();
        overlay.activate();
        assert_eq!(overlay.draft.cpus, 2);
        assert_eq!(saved.cpus, 1);
        overlay.activate();
        assert_eq!(overlay.draft.cpus, 1);
        overlay.activate();
        overlay.selected = overlay
            .rows()
            .iter()
            .position(|r| r.field == Field::Model)
            .unwrap();
        overlay.activate(); // SS20 -> Axil retains two single-CPU modules.
        assert_eq!(overlay.draft.cpus, 2);
        overlay.activate(); // Axil -> SS5 has only one integrated CPU.
        assert_eq!(overlay.draft.cpus, 1);
        assert!(!overlay.rows().iter().any(|r| r.field == Field::CpuCount));
        overlay.draft.validate().unwrap();
    }

    #[test]
    fn ram_row_cycles_core_choices_without_touching_live_settings() {
        let settings = Settings::default();
        let mut overlay = Overlay::new(&settings);
        let memory = overlay
            .rows()
            .into_iter()
            .find(|row| row.field == Field::Ram)
            .expect("RAM row");
        assert_eq!(memory.value, "128 MiB");
        overlay.selected = overlay
            .rows()
            .iter()
            .position(|row| row.field == Field::Ram)
            .unwrap();
        let choices = overlay.draft.valid_ram_mib().unwrap();
        let index = choices.iter().position(|&mib| mib == 128).unwrap();
        overlay.activate();
        assert_eq!(overlay.draft.ram_mib, choices[(index + 1) % choices.len()]);
        assert_eq!(settings.ram_mib, 128);
        assert!(overlay.dirty(&settings));
    }

    #[test]
    fn hardware_changes_keep_valid_ram_or_adjust_with_a_visible_notice() {
        let saved = Settings {
            model: crate::settings::Model::Sparcstation20,
            display: Display::Cgsix,
            ram_mib: 512,
            ..Settings::default()
        };
        let mut overlay = Overlay::new(&saved);
        overlay.selected = overlay
            .rows()
            .iter()
            .position(|row| row.field == Field::Model)
            .unwrap();
        overlay.activate(); // SS20 -> Axil: 512 remains valid.
        assert_eq!(overlay.draft.ram_mib, 512);
        assert!(overlay.notice.is_none());
        overlay.activate(); // Axil -> Ultra 5: the fixed memory size is 512.
        assert_eq!(overlay.draft.model, crate::settings::Model::Ultra5);
        assert_eq!(overlay.draft.ram_mib, 512);
        overlay.activate(); // Ultra 5 -> SS-5.
        assert_eq!(overlay.draft.ram_mib, 256);
        assert!(overlay.notice.as_deref().unwrap().contains("512 to 256"));
        assert_eq!(saved.ram_mib, 512);

        let mut overlay = Overlay::new(&saved);
        overlay.selected = overlay
            .rows()
            .iter()
            .position(|row| row.field == Field::Display)
            .unwrap();
        overlay.activate(); // CGsix 512 -> SX 448.
        assert_eq!(overlay.draft.display, Display::Sx);
        assert_eq!(overlay.draft.ram_mib, 448);
        assert!(overlay.notice.as_deref().unwrap().contains("512 to 448"));

        let invalid = Settings {
            ram_mib: 4,
            ..Settings::default()
        };
        let mut overlay = Overlay::new(&invalid);
        overlay.selected = overlay
            .rows()
            .iter()
            .position(|row| row.field == Field::Model)
            .unwrap();
        overlay.activate(); // SS5 -> SS20: minimum becomes 16 MiB.
        assert_eq!(overlay.draft.ram_mib, 16);
        assert!(overlay.notice.as_deref().unwrap().contains("4 to 16"));
    }

    #[test]
    fn named_machine_menu_keeps_selection_and_name_errors_in_the_draft() {
        let mut settings = Settings::default();
        settings.save_as_machine("Original").unwrap();
        let mut overlay = Overlay::new(&settings);
        overlay.selected = 1;
        overlay.activate();
        overlay.edit = Some("Original".into());
        assert!(overlay.finish_edit().is_err());
        assert_eq!(overlay.edit.as_deref(), Some("Original"));
        assert_eq!(overlay.draft, settings);
        overlay.edit = Some("Solaris 2.6".into());
        overlay.finish_edit().unwrap();
        assert_eq!(overlay.draft.machine_label.as_deref(), Some("Solaris 2.6"));
        assert_eq!(settings.machine_label.as_deref(), Some("Original"));
        overlay.draft.network = false;
        overlay.selected = 0;
        overlay.activate();
        assert_eq!(overlay.draft.machine_label.as_deref(), Some("Original"));
        assert!(overlay.draft.network);
        overlay.activate();
        assert_eq!(overlay.draft.machine_label.as_deref(), Some("Solaris 2.6"));
        assert!(!overlay.draft.network);
        drop(overlay); // Discard does not change the live catalog.
        assert_eq!(settings.machines.len(), 1);
    }

    #[test]
    fn edits_are_drafts_and_cancel_does_not_apply() {
        let settings = Settings::default();
        let mut overlay = Overlay::new(&settings);
        overlay.selected = overlay
            .rows()
            .iter()
            .position(|row| row.field == Field::Prom)
            .unwrap();
        overlay.activate();
        overlay.edit = Some("discarded.bin".into());
        overlay.edit = None;
        assert_eq!(overlay.draft, settings);
        overlay.activate();
        overlay.edit = Some("accepted.bin".into());
        overlay.finish_edit().unwrap();
        assert_eq!(overlay.draft.prom, "accepted.bin");
        assert!(settings.prom.is_empty());
        assert!(overlay.tabs().contains(&Tab::Advanced));
        overlay.draft.tinker = true;
        assert!(overlay.tabs().contains(&Tab::Advanced));
    }
    #[test]
    fn machine_row_cycles_the_draft_model_without_touching_live_settings() {
        let settings = Settings::default();
        let mut o = Overlay::new(&settings);
        o.selected = o
            .rows()
            .iter()
            .position(|row| row.field == Field::Model)
            .unwrap();
        assert_eq!(o.field(), Field::Model);
        o.activate();
        assert_eq!(o.draft.model, crate::settings::Model::Sparcstation20);
        assert_eq!(settings.model, crate::settings::Model::Sparcstation5);
        assert!(o.dirty(&settings));
        o.activate();
        assert_eq!(o.draft.model, crate::settings::Model::Axil320);
        o.activate();
        assert_eq!(o.draft.model, crate::settings::Model::Ultra5);
        assert_eq!(o.draft.ram_mib, 512);
        o.activate();
        assert_eq!(o.draft.model, crate::settings::Model::Sparcstation5);
        assert!(o.draft.validate_hardware().is_ok());
        o.selected = o
            .rows()
            .iter()
            .position(|row| row.field == Field::Display)
            .unwrap();
        assert_eq!(o.field(), Field::Display);
        o.activate();
        assert_eq!(o.draft.display, Display::Cgsix);
    }
    #[test]
    fn tabs_have_no_save_rows_and_empty_tabs_navigate_safely() {
        let settings = Settings::default();
        let mut overlay = Overlay::new(&settings);
        for tab in overlay.tabs() {
            overlay.tab = tab;
            assert!(
                overlay
                    .rows()
                    .iter()
                    .all(|row| row.label != "Save settings" && row.label != "Discard changes")
            );
        }
        overlay.tab = Tab::About;
        overlay.selected = 0;
        assert_eq!(overlay.field(), Field::Fixed);
        overlay.move_row(1);
        assert!(overlay.dirty(&Settings {
            smoothing: true,
            ..Settings::default()
        }));
        assert_eq!(Confirm::Unsaved.default_choice(), 2);
        assert_eq!(Confirm::Unsaved.choices(), ["Save", "Discard", "Cancel"]);
        assert_eq!(Confirm::Quit.default_choice(), 1);
    }
    #[test]
    fn network_tab_toggles_the_draft_and_requires_a_cold_reset() {
        for (model, interface) in [
            (crate::settings::Model::Sparcstation5, "le0"),
            (crate::settings::Model::Ultra5, "hme0"),
        ] {
            let settings = Settings {
                model,
                ..Settings::default()
            };
            let mut o = Overlay::new(&settings);
            o.tab = Tab::Network;
            o.selected = 0;
            assert_eq!(o.field(), Field::Network);
            assert!(o.draft.network);
            assert!(
                o.rows()
                    .iter()
                    .any(|r| r.label == "Guest setup" && r.value.ends_with(interface))
            );
            o.activate();
            assert!(!o.draft.network);
            assert!(o.dirty(&settings));
            assert!(!settings.hardware_eq(&o.draft));
        }
    }

    #[test]
    fn rtc_startup_is_tinker_gated_and_transactional() {
        let settings = Settings::default();
        let mut o = Overlay::new(&settings);
        o.tab = Tab::Advanced;
        assert!(!o.rows().iter().any(|r| r.field == Field::RtcHostTime));
        let settings = Settings {
            tinker: true,
            ..settings
        };
        let mut o = Overlay::new(&settings);
        o.tab = Tab::Advanced;
        o.selected = o
            .rows()
            .iter()
            .position(|r| r.field == Field::RtcHostTime)
            .unwrap();
        assert_eq!(o.rows()[o.selected].value, "Host UTC");
        o.activate();
        assert_eq!(o.rows()[o.selected].value, "Saved guest time");
        assert!(settings.rtc_host_time);
        assert!(!o.draft.rtc_host_time);
        assert!(o.dirty(&settings));
        assert!(!o.draft.hardware_eq(&settings));
        assert!(!o.draft.accurate_cpu_clocks);
        o.activate();
        assert!(!o.dirty(&settings));
    }

    #[test]
    fn accurate_clocks_are_tinker_gated_and_only_change_the_hardware_draft() {
        let mut settings = Settings::default();
        let mut o = Overlay::new(&settings);
        o.tab = Tab::Advanced;
        assert!(!o.rows().iter().any(|r| r.field == Field::AccurateClocks));
        settings.tinker = true;
        let mut o = Overlay::new(&settings);
        o.tab = Tab::Advanced;
        o.selected = o
            .rows()
            .iter()
            .position(|r| r.field == Field::AccurateClocks)
            .unwrap();
        assert!(!o.draft.accurate_cpu_clocks);
        o.activate();
        assert!(o.draft.accurate_cpu_clocks);
        assert!(!settings.accurate_cpu_clocks);
        assert!(o.dirty(&settings));
        assert!(!o.draft.hardware_eq(&settings));
        o.activate();
        assert!(!o.dirty(&settings));
    }

    #[test]
    fn advanced_disk_access_toggles_the_draft_and_is_dirty() {
        let settings = Settings::default();
        let mut o = Overlay::new(&settings);
        o.tab = Tab::Advanced;
        o.selected = 0;
        assert_eq!(o.field(), Field::DiskAccess);
        assert!(!o.draft.disk_read_only);
        o.activate();
        assert!(o.draft.disk_read_only);
        assert!(o.dirty(&settings));
        o.activate();
        assert!(!o.draft.disk_read_only);
        assert!(!o.dirty(&settings));
    }
    #[test]
    fn scsi_type_image_and_detach_only_change_the_selected_draft_target() {
        let settings = Settings::default();
        let mut o = Overlay::new(&settings);
        o.tab = Tab::Media;
        o.selected = 1;
        o.cycle_target();
        assert_eq!(o.draft.scsi[1].kind, TargetKind::Disk);
        o.cycle_target();
        o.set_path(Field::Scsi(1), "solaris.iso".into());
        assert_eq!(o.draft.scsi[1].kind, TargetKind::Cdrom);
        assert!(settings.scsi[1].path.is_empty());
        assert_eq!(o.draft.scsi[3], settings.scsi[3]);
        o.cycle_target();
        assert_eq!(o.draft.scsi[1].kind, TargetKind::Disconnected);
        assert_eq!(o.draft.scsi[1].path, "solaris.iso");
        o.detach();
        assert!(o.draft.scsi[1].path.is_empty());
        assert_eq!(
            o.rows()
                .iter()
                .filter(|row| matches!(row.field, Field::Scsi(_)))
                .count(),
            7
        );
    }
}
