use std::time::{Duration, Instant};

const CPU: u64 = 1;
const SCSI_IDS: usize = 7;
const ETHERNET: u64 = 1 << (1 + SCSI_IDS * 2);

pub fn disk_read(id: usize) -> u64 {
    1 << (1 + id * 2)
}

pub fn disk_write(id: usize) -> u64 {
    disk_read(id) << 1
}

#[derive(Default)]
pub struct Leds {
    until: [Option<Instant>; 2 + SCSI_IDS * 2],
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
