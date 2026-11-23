use crate::settings::{Model, SCSI_TARGETS, Settings, TargetKind};
use helios::media::MediaKind;
use std::time::{Duration, Instant};

const CPU: u64 = 1;
const ETHERNET: u64 = 1 << (1 + SCSI_TARGETS * 2);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageLamp {
    pub label: &'static str,
    pub media: Option<MediaKind>,
}

pub fn storage_lamps(settings: &Settings) -> [Option<StorageLamp>; SCSI_TARGETS] {
    const SCSI: [&str; SCSI_TARGETS] = [
        "SCSI0", "SCSI1", "SCSI2", "SCSI3", "SCSI4", "SCSI5", "SCSI6",
    ];
    const IDE: [&str; 4] = ["IDE0M", "IDE0S", "IDE1M", "IDE1S"];
    std::array::from_fn(|id| {
        if settings.model == Model::Ultra5 {
            let label = *IDE.get(id)?;
            let media = match id {
                0 if !settings.ide_disk.is_empty() => Some(MediaKind::Disk),
                2 if settings.scsi[6].kind == TargetKind::Cdrom
                    && !settings.scsi[6].path.is_empty() =>
                {
                    Some(MediaKind::Cdrom)
                }
                _ => None,
            };
            Some(StorageLamp { label, media })
        } else {
            let target = &settings.scsi[id];
            let media = target.kind.media_kind()?;
            Some(StorageLamp {
                label: SCSI[id],
                media: Some(media),
            })
        }
    })
}

pub fn disk_read(id: usize) -> u64 {
    1 << (1 + id * 2)
}

pub fn disk_write(id: usize) -> u64 {
    disk_read(id) << 1
}

#[derive(Default)]
pub struct Leds {
    until: [Option<Instant>; 2 + SCSI_TARGETS * 2],
    pub lit: u64,
}

impl Leds {
    pub fn update(&mut self, events: u64, now: Instant) {
        self.lit = 0;
        for (bit, until) in self.until.iter_mut().enumerate() {
            if events & (1 << bit) != 0 {
                *until = Some(now + Duration::from_millis(150));
            }
            if until.is_some_and(|deadline| now < deadline) {
                self.lit |= 1 << bit;
            }
        }
    }
}

pub fn cpu() -> u64 {
    CPU
}

pub fn ethernet() -> u64 {
    ETHERNET
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_follow_the_bus_and_secondary_master_cdrom() {
        let mut settings = Settings::default();
        for (id, target) in settings.scsi.iter_mut().enumerate() {
            target.kind = if id == 6 {
                TargetKind::Cdrom
            } else {
                TargetKind::Disk
            };
        }
        let scsi = storage_lamps(&settings);
        for (id, lamp) in scsi.iter().enumerate() {
            assert_eq!(lamp.unwrap().label, format!("SCSI{id}"));
        }
        settings.scsi[3].kind = TargetKind::Disconnected;
        assert!(storage_lamps(&settings)[3].is_none());
        settings.model = Model::Ultra5;
        settings.ide_disk = "solaris-ide0.img".into();
        settings.scsi[6].path = "install.iso".into();
        let ide = storage_lamps(&settings);
        let visible: Vec<_> = ide
            .into_iter()
            .flatten()
            .map(|lamp| (lamp.label, lamp.media))
            .collect();
        assert_eq!(
            visible,
            [
                ("IDE0M", Some(MediaKind::Disk)),
                ("IDE0S", None),
                ("IDE1M", Some(MediaKind::Cdrom)),
                ("IDE1S", None)
            ]
        );
        settings.scsi[6].path.clear();
        assert_eq!(storage_lamps(&settings)[2].unwrap().media, None);
    }

    #[test]
    fn pulses_expire_and_cpu_and_disk_bits_do_not_overlap() {
        let now = Instant::now();
        let mut leds = Leds::default();
        let events =
            cpu() | ethernet() | disk_read(0) | disk_write(0) | disk_read(6) | disk_write(6);
        leds.update(events, now);
        assert_eq!(leds.lit, events);
        leds.update(0, now + Duration::from_millis(151));
        assert_eq!(leds.lit, 0);
        assert_eq!(events.count_ones(), 6);
    }
}
