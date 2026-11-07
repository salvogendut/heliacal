use crate::settings::{Settings, TargetKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    General,
    Display,
    Media,
    Advanced,
    About,
}
impl Tab {
    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Display => "Display",
            Self::Media => "Media",
            Self::Advanced => "Advanced",
            Self::About => "About",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Fixed,
    Prom,
    Rom,
    Scsi(usize),
    Smoothing,
    Scanlines,
    Status,
    Tinker,
}
impl Field {
    pub fn is_path(self) -> bool {
        matches!(self, Self::Prom | Self::Rom | Self::Scsi(_))
    }
}
pub struct Row {
    pub label: &'static str,
    pub value: String,
    pub field: Field,
}
pub struct Overlay {
    pub draft: Settings,
    pub tab: Tab,
    pub selected: usize,
    pub edit: Option<String>,
}
impl Overlay {
    pub fn new(settings: &Settings) -> Self {
        Self {
            draft: settings.clone(),
            tab: Tab::General,
            selected: 4,
            edit: None,
        }
    }
    pub fn tabs(&self) -> Vec<Tab> {
        let mut tabs = vec![Tab::General, Tab::Display, Tab::Media];
        if self.draft.tinker {
            tabs.push(Tab::Advanced);
        }
        tabs.push(Tab::About);
        tabs
    }
    pub fn rows(&self) -> Vec<Row> {
        match self.tab {
            Tab::General => vec![
                ("Machine", "Sun SPARCstation 5".into(), Field::Fixed),
                ("Processor", "microSPARC-II / 85 MHz".into(), Field::Fixed),
                ("Memory", "128 MiB".into(), Field::Fixed),
                (
                    "Graphics",
                    "TurboGX / CGsix / 1152 x 900".into(),
                    Field::Fixed,
                ),
                ("Sun SS-5 PROM", path_label(&self.draft.prom), Field::Prom),
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
            .collect(),
            Tab::Advanced | Tab::About => vec![],
        }
        .into_iter()
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
        let tabs = self.tabs();
        let index = tabs.iter().position(|t| *t == self.tab).unwrap_or(0);
        self.tab = tabs[(index as isize + delta).rem_euclid(tabs.len() as isize) as usize];
        self.selected = 0;
    }
    pub fn path(&self, field: Field) -> Option<&String> {
        match field {
            Field::Prom => Some(&self.draft.prom),
            Field::Rom => Some(&self.draft.cgsix_rom),
            Field::Scsi(id) => Some(&self.draft.scsi[id].path),
            _ => None,
        }
    }
    pub fn path_mut(&mut self, field: Field) -> Option<&mut String> {
        match field {
            Field::Prom => Some(&mut self.draft.prom),
            Field::Rom => Some(&mut self.draft.cgsix_rom),
            Field::Scsi(id) => Some(&mut self.draft.scsi[id].path),
            _ => None,
        }
    }
    pub fn activate(&mut self) {
        match self.field() {
            Field::Smoothing => self.draft.smoothing ^= true,
            Field::Scanlines => self.draft.scanlines ^= true,
            Field::Status => self.draft.show_status ^= true,
            Field::Tinker => self.draft.tinker ^= true,
            field => {
                self.edit = self.path_mut(field).cloned();
            }
        }
    }
    pub fn finish_edit(&mut self) {
        if let Some(value) = self.edit.take() {
            self.set_path(self.field(), value);
        }
    }
    pub fn set_path(&mut self, field: Field, value: String) {
        if let Field::Scsi(id) = field
            && !value.is_empty()
            && self.draft.scsi[id].kind == TargetKind::Disconnected
        {
            self.draft.scsi[id].kind = TargetKind::Disk;
        }
        if let Some(path) = self.path_mut(field) {
            *path = value;
        }
    }
    pub fn cycle_target(&mut self) {
        if let Field::Scsi(id) = self.field() {
            self.draft.scsi[id].kind = self.draft.scsi[id].kind.next();
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
    fn edits_are_drafts_and_cancel_does_not_apply() {
        let settings = Settings::default();
        let mut overlay = Overlay::new(&settings);
        overlay.activate();
        overlay.edit = Some("discarded.bin".into());
        overlay.edit = None;
        assert_eq!(overlay.draft, settings);
        overlay.activate();
        overlay.edit = Some("accepted.bin".into());
        overlay.finish_edit();
        assert_eq!(overlay.draft.prom, "accepted.bin");
        assert!(settings.prom.is_empty());
        assert!(!overlay.tabs().contains(&Tab::Advanced));
        overlay.draft.tinker = true;
        assert!(overlay.tabs().contains(&Tab::Advanced));
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
