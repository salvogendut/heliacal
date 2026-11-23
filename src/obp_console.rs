//! Console selection in initialized NVRAM for the three pinned original PROMs.
//! These offsets and XOR deltas were verified against OBP `setenv` output;
//! see docs/serial-console.md. Never manufacture an initialized firmware image.
use crate::settings::Model;
use helios::devices::nvram::Nvram;

pub fn select(model: Model, prom: &[u8], nvram: Nvram, headless: bool) -> Result<Nvram, String> {
    let expected = match model {
        Model::Sparcstation5 => (262144, 0xf438e2ee, 0x769),
        Model::Sparcstation20 => (524288, 0x105ba132, 0x769),
        Model::Ultra5 => return Err("Ultra 5 uses its onboard keyboard and Mach64".into()),
        Model::Axil320 => (524288, 0x65290ef7, 0x781),
    };
    if prom.len() != expected.0 || crc32fast::hash(prom) != expected.1 {
        if headless {
            eprintln!(
                "Unrecognized PROM: automatic console selection unavailable; configure input-device/output-device as ttya in this firmware."
            );
        }
        return Ok(nvram);
    }
    let mut bytes = nvram.image();
    let fields = [expected.2, expected.2 + 34];
    if !fields
        .iter()
        .all(|&start| valid_string(&bytes[start..start + 34]))
    {
        if headless {
            return Err("Headless boot needs initialized OBP NVRAM. Boot graphically first, then enable Headless in F9 > Advanced. Existing NVRAM has been preserved.".into());
        }
        return Ok(nvram);
    }
    for (start, value) in fields.into_iter().zip(if headless {
        [b"ttya".as_slice(), b"ttya".as_slice()]
    } else {
        [b"keyboard".as_slice(), b"screen".as_slice()]
    }) {
        set_string(&mut bytes, start, value);
    }
    Nvram::from_image(&bytes).map_err(|e| e.to_string())
}

fn valid_string(field: &[u8]) -> bool {
    let len = usize::from(field[0]);
    (1..=32).contains(&len)
        && field[len + 1] == 0
        && field[1..=len].iter().all(u8::is_ascii_graphic)
}

fn set_string(bytes: &mut [u8], start: usize, value: &[u8]) {
    // Preserve unused tail bytes just as OBP setenv does, and adjust only the
    // existing checksum. This does not validate/repair an already corrupt image.
    for (i, new) in std::iter::once(value.len() as u8)
        .chain(value.iter().copied())
        .chain(std::iter::once(0))
        .enumerate()
    {
        bytes[0] ^= bytes[start + i] ^ new;
        bytes[start + i] = new;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn captured_setenv_delta_preserves_tail_identity_and_other_settings() {
        for start in [0x769, 0x781] {
            let mut image = [0x55; 8192];
            image[0] = 0xc4;
            image[start..start + 10].copy_from_slice(b"\x08keyboard\0");
            image[start + 34..start + 42].copy_from_slice(b"\x06screen\0");
            let before = image;
            set_string(&mut image, start, b"ttya");
            set_string(&mut image, start + 34, b"ttya");
            assert_eq!(image[0], 0xd2); // Measured original SS-20 OBP setenv pair.
            assert_eq!(&image[start..start + 10], b"\x04ttya\0ard\0");
            for i in 1..image.len() {
                if !(start..start + 10).contains(&i) && !(start + 34..start + 42).contains(&i) {
                    assert_eq!(image[i], before[i]);
                }
            }
            set_string(&mut image, start, b"keyboard");
            set_string(&mut image, start + 34, b"screen");
            assert_eq!(image, before);
        }
    }
    #[test]
    fn refuses_empty_and_malformed_fields() {
        assert!(!valid_string(&[0; 34]));
        let mut field = [0; 34];
        field[0] = 255;
        assert!(!valid_string(&field));
        field[0] = 4;
        field[1..5].copy_from_slice(b"ttya");
        assert!(valid_string(&field));
        field[5] = 1;
        assert!(!valid_string(&field));
    }
}
