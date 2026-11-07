//! PUSH-ROUTE-1: push one adjacent creature or player to one adjacent cell.
use crate::world_spatial::{ActorPosition, WorldSpatialError};
use crate::world_spatial_entities::EntityRef;

pub const CAPABILITY_CREATURE_PUSH_V1: u32 = 82;
pub const COMMAND_TYPE_CREATURE_PUSH_INTENT: u32 = 98;
pub const MAX_CREATURE_PUSH_BYTES: usize = 64;

/// `CreaturePushResult.disposition`; only [`CreaturePushDisposition::Moved`] moved anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CreaturePushDisposition {
    Moved = 1,
    NotMovable = 2,
    Rejected = 3,
    NotPossible = 4,
    NotEnoughRoom = 5,
    DestinationOutOfReach = 6,
    Exhausted = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreaturePushIntent {
    pub target: EntityRef,
    pub destination: ActorPosition,
}

/// Exactly target (1) then destination (2), each once; anything else is malformed.
pub fn decode_creature_push_intent(
    payload: &[u8],
) -> Result<CreaturePushIntent, WorldSpatialError> {
    use crate::charm_wire::{read_bytes, read_varint};
    if payload.len() > MAX_CREATURE_PUSH_BYTES {
        return Err(WorldSpatialError::LimitExceeded);
    }
    let malformed = || WorldSpatialError::Malformed;
    let mut cursor = 0;
    if read_varint(payload, &mut cursor).map_err(|_| malformed())? != 0x0a {
        return Err(malformed());
    }
    let target = crate::attack::decode_attack_target_intent(
        read_bytes(payload, &mut cursor).map_err(|_| malformed())?,
    )
    .map_err(|_| malformed())?
    .ok_or_else(malformed)?;
    if read_varint(payload, &mut cursor).map_err(|_| malformed())? != 0x12 {
        return Err(malformed());
    }
    let destination = crate::world_spatial::decode_position(
        read_bytes(payload, &mut cursor).map_err(|_| malformed())?,
    )?;
    if cursor != payload.len() {
        return Err(malformed());
    }
    Ok(CreaturePushIntent {
        target,
        destination,
    })
}

/// The client side of [`decode_creature_push_intent`].
pub fn encode_creature_push_intent(
    intent: &CreaturePushIntent,
) -> Result<Vec<u8>, WorldSpatialError> {
    let target = crate::attack::encode_attack_target_intent(Some(&intent.target))
        .map_err(|_| WorldSpatialError::Malformed)?;
    let destination = crate::world_spatial::encode_position(&intent.destination);
    let mut output = Vec::with_capacity(4 + target.len() + destination.len());
    for (tag, field) in [(0x0a, target), (0x12, destination)] {
        output.push(tag);
        output.push(u8::try_from(field.len()).map_err(|_| WorldSpatialError::LimitExceeded)?);
        output.extend_from_slice(&field);
    }
    if output.len() > MAX_CREATURE_PUSH_BYTES {
        return Err(WorldSpatialError::LimitExceeded);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent() -> CreaturePushIntent {
        CreaturePushIntent {
            target: EntityRef {
                identity: [0x7f; 16],
                generation: u64::MAX,
            },
            destination: ActorPosition {
                x: i32::MIN,
                y: i32::MAX,
                floor: 15,
            },
        }
    }

    #[test]
    fn creature_push_strict_fields_and_bounds() -> Result<(), WorldSpatialError> {
        let bytes = encode_creature_push_intent(&intent())?;
        assert!(bytes.len() <= MAX_CREATURE_PUSH_BYTES, "{}", bytes.len());
        assert_eq!(decode_creature_push_intent(&bytes), Ok(intent()));
        let target_len = usize::from(bytes[1]);
        let target = bytes[..2 + target_len].to_vec();
        let destination = bytes[2 + target_len..].to_vec();
        let mut swapped = destination.clone();
        swapped.extend_from_slice(&target);
        let mut repeated = bytes.clone();
        repeated.extend_from_slice(&destination);
        let mut unknown = bytes.clone();
        unknown.extend_from_slice(&[24, 1]);
        for bad in [
            vec![],
            target.clone(),
            destination,
            swapped,
            repeated,
            unknown,
            // No target: an empty AttackTargetIntent means "stop", not a push.
            [vec![10, 0], bytes[2 + target_len..].to_vec()].concat(),
            vec![0; MAX_CREATURE_PUSH_BYTES + 1],
        ] {
            assert!(decode_creature_push_intent(&bad).is_err(), "{bad:?}");
        }
        assert_eq!(
            decode_creature_push_intent(&[0; MAX_CREATURE_PUSH_BYTES + 1]),
            Err(WorldSpatialError::LimitExceeded)
        );
        Ok(())
    }
}
