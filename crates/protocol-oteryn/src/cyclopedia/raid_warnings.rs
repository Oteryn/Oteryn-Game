//! Capability-gated raid warning reply for the Cyclopedia map query.
use super::MapWireError;

pub const CAPABILITY_CYCLOPEDIA_RAID_WARNINGS_V1: u32 = 626;
pub const QUERY: [u8; 1] = [129];
pub const MAX_WARNINGS: usize = 1024;
const RECORD_BYTES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Warning {
    pub area: u32,
    pub subarea: Option<u32>,
}

pub fn encode(warnings: &[Warning]) -> Result<Vec<u8>, MapWireError> {
    if warnings.len() > MAX_WARNINGS || !valid(warnings) {
        return Err(MapWireError);
    }
    let mut bytes = (warnings.len() as u16).to_le_bytes().to_vec();
    for warning in warnings {
        bytes.extend(warning.area.to_le_bytes());
        bytes.extend(warning.subarea.unwrap_or(0).to_le_bytes());
    }
    Ok(bytes)
}

pub fn decode(bytes: &[u8]) -> Result<Vec<Warning>, MapWireError> {
    let (count, records) = bytes.split_first_chunk::<2>().ok_or(MapWireError)?;
    let count = usize::from(u16::from_le_bytes(*count));
    if count > MAX_WARNINGS || records.len() != count * RECORD_BYTES {
        return Err(MapWireError);
    }
    let (records, _) = records.as_chunks::<RECORD_BYTES>();
    let warnings = records
        .iter()
        .map(|&[a, b, c, d, e, f, g, h]| {
            let subarea = u32::from_le_bytes([e, f, g, h]);
            Warning {
                area: u32::from_le_bytes([a, b, c, d]),
                subarea: (subarea != 0).then_some(subarea),
            }
        })
        .collect::<Vec<_>>();
    valid(&warnings).then_some(warnings).ok_or(MapWireError)
}

fn valid(warnings: &[Warning]) -> bool {
    warnings.iter().all(|warning| {
        warning.area != 0 && warning.subarea != Some(0) && warning.subarea != Some(warning.area)
    }) && warnings.windows(2).all(|pair| pair[0] < pair[1])
}

#[cfg(test)]
#[path = "raid_warnings_tests.rs"]
mod tests;
