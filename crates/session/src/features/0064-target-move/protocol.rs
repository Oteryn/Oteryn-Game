//! Typed target-cell input through the existing admitted command owner.
use crate::{ActorPosition, GatedCommandResult, Session, SessionError, SessionStream};
use oteryn_protocol_oteryn::target_move;

impl<S: SessionStream> Session<S> {
    /// Requests server-owned pathfinding. Movement arrives separately on domain 1.
    pub async fn target_walk(
        &mut self,
        position: ActorPosition,
    ) -> Result<GatedCommandResult, SessionError> {
        let payload = encode_cell(position);
        target_move::decode_target_move_intent(&payload).map_err(SessionError::WorldSpatial)?;
        self.gated_command(
            target_move::CAPABILITY_TARGET_MOVE_V1,
            target_move::COMMAND_TYPE_TARGET_MOVE_INTENT,
            &payload,
        )
        .await
    }
}

// Client adapter for the existing protobuf cell oneof; no protocol schema change.
fn encode_cell(position: ActorPosition) -> Vec<u8> {
    let mut cell = Vec::new();
    for (field, value) in [
        (1, position.x),
        (2, position.y),
        (3, i32::from(position.floor)),
    ] {
        cell.push(field << 3);
        let mut value = ((value as u32) << 1) ^ ((value >> 31) as u32);
        while value >= 128 {
            cell.push((value as u8 & 127) | 128);
            value >>= 7;
        }
        cell.push(value as u8);
    }
    let mut payload = vec![10, cell.len() as u8];
    payload.extend(cell);
    payload
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_roundtrip_at_signed_coordinate_and_floor_limits() -> Result<(), SessionError> {
        for position in [
            ActorPosition {
                x: 0,
                y: 0,
                floor: 0,
            },
            ActorPosition {
                x: i32::MIN,
                y: i32::MAX,
                floor: i16::MIN,
            },
            ActorPosition {
                x: -1,
                y: 200,
                floor: i16::MAX,
            },
        ] {
            let payload = encode_cell(position);
            assert!(payload.len() <= target_move::MAX_TARGET_MOVE_BYTES);
            assert_eq!(
                target_move::decode_target_move_intent(&payload)?,
                target_move::TargetMoveIntent::Walk(position)
            );
        }
        Ok(())
    }
}
