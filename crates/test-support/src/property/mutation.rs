//! Bounded mutations of valid byte encodings.

use super::Rng;

/// Applies one byte mutation, growing the input by at most 64 bytes.
/// Mutations requiring bytes may leave an empty or short input unchanged.
#[must_use]
pub fn mutate(rng: &mut Rng, input: &[u8]) -> Vec<u8> {
    let mut output = input.to_vec();
    match rng.below(8) {
        0 if !output.is_empty() => {
            let index = rng.below(output.len() as u64) as usize;
            output[index] ^= 1 << rng.below(8);
        }
        1 if !output.is_empty() => {
            let index = rng.below(output.len() as u64) as usize;
            output[index] = *rng.pick(&[0x00, 0xff, 0x7f, 0x80]);
        }
        2 => {
            let len = rng.below(output.len() as u64 + 1) as usize;
            output.truncate(len);
        }
        3 => {
            let len = rng.below(64) as usize + 1;
            output.extend(rng.bytes(len));
        }
        4 => {
            let index = rng.below(output.len() as u64 + 1) as usize;
            output.insert(index, rng.u64() as u8);
        }
        5 if !output.is_empty() => {
            let index = rng.below(output.len() as u64) as usize;
            output.remove(index);
        }
        6 if !output.is_empty() => {
            let start = rng.below(output.len() as u64) as usize;
            let len = rng.below((output.len() - start).min(64) as u64) as usize + 1;
            output.extend_from_within(start..start + len);
        }
        7 => {
            let width = *rng.pick(&[2, 4, 8]);
            if output.len() >= width {
                let start = rng.below((output.len() - width + 1) as u64) as usize;
                let value: u64 = if rng.bool() { u64::MAX } else { 0 };
                // MAX and zero have identical bytes in either endian, at all widths.
                let bytes = if rng.bool() {
                    value.to_le_bytes()
                } else {
                    value.to_be_bytes()
                };
                output[start..start + width].copy_from_slice(&bytes[..width]);
            }
        }
        _ => {}
    }
    output
}
