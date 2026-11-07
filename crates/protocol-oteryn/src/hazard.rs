//! Hazard selection: empty payload queries; a single level byte selects an unlocked level.
pub use crate::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1 as CAPABILITY_HAZARD_V1;
pub const COMMAND_TYPE_HAZARD: u32 = 169;

/// Returns a selection or a query; unknown fields and levels are refused.
pub fn decode_request(bytes: &[u8]) -> Result<Option<u16>, &'static str> {
    match bytes {
        [] => Ok(None),
        [level @ 1..=12] => Ok(Some(u16::from(*level))),
        _ => Err("invalid hazard selection"),
    }
}
