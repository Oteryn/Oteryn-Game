//! Explicit Rotate is separate from Use (Crystal game.cpp:4521-4573).
//! Capability 388 requires WORLD_INTERACTION_V1; older clients keep their USE behavior.
use crate::item_view::CAPABILITY_ITEM_VIEW_MOVE_V1;
pub use crate::world_object::{
    UseDisposition, UseIntent, UseTarget, WorldObjectError, WorldObjectTarget,
    decode_use_result_with_gate as decode_rotate_result,
    encode_use_result_with_gate as encode_rotate_result,
};
use crate::world_object::{decode_use_intent_target, encode_use_intent, encode_use_item_intent};

pub const CAPABILITY_WORLD_ROTATE_V1: u32 = 388;
pub const COMMAND_TYPE_ROTATE_WORLD_ITEM: u32 = 670;
pub const MAX_ROTATE_INTENT_BYTES: usize = crate::world_object::MAX_USE_INTENT_BYTES;
pub const MAX_ROTATE_RESULT_BYTES: usize = crate::world_object::MAX_USE_RESULT_BYTES;

/// Reuse opaque target encodings. Item handles additionally bind the item definition
/// the player observed, matching Crystal's client item-id check before Rotate.
pub fn encode_rotate_intent(
    target: &UseTarget,
    observed_definition: Option<std::num::NonZeroU32>,
) -> Result<Vec<u8>, WorldObjectError> {
    match (target, observed_definition) {
        (UseTarget::WorldObject(target), None) => encode_use_intent(target),
        (UseTarget::Item(handle), Some(definition)) => {
            let mut payload = encode_use_item_intent(*handle);
            crate::charm_wire::push_nonzero_varint_field(
                &mut payload,
                3,
                u64::from(definition.get()),
            );
            Ok(payload)
        }
        _ => Err(WorldObjectError::Malformed),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RotateRequest {
    pub intent: UseIntent,
    pub observed_definition: Option<std::num::NonZeroU32>,
    pub probe: bool,
}

/// An advisory carried-item affordance probe; never a transform request.
pub fn encode_rotate_probe(
    handle: crate::item_view::ItemHandle,
    observed_definition: std::num::NonZeroU32,
) -> Vec<u8> {
    let mut payload = encode_use_item_intent(handle);
    crate::charm_wire::push_nonzero_varint_field(
        &mut payload,
        3,
        u64::from(observed_definition.get()),
    );
    crate::charm_wire::push_nonzero_varint_field(&mut payload, 4, 1);
    payload
}

pub fn encode_rotate_affordance(allowed: bool) -> Vec<u8> {
    vec![u8::from(allowed)]
}

pub fn decode_rotate_affordance(payload: &[u8]) -> Result<bool, WorldObjectError> {
    match payload {
        [0] => Ok(false),
        [1] => Ok(true),
        _ => Err(WorldObjectError::Malformed),
    }
}

pub fn decode_rotate_intent(
    payload: &[u8],
) -> Result<(UseIntent, Option<std::num::NonZeroU32>), WorldObjectError> {
    let request = decode_rotate_request(payload)?;
    if request.probe {
        return Err(WorldObjectError::Malformed);
    }
    Ok((request.intent, request.observed_definition))
}

pub fn decode_rotate_request(payload: &[u8]) -> Result<RotateRequest, WorldObjectError> {
    use crate::charm_wire::{read_bytes, read_varint};
    if payload.len() > MAX_ROTATE_INTENT_BYTES {
        return Err(WorldObjectError::LimitExceeded);
    }
    let (mut target, mut observed_definition, mut probe) = (None, None, None);
    let mut cursor = 0;
    while cursor < payload.len() {
        let start = cursor;
        match read_varint(payload, &mut cursor).map_err(|_| WorldObjectError::Malformed)? {
            0x0a | 0x12 if target.is_none() => {
                read_bytes(payload, &mut cursor).map_err(|_| WorldObjectError::Malformed)?;
                target = Some(&payload[start..cursor]);
            }
            0x18 if observed_definition.is_none() => {
                let value =
                    read_varint(payload, &mut cursor).map_err(|_| WorldObjectError::Malformed)?;
                let value = u32::try_from(value).map_err(|_| WorldObjectError::Malformed)?;
                observed_definition =
                    Some(std::num::NonZeroU32::new(value).ok_or(WorldObjectError::Malformed)?);
            }
            0x20 if probe.is_none() => {
                let value =
                    read_varint(payload, &mut cursor).map_err(|_| WorldObjectError::Malformed)?;
                if value > 1 {
                    return Err(WorldObjectError::Malformed);
                }
                probe = Some(value == 1);
            }
            _ => return Err(WorldObjectError::Malformed),
        }
    }
    // Selecting only item targets refuses USE-WITH and ItemByDefinition.
    let intent = decode_use_intent_target(
        &[CAPABILITY_ITEM_VIEW_MOVE_V1],
        target.ok_or(WorldObjectError::Malformed)?,
    )?;
    let probe = probe.unwrap_or(false);
    match (&intent.target, observed_definition, probe) {
        (UseTarget::Item(_), Some(_), _) | (UseTarget::WorldObject(_), None, false) => {
            Ok(RotateRequest {
                intent,
                observed_definition,
                probe,
            })
        }
        _ => Err(WorldObjectError::Malformed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item_view::ItemHandle;

    #[test]
    fn inventory_probe_is_advisory_and_cannot_decode_as_rotate() -> Result<(), WorldObjectError> {
        let payload = encode_rotate_probe(ItemHandle::MIN, std::num::NonZeroU32::MIN);
        let request = decode_rotate_request(&payload)?;
        assert!(request.probe);
        assert_eq!(request.observed_definition, Some(std::num::NonZeroU32::MIN));
        assert!(decode_rotate_intent(&payload).is_err());
        let mut duplicate = payload;
        duplicate.extend([32, 1]);
        assert!(decode_rotate_request(&duplicate).is_err());
        let mut object = encode_rotate_intent(
            &UseTarget::WorldObject(WorldObjectTarget {
                placement: vec![9; 40],
                expected_revision: 1,
            }),
            None,
        )?;
        object.extend([32, 1]);
        assert!(decode_rotate_request(&object).is_err());
        for value in [false, true] {
            assert_eq!(
                decode_rotate_affordance(&encode_rotate_affordance(value))?,
                value
            );
        }
        for bad in [&[][..], &[2][..], &[8, 1][..]] {
            assert!(decode_rotate_affordance(bad).is_err());
        }
        Ok(())
    }

    #[test]
    fn rotate_wire_keeps_opaque_targets_and_rejects_use_with() -> Result<(), WorldObjectError> {
        for (target, definition) in [
            (
                UseTarget::Item(ItemHandle::MIN),
                Some(std::num::NonZeroU32::MIN),
            ),
            (
                UseTarget::Item(ItemHandle::MAX),
                Some(std::num::NonZeroU32::MAX),
            ),
            (
                UseTarget::WorldObject(WorldObjectTarget {
                    placement: vec![9; 40],
                    expected_revision: 7,
                }),
                None,
            ),
        ] {
            let payload = encode_rotate_intent(&target, definition)?;
            let (intent, observed) = decode_rotate_intent(&payload)?;
            assert_eq!(intent.target, target);
            assert_eq!(observed, definition);
            assert!(intent.use_with.is_none());
            assert!(intent.use_with_map_item.is_none());
            assert!(intent.use_with_position.is_none());
        }
        let mut with = encode_rotate_intent(
            &UseTarget::Item(ItemHandle::MIN),
            Some(std::num::NonZeroU32::MIN),
        )?;
        with.extend([34, 0]);
        assert!(decode_rotate_intent(&with).is_err());
        assert!(decode_rotate_intent(&[42, 2, 8, 1]).is_err());
        assert!(decode_rotate_intent(&[0; MAX_ROTATE_INTENT_BYTES + 1]).is_err());
        assert!(decode_rotate_intent(&[]).is_err());
        assert!(decode_rotate_intent(&[18, 2, 8, 1]).is_err());
        assert!(decode_rotate_intent(&[18, 2, 8, 1, 24, 0]).is_err());
        assert!(decode_rotate_intent(&[18, 2, 8, 1, 24, 128, 128, 128, 128, 16]).is_err());
        assert!(decode_rotate_intent(&[18, 2, 8, 1, 24, 1, 24, 1]).is_err());
        assert!(encode_rotate_intent(&UseTarget::Item(ItemHandle::MIN), None).is_err());
        Ok(())
    }
}
