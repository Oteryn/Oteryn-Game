//! OTClient wheelclass.lua onImportConfig / getExportCode. Shared gems use
//! one-byte reference identities, which the Oteryn Atelier does not supply.
use oteryn_client_ui::wheel_allocation::{WheelPanel, diagram};

const ORDER: [usize; 36] = [
    15, 14, 9, 13, 8, 3, 7, 2, 1, 16, 17, 10, 18, 11, 4, 12, 5, 6, 22, 23, 28, 24, 29, 34, 30, 35,
    36, 21, 20, 27, 19, 26, 33, 32, 25, 31,
];
const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn metadata(panel: &WheelPanel) -> Result<(&'static str, u32), &'static str> {
    let reply = diagram::metadata(panel).ok_or("Refresh the Wheel first")?;
    let prefix = match reply
        .vocation
        .to_ascii_lowercase()
        .split_whitespace()
        .last()
    {
        Some("knight") => "K0",
        Some("paladin") => "P0",
        Some("sorcerer") => "S0",
        Some("druid") => "D0",
        Some("monk") => "M0",
        _ => return Err("Unknown Wheel vocation"),
    };
    Ok((prefix, reply.available))
}

pub(super) fn export(panel: &WheelPanel) -> Result<String, &'static str> {
    let (prefix, budget) = metadata(panel)?;
    let mut bytes = [255; 42];
    bytes[..2].copy_from_slice(
        &u16::try_from(budget)
            .map_err(|_| "Budget exceeds Tibia code limit")?
            .to_le_bytes(),
    );
    for (out, slot) in bytes[2..38].iter_mut().zip(ORDER) {
        *out =
            u8::try_from(panel.points[slot - 1]).map_err(|_| "Slice exceeds Tibia code limit")?;
    }
    if panel.points.iter().copied().map(u32::from).sum::<u32>() > budget {
        return Err("Draft exceeds available points");
    }
    let mut result = String::from(prefix);
    for chunk in bytes.as_chunks::<3>().0 {
        let bits = u32::from_be_bytes([0, chunk[0], chunk[1], chunk[2]]);
        for shift in [18, 12, 6, 0] {
            result.push(char::from(ALPHABET[((bits >> shift) & 63) as usize]));
        }
    }
    Ok(result)
}

pub(super) fn import(panel: &mut WheelPanel, code: &str) -> Result<(), &'static str> {
    let (prefix, budget) = metadata(panel)?;
    let body = code
        .trim()
        .strip_prefix(prefix)
        .ok_or("Code does not match this vocation")?;
    let stripped = body.trim_end_matches('=');
    if body.len() > 56
        || stripped.len() > 56
        || stripped.len() % 4 == 1
        || (stripped.len() != body.len() && body.len() != stripped.len().next_multiple_of(4))
    {
        return Err("Invalid Wheel code length");
    }
    let mut decoded = Vec::with_capacity(42);
    let mut bits = 0u16;
    let mut count = 0;
    for byte in stripped.bytes() {
        let normalized = match byte {
            b'-' => b'+',
            b'_' => b'/',
            b => b,
        };
        let value = ALPHABET
            .iter()
            .position(|b| *b == normalized)
            .ok_or("Invalid Wheel code character")?;
        bits = (bits << 6) | value as u16;
        count += 6;
        if count >= 8 {
            count -= 8;
            decoded.push((bits >> count) as u8);
            bits &= (1 << count) - 1;
        }
    }
    if bits != 0 || decoded.len() < 2 {
        return Err("Invalid Wheel code");
    }
    if decoded
        .get(38..)
        .is_some_and(|gems| gems.iter().any(|g| *g != 255))
    {
        return Err("This build contains Tibia gem identities unavailable on this server");
    }
    let maximum = u32::from(u16::from_le_bytes([decoded[0], decoded[1]]));
    let allocations = &decoded[2..decoded.len().min(38)];
    if allocations.iter().copied().map(u32::from).sum::<u32>() > maximum {
        return Err("Invalid Wheel point budget");
    }
    let mut points = [0; 36];
    let mut remaining = budget;
    for (value, slot) in allocations.iter().zip(ORDER) {
        if u16::from(*value) > oteryn_client_ui::wheel_allocation::SLOT_CAPACITIES[slot - 1] {
            return Err(tr!("Slot points exceed capacity"));
        }
        let value = u32::from(*value).min(remaining);
        remaining -= value;
        points[slot - 1] = value as u16;
    }
    let mut draft = panel.clone();
    draft.points = points;
    panel.import_build(&draft.build_code())
}
