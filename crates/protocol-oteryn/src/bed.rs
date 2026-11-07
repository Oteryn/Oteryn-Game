//! BED0 §9: command 31 under WORLD_INTERACTION_V1. A handle is session-local;
//! the server resolves it against the current map before using either bed half.

use crate::charm_wire::{WireResult, read_bytes, read_varint, set_once};
use crate::item_view::{ItemHandle, decode_item_target};
use crate::world_interaction::WorldInteractionWireError;

pub const COMMAND_TYPE_BED_SLEEP_INTENT: u32 = 31;
pub const MAX_BED_SLEEP_INTENT_BYTES: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BedSleepIntent {
    pub target: ItemHandle,
    /// 1 fist, 2 club, 3 sword, 4 axe, 5 distance, 6 magic level.
    pub skill: u8,
}

/// Both fields are required, including the nested nonzero map-item handle.
pub fn decode_bed_sleep_intent(payload: &[u8]) -> WireResult<BedSleepIntent> {
    if payload.len() > MAX_BED_SLEEP_INTENT_BYTES {
        return Err(WorldInteractionWireError::LimitExceeded);
    }
    let (mut target, mut skill) = (None, None);
    let mut cursor = 0;
    while cursor < payload.len() {
        match read_varint(payload, &mut cursor)? {
            0x0a => set_once(
                &mut target,
                decode_item_target(read_bytes(payload, &mut cursor)?)
                    .map_err(|_| WorldInteractionWireError::Malformed)?,
            )?,
            0x10 => {
                let value = read_varint(payload, &mut cursor)?;
                if !(1..=6).contains(&value) {
                    return Err(WorldInteractionWireError::Malformed);
                }
                set_once(&mut skill, value as u8)?;
            }
            _ => return Err(WorldInteractionWireError::Malformed),
        }
    }
    Ok(BedSleepIntent {
        target: target.ok_or(WorldInteractionWireError::Malformed)?,
        skill: skill.ok_or(WorldInteractionWireError::Malformed)?,
    })
}

#[cfg(test)]
#[path = "bed_tests.rs"]
mod tests;
