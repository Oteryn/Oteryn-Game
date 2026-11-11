//! DromeResult field13: server-owned bounded tile marks and seeds.
use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DromeHazardKind {
    Lava = 5,
    Beam = 6,
    Tanked = 7,
    Sorrow = 8,
    Roots = 9,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DromeHazard {
    pub kind: DromeHazardKind,
    pub expires_at_micros: u64,
    pub positions: Vec<crate::world_spatial::ActorPosition>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DromeHazards {
    /// Selected modifier IDs; [0, 0] preserves the legacy absent projection.
    pub selected: [u8; 2],
    pub observed_at_micros: u64,
    pub entries: Vec<DromeHazard>,
}
pub const MAX_DROME_RESULT_BYTES: usize = 8192;

pub fn append_drome_hazards(
    mut result: Vec<u8>,
    hazards: &DromeHazards,
) -> Result<Vec<u8>, ArenaWireError> {
    let (_, existing) = split(&result)?;
    if existing.is_some() {
        return Err(ArenaWireError::Malformed);
    }
    DromeResult::decode(&result)?;
    validate(hazards)?;
    if hazards.entries.is_empty() && hazards.selected == [0, 0] {
        return Ok(result);
    }
    let mut body = Vec::new();
    push_varint_field(&mut body, 1, hazards.observed_at_micros);
    for entry in &hazards.entries {
        let mut row = Vec::new();
        push_varint_field(&mut row, 1, entry.kind as u64);
        push_varint_field(&mut row, 2, entry.expires_at_micros);
        for position in &entry.positions {
            let mut at = Vec::new();
            push_varint_field(&mut at, 1, position.x as u64);
            push_varint_field(&mut at, 2, position.y as u64);
            // Native floors -15..15 are represented by the explicit +15 bias.
            push_varint_field(&mut at, 3, (i32::from(position.floor) + 15) as u64);
            push_message_field(&mut row, 3, &at);
        }
        push_message_field(&mut body, 2, &row);
    }
    if hazards.selected != [0, 0] {
        for selected in hazards.selected {
            push_varint_field(&mut body, 3, u64::from(selected));
        }
    }
    push_message_field(&mut result, 13, &body);
    if result.len() > MAX_DROME_RESULT_BYTES {
        return Err(ArenaWireError::LimitExceeded);
    }
    Ok(result)
}
pub fn decode_drome_hazards(payload: &[u8]) -> Result<DromeHazards, ArenaWireError> {
    let (scalar, body) = split(payload)?;
    let fields = read_uint32_fields::<12>(&scalar)?;
    if fields[0] != 2 || !(1..=8).contains(&fields[1]) || fields[5] > 2 {
        return Err(ArenaWireError::Malformed);
    }
    let Some(body) = body else {
        return Ok(DromeHazards::default());
    };
    let (mut cursor, mut observed, mut entries, mut selected) = (0, None, Vec::new(), Vec::new());
    while cursor < body.len() {
        match read_varint(body, &mut cursor)? {
            8 => crate::charm_wire::set_once(&mut observed, read_varint(body, &mut cursor)?)?,
            18 => {
                if entries.len() == 2 {
                    return Err(ArenaWireError::LimitExceeded);
                }
                entries.push(decode_entry(read_bytes(body, &mut cursor)?)?);
            }
            24 => {
                if selected.len() == 2 {
                    return Err(ArenaWireError::LimitExceeded);
                }
                selected.push(
                    u8::try_from(read_varint(body, &mut cursor)?)
                        .map_err(|_| ArenaWireError::Malformed)?,
                );
            }
            _ => return Err(ArenaWireError::Malformed),
        }
    }
    let selected = match selected.as_slice() {
        [] => [0, 0],
        [a, b] => [*a, *b],
        _ => return Err(ArenaWireError::Malformed),
    };
    let snapshot = DromeHazards {
        selected,
        observed_at_micros: observed.ok_or(ArenaWireError::Malformed)?,
        entries,
    };
    if snapshot.entries.is_empty() && snapshot.selected == [0, 0] {
        return Err(ArenaWireError::Malformed);
    }
    validate(&snapshot)?;
    Ok(snapshot)
}
pub(super) fn scalar_result(payload: &[u8]) -> Result<Vec<u8>, ArenaWireError> {
    let (scalar, _) = split(payload)?;
    decode_drome_hazards(payload)?;
    Ok(scalar)
}
fn split(payload: &[u8]) -> Result<(Vec<u8>, Option<&[u8]>), ArenaWireError> {
    if payload.len() > MAX_DROME_RESULT_BYTES {
        return Err(ArenaWireError::LimitExceeded);
    }
    let (mut cursor, mut scalar, mut hazards) = (0, Vec::new(), None);
    while cursor < payload.len() {
        let start = cursor;
        let tag = read_varint(payload, &mut cursor)?;
        if tag == 106 {
            crate::charm_wire::set_once(&mut hazards, read_bytes(payload, &mut cursor)?)?;
        } else if tag & 7 == 0 && (1..=12).contains(&(tag >> 3)) {
            read_varint(payload, &mut cursor)?;
            scalar.extend_from_slice(&payload[start..cursor]);
        } else {
            return Err(ArenaWireError::Malformed);
        }
    }
    Ok((scalar, hazards))
}
fn decode_entry(body: &[u8]) -> Result<DromeHazard, ArenaWireError> {
    let (mut cursor, mut kind, mut expires, mut positions) = (0, None, None, Vec::new());
    while cursor < body.len() {
        match read_varint(body, &mut cursor)? {
            8 => crate::charm_wire::set_once(
                &mut kind,
                match read_varint(body, &mut cursor)? {
                    5 => DromeHazardKind::Lava,
                    6 => DromeHazardKind::Beam,
                    7 => DromeHazardKind::Tanked,
                    8 => DromeHazardKind::Sorrow,
                    9 => DromeHazardKind::Roots,
                    _ => return Err(ArenaWireError::Malformed),
                },
            )?,
            16 => crate::charm_wire::set_once(&mut expires, read_varint(body, &mut cursor)?)?,
            26 => {
                if positions.len() == 100 {
                    return Err(ArenaWireError::LimitExceeded);
                }
                positions.push(decode_position(read_bytes(body, &mut cursor)?)?);
            }
            _ => return Err(ArenaWireError::Malformed),
        }
    }
    Ok(DromeHazard {
        kind: kind.ok_or(ArenaWireError::Malformed)?,
        expires_at_micros: expires.ok_or(ArenaWireError::Malformed)?,
        positions,
    })
}
fn decode_position(body: &[u8]) -> Result<crate::world_spatial::ActorPosition, ArenaWireError> {
    let (mut cursor, mut x, mut y, mut floor) = (0, None, None, None);
    while cursor < body.len() {
        match read_varint(body, &mut cursor)? {
            8 => crate::charm_wire::set_once(&mut x, read_varint(body, &mut cursor)?)?,
            16 => crate::charm_wire::set_once(&mut y, read_varint(body, &mut cursor)?)?,
            24 => crate::charm_wire::set_once(&mut floor, read_varint(body, &mut cursor)?)?,
            _ => return Err(ArenaWireError::Malformed),
        }
    }
    let (x, y, floor) = (
        x.ok_or(ArenaWireError::Malformed)?,
        y.ok_or(ArenaWireError::Malformed)?,
        floor.ok_or(ArenaWireError::Malformed)?,
    );
    if x > 65535 || y > 65535 || floor > 30 {
        return Err(ArenaWireError::Malformed);
    }
    Ok(crate::world_spatial::ActorPosition {
        x: x as i32,
        y: y as i32,
        floor: floor as i16 - 15,
    })
}
fn validate(snapshot: &DromeHazards) -> Result<(), ArenaWireError> {
    if snapshot.selected != [0, 0]
        && (snapshot.selected[0] == snapshot.selected[1]
            || snapshot.selected.iter().any(|id| !(1..=9).contains(id)))
    {
        return Err(ArenaWireError::Malformed);
    }
    if snapshot.entries.len() > 2 {
        return Err(ArenaWireError::LimitExceeded);
    }
    let mut kinds = Vec::new();
    for entry in &snapshot.entries {
        if kinds.contains(&entry.kind) {
            return Err(ArenaWireError::Malformed);
        }
        kinds.push(entry.kind);
        let count = if matches!(entry.kind, DromeHazardKind::Sorrow | DromeHazardKind::Roots) {
            1
        } else {
            100
        };
        if entry.positions.len() != count
            || entry.expires_at_micros <= snapshot.observed_at_micros
            || entry.expires_at_micros - snapshot.observed_at_micros > 6_000_000
        {
            return Err(ArenaWireError::Malformed);
        }
        let mut positions = Vec::new();
        for position in &entry.positions {
            if !(0..=65535).contains(&position.x)
                || !(0..=65535).contains(&position.y)
                || !(-15..=15).contains(&position.floor)
                || positions.contains(position)
            {
                return Err(ArenaWireError::Malformed);
            }
            positions.push(*position);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
