//! TARGET-MOVE-1: bounded server walking, stop, turn and visible-entity follow.
use crate::world_spatial::{ActorPosition, StepDirection, WorldSpatialError};
use crate::world_spatial_entities::EntityRef;

pub const CAPABILITY_TARGET_MOVE_V1: u32 = 64;
pub const COMMAND_TYPE_TARGET_MOVE_INTENT: u32 = 81;
pub const MAX_TARGET_MOVE_BYTES: usize = 48;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetMoveIntent {
    Walk(ActorPosition),
    Stop,
    Turn(StepDirection),
    Follow(EntityRef),
}

/// Exactly one protobuf oneof: cell (1), stop=true (2), step direction (3), entity (4).
pub fn decode_target_move_intent(payload: &[u8]) -> Result<TargetMoveIntent, WorldSpatialError> {
    use crate::charm_wire::{read_bytes, read_varint};
    if payload.len() > MAX_TARGET_MOVE_BYTES {
        return Err(WorldSpatialError::LimitExceeded);
    }
    let malformed = || WorldSpatialError::Malformed;
    let mut cursor = 0;
    let tag = read_varint(payload, &mut cursor).map_err(|_| malformed())?;
    let intent = match tag {
        0x0a => TargetMoveIntent::Walk(crate::world_spatial::decode_position(
            read_bytes(payload, &mut cursor).map_err(|_| malformed())?,
        )?),
        0x10 if read_varint(payload, &mut cursor).map_err(|_| malformed())? == 1 => {
            TargetMoveIntent::Stop
        }
        0x1a => TargetMoveIntent::Turn(crate::world_spatial::decode_step_intent(
            read_bytes(payload, &mut cursor).map_err(|_| malformed())?,
        )?),
        0x22 => TargetMoveIntent::Follow(
            crate::attack::decode_attack_target_intent(
                read_bytes(payload, &mut cursor).map_err(|_| malformed())?,
            )
            .map_err(|_| malformed())?
            .ok_or_else(malformed)?,
        ),
        _ => return Err(malformed()),
    };
    if cursor != payload.len() {
        return Err(malformed());
    }
    Ok(intent)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn target_move_strict_oneof_and_bounds() {
        assert_eq!(
            decode_target_move_intent(&[16, 1]),
            Ok(TargetMoveIntent::Stop)
        );
        assert_eq!(
            decode_target_move_intent(&[26, 2, 8, 4]),
            Ok(TargetMoveIntent::Turn(StepDirection::West))
        );
        for bytes in [
            vec![],
            vec![16, 0],
            vec![16, 1, 16, 1],
            vec![26, 2, 8, 5],
            vec![34, 0],
            vec![40, 1],
            vec![0; 49],
        ] {
            assert!(decode_target_move_intent(&bytes).is_err(), "{bytes:?}");
        }
    }
}
