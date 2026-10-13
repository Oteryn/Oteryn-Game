//! World interaction wire (WORLDINT-WIRE-1).
//!
//! Schema: `docs/contracts/protocol-oteryn/v1/world_interaction_v1.proto`, from WORLD-INTERACTION-0
//! §10.1. Capability 11 `WORLD_INTERACTION_V1` (requires 15 `ITEM_USE_V1` and 18
//! `WORLD_MAP_VIEW_V1`) and command type 15 `PUSH_INTENT` are registered in
//! `PROTOCOL_OTERYN_V1_REGISTRY.json`, leased by the #1622 control plane. The capability also adds
//! `UseIntentV1` field 6 and the use dispositions 11 to 17 (`world_object.rs`); the server does not
//! offer it before WORLDINT-USE-1 composes the interaction gate.
//!
//! Decoding is strict, and encoding refuses the same values as a fault before any byte is emitted:
//! a target whose identity is not 16 bytes or whose generation is zero, a missing target or tile, a
//! payload over its bound, and unknown or repeated fields all fail closed.

use crate::charm_wire::{WireResult, push_message_field, read_bytes, read_varint, set_once};
use crate::item_view::{WorldTilePosition, decode_position, encode_position};
use crate::world_object::{
    CAPABILITY_ITEM_USE_V1, MAX_USE_RESULT_BYTES, UseDisposition, WorldObjectError,
    decode_use_result, encode_use_result,
};
use crate::world_spatial_entities::{ENTITY_IDENTITY_BYTES, EntityRef};

pub use crate::charm_wire::CyclopediaWireError as WorldInteractionWireError;

/// Registered capability `WORLD_INTERACTION_V1`: command type 15, no state domain.
pub const CAPABILITY_WORLD_INTERACTION_V1: u32 = 11;
/// The capability `WORLD_INTERACTION_V1` requires: 15 `ITEM_USE_V1` and 18 `WORLD_MAP_VIEW_V1`.
pub const CAPABILITY_WORLD_INTERACTION_V1_REQUIRES: [u32; 2] = [
    CAPABILITY_ITEM_USE_V1,
    crate::world_map::CAPABILITY_WORLD_MAP_VIEW_V1,
];
/// Registered command type `PUSH_INTENT` (capability 11).
pub const COMMAND_TYPE_PUSH_INTENT: u32 = 15;

/// An `EntityRefV1` with a target: identity 1 + 1 + 16, generation 1 + 10.
const MAX_TARGET_REF_BYTES: usize = 29;
/// `PushIntentV1`: the target 1 + 1 + 29 and the tile 1 + 1 + 16 (`MAX_WORLD_TILE_POSITION_BYTES`).
/// WORLD-INTERACTION-0 §10.1 estimated 32 (`WORLDINT0-RL-21`); the measured bound is larger because
/// the target is the 16-byte spatial identity.
pub const MAX_PUSH_INTENT_BYTES: usize = 49;
/// `PushResultV1` is the USE result message, at most 4 bytes.
pub const MAX_PUSH_RESULT_BYTES: usize = MAX_USE_RESULT_BYTES;

/// A decoded `PushIntentV1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PushIntent {
    pub target: EntityRef,
    pub to: WorldTilePosition,
}

pub fn encode_push_intent(intent: &PushIntent) -> WireResult<Vec<u8>> {
    if intent.target.generation == 0 {
        return Err(WorldInteractionWireError::Malformed);
    }
    let mut target = Vec::with_capacity(MAX_TARGET_REF_BYTES);
    push_message_field(&mut target, 1, &intent.target.identity);
    crate::charm_wire::push_varint_field(&mut target, 2, intent.target.generation);
    let mut output = Vec::with_capacity(MAX_PUSH_INTENT_BYTES);
    push_message_field(&mut output, 1, &target);
    push_message_field(&mut output, 2, &encode_position(intent.to));
    Ok(output)
}

fn decode_target(input: &[u8]) -> WireResult<EntityRef> {
    let (mut identity, mut generation) = (None, None);
    let mut cursor = 0;
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            0x0a => {
                let bytes: [u8; ENTITY_IDENTITY_BYTES] = read_bytes(input, &mut cursor)?
                    .try_into()
                    .map_err(|_| WorldInteractionWireError::Malformed)?;
                set_once(&mut identity, bytes)?;
            }
            0x10 => set_once(&mut generation, read_varint(input, &mut cursor)?)?,
            _ => return Err(WorldInteractionWireError::Malformed),
        }
    }
    Ok(EntityRef {
        identity: identity.ok_or(WorldInteractionWireError::Malformed)?,
        generation: generation
            .filter(|generation| *generation != 0)
            .ok_or(WorldInteractionWireError::Malformed)?,
    })
}

/// Both the target and the tile are required: a push with no tile names nowhere.
pub fn decode_push_intent(payload: &[u8]) -> WireResult<PushIntent> {
    if payload.len() > MAX_PUSH_INTENT_BYTES {
        return Err(WorldInteractionWireError::LimitExceeded);
    }
    let (mut target, mut to) = (None, None);
    let mut cursor = 0;
    while cursor < payload.len() {
        match read_varint(payload, &mut cursor)? {
            0x0a => set_once(
                &mut target,
                decode_target(read_bytes(payload, &mut cursor)?)?,
            )?,
            0x12 => set_once(&mut to, decode_position(read_bytes(payload, &mut cursor)?)?)?,
            _ => return Err(WorldInteractionWireError::Malformed),
        }
    }
    Ok(PushIntent {
        target: target.ok_or(WorldInteractionWireError::Malformed)?,
        to: to.ok_or(WorldInteractionWireError::Malformed)?,
    })
}

/// `PushResultV1` carries a `UseDispositionV1`.
pub fn encode_push_result(disposition: UseDisposition) -> Vec<u8> {
    encode_use_result(disposition)
}

pub fn decode_push_result(payload: &[u8]) -> Result<UseDisposition, WorldObjectError> {
    decode_use_result(payload)
}

#[cfg(test)]
#[path = "world_interaction_tests.rs"]
mod tests;
