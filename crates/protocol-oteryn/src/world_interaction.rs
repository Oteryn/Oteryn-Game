//! World interaction wire (WORLDINT-WIRE-1).
//!
//! Schema: `docs/contracts/protocol-oteryn/v1/world_interaction_v1.proto`, from WORLD-INTERACTION-0
//! §10.1. Capability 11 `WORLD_INTERACTION_V1` (requires 15 `ITEM_USE_V1` and 18
//! `WORLD_MAP_VIEW_V1`) and command type 15 `PUSH_INTENT` are registered in
//! `PROTOCOL_OTERYN_V1_REGISTRY.json`, leased by the #1622 control plane. The capability also adds
//! `UseIntentV1` field 6 and the use dispositions 11 to 17 (`world_object.rs`); the server does not
//! offer it before WORLDINT-USE-1 composes the interaction gate.
//!
//! The capability changes no other command: command type 9 keeps its meaning, and its map-item and
//! Ground sources are a separate registry change.
//!
//! `PUSH_INTENT` is flat: the 16-byte identity, the full `uint64` generation, and the destination x
//! and y, both always present (zero is an explicit varint, never an absent field). That is 37
//! bytes at most, 5 over `WORLDINT0-RL-21`'s 32; narrowing the generation to `uint32` would give
//! exactly 32 but breaks the `session-generation` width, so the bound is held at 37 pending an
//! owner decision. A creature push never changes floor (Canary
//! `Game::playerMoveCreature` bounds the throw by `getThrowRange`, 1, against a z distance
//! times 4), so the destination floor is the target's and is not sent.
//!
//! Decoding is strict, and encoding refuses the same values as a fault before any byte is emitted:
//! an identity that is not 16 bytes, a zero generation, a coordinate outside the 3-byte
//! range, a missing target, generation, x or y, a payload over its bound, and unknown
//! or repeated fields all fail closed.

use crate::charm_wire::{
    WireResult, push_message_field, push_nonzero_varint_field, push_varint, read_bytes,
    read_varint, set_once,
};
use crate::item_view::decode_sint32;
use crate::world_object::{
    CAPABILITY_ITEM_USE_V1, MAX_USE_RESULT_BYTES, UseDisposition, WorldObjectError,
    decode_use_result_with_gate, encode_use_result_with_gate,
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

/// Destination coordinates are `sint32` limited to 3-byte varints: -2^20 to 2^20 - 1.
pub const MIN_PUSH_COORDINATE: i32 = -(1 << 20);
pub const MAX_PUSH_COORDINATE: i32 = (1 << 20) - 1;
/// `PushIntentV1`: identity 1 + 1 + 16, generation 1 + 10 (u64), x 1 + 3, y 1 + 3 = 37 bytes.
/// `WORLDINT0-RL-21` says 32; see the module docs.
pub const MAX_PUSH_INTENT_BYTES: usize = 37;

fn push_explicit_sint32(output: &mut Vec<u8>, field: u64, value: i32) {
    let zigzag = ((value << 1) ^ (value >> 31)) as u32;
    push_varint(output, field << 3);
    push_varint(output, u64::from(zigzag));
}
/// `PushResultV1` is the USE result message, at most 4 bytes.
pub const MAX_PUSH_RESULT_BYTES: usize = MAX_USE_RESULT_BYTES;

/// A decoded `PushIntentV1`: the target on the target's floor, pushed to `(to_x, to_y)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PushIntent {
    pub target: EntityRef,
    pub to_x: i32,
    pub to_y: i32,
}

fn coordinate_in_range(value: i32) -> bool {
    (MIN_PUSH_COORDINATE..=MAX_PUSH_COORDINATE).contains(&value)
}

pub fn encode_push_intent(intent: &PushIntent) -> WireResult<Vec<u8>> {
    if intent.target.generation == 0
        || !coordinate_in_range(intent.to_x)
        || !coordinate_in_range(intent.to_y)
    {
        return Err(WorldInteractionWireError::Malformed);
    }
    let mut output = Vec::with_capacity(MAX_PUSH_INTENT_BYTES);
    push_message_field(&mut output, 1, &intent.target.identity);
    push_nonzero_varint_field(&mut output, 2, intent.target.generation);
    push_explicit_sint32(&mut output, 3, intent.to_x);
    push_explicit_sint32(&mut output, 4, intent.to_y);
    Ok(output)
}

/// The target, generation and both coordinates are required; zero must be written explicitly.
pub fn decode_push_intent(payload: &[u8]) -> WireResult<PushIntent> {
    if payload.len() > MAX_PUSH_INTENT_BYTES {
        return Err(WorldInteractionWireError::LimitExceeded);
    }
    let (mut identity, mut generation, mut to_x, mut to_y) = (None, None, None, None);
    let mut cursor = 0;
    while cursor < payload.len() {
        match read_varint(payload, &mut cursor)? {
            0x0a => {
                let bytes: [u8; ENTITY_IDENTITY_BYTES] = read_bytes(payload, &mut cursor)?
                    .try_into()
                    .map_err(|_| WorldInteractionWireError::Malformed)?;
                set_once(&mut identity, bytes)?;
            }
            0x10 => set_once(&mut generation, read_varint(payload, &mut cursor)?)?,
            0x18 => set_once(
                &mut to_x,
                decode_sint32(read_varint(payload, &mut cursor)?)?,
            )?,
            0x20 => set_once(
                &mut to_y,
                decode_sint32(read_varint(payload, &mut cursor)?)?,
            )?,
            _ => return Err(WorldInteractionWireError::Malformed),
        }
    }
    let (to_x, to_y) = (
        to_x.ok_or(WorldInteractionWireError::Malformed)?,
        to_y.ok_or(WorldInteractionWireError::Malformed)?,
    );
    if !coordinate_in_range(to_x) || !coordinate_in_range(to_y) {
        return Err(WorldInteractionWireError::Malformed);
    }
    Ok(PushIntent {
        target: EntityRef {
            identity: identity.ok_or(WorldInteractionWireError::Malformed)?,
            generation: generation
                .filter(|generation| *generation != 0)
                .ok_or(WorldInteractionWireError::Malformed)?,
        },
        to_x,
        to_y,
    })
}

/// `PushResultV1`: the disposition and, with `SEALED` only, the gate message id (1 to 127).
pub fn encode_push_result(
    disposition: UseDisposition,
    gate_message_id: Option<u8>,
) -> Result<Vec<u8>, WorldObjectError> {
    encode_use_result_with_gate(disposition, gate_message_id)
}

pub fn decode_push_result(
    payload: &[u8],
) -> Result<(UseDisposition, Option<u8>), WorldObjectError> {
    decode_use_result_with_gate(payload)
}

#[cfg(test)]
#[path = "world_interaction_tests.rs"]
mod tests;
