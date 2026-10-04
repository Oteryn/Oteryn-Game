//! `USE-WIRE-V1` typed payloads (#162 owner acceptance 5864914163; schema in
//! `docs/contracts/protocol-oteryn/v1/world_object_v1.proto`). Command type 2
//! `USE_INTENT` and state domain 2 `WORLD_OBJECT_OVERLAY` with delta type 1 and
//! snapshot type 1. The client sends intent only; the server selects the unique bound
//! transition out of the object's current state (composed in M2). Decoding is strict:
//! zero or unknown enum values, unknown or repeated fields, over-bound payloads and a
//! `content_generation` other than 32 bytes all fail closed. A standard proto3 encoder
//! omits a scalar or bytes field holding its default value (an empty `placement`/`state`
//! key, a zero `expected_revision`/`revision`), so decoding accepts that omission and
//! defaults the field; semantic presence is validated after decoding, not inferred from
//! wire-default omission (FND-02 §7). Encoding is likewise strict: an oversized
//! `placement`/`state` key is refused before any bytes are emitted, so this side never
//! produces a payload a conforming decoder would reject.

// The client-side codecs (intent encode, result and overlay decode) are exercised by
// the round-trip tests; the server composes only its own direction (M2).
#![cfg_attr(not(test), allow(dead_code))]

use std::num::NonZeroU32;

use crate::item_view::{
    CAPABILITY_ITEM_VIEW_MOVE_V1, ItemHandle, decode_item_target, encode_item_target,
};
use crate::world_spatial_entities::{ENTITY_IDENTITY_BYTES, EntityRef};

/// Registered capability `ITEM_USE_V1` (ITEM-USE-0 §3; number leased by ARCH-BATCH-ROOT-PACKETS-V1
/// §0.1). It owns no command type or domain: it extends command type 2 with fields 4 and 5 and the
/// dispositions 7 to 10. Not offered before ITEM-USE-1.
pub const CAPABILITY_ITEM_USE_V1: u32 = 15;
/// The registry `requires` of capability 15.
pub const CAPABILITY_ITEM_USE_V1_REQUIRES: &[u32] = &[CAPABILITY_ITEM_VIEW_MOVE_V1];

pub const COMMAND_TYPE_USE_INTENT: u32 = 2;
pub const STATE_DOMAIN_WORLD_OBJECT_OVERLAY: u32 = 2;
pub const DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1: u32 = 1;
pub const SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1: u32 = 1;

/// USE-WIRE-V1 semantic bounds (#162 comment 5864914163).
///
/// `WorldObjectTargetV1`: `placement` (1 tag + 2 length + 512 bytes = 515) plus
/// `expected_revision` (1 tag + 10-byte u64 varint = 11) = 526, wrapped as the
/// `UseIntentV1.world_object` submessage field (1 tag + 2 length = 3) = 529.
///
/// `ITEMUSE0-RL-03`: fields 4 and 5 keep this bound. The largest intent with them is field 2 (a
/// 10-byte handle: 2 + 11 = 13) or field 5 (a 5-byte index: 2 + 6 = 8) plus field 4 (2 + 29 = 31),
/// so 44 bytes; field 4 never combines with field 1.
pub const MAX_USE_INTENT_BYTES: usize = 529;
/// The encoded `EntityRefV1` of field 4: identity (1 + 1 + 16) and generation (1 + 10).
const MAX_USE_WITH_BYTES: usize = 29;
/// A single small enum field (1 tag + 1 value byte = 2), with the same slack as
/// `MAX_STEP_RESULT_BYTES` in `world_spatial.rs` to keep the repeated-field and
/// over-bound decode failures distinct in tests.
pub const MAX_USE_RESULT_BYTES: usize = 4;
/// One `WorldObjectOverlayEntryV1` as an element of a repeated `entries` field:
/// see `WOBJ-RL-03` in `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` for the full
/// arithmetic. Delta payload = exactly one such entry (`WOBJ-RL-02` = 1).
pub const MAX_WORLD_OBJECT_OVERLAY_DELTA_BYTES: usize = 1078;
/// `WOBJ-RL-03` (= 486) entries, each at most `MAX_WORLD_OBJECT_OVERLAY_DELTA_BYTES`:
/// derived from the existing single-chunk snapshot bound `FND02-SNAPSHOT-CHUNK-BYTES`
/// (524,288 bytes; `apps/game-server/src/foundation/protocol.rs`
/// `MAX_SNAPSHOT_CHUNK_BYTES`). 486 * 1,078 = 523,908.
pub const MAX_WORLD_OBJECT_OVERLAY_SNAPSHOT_BYTES: usize = 523_908;
/// `WOBJ-RL-03`: overlay entries per snapshot.
pub const MAX_SNAPSHOT_ENTRIES: usize = 486;
const CONTENT_GENERATION_BYTES: usize = 32;
/// The existing accepted content production-key maximum (`WOBJ-RL-03` note), reused
/// for both `placement` and the overlay `state` key. Mirrors
/// `apps/game-server/src/content/production.rs::FIRST_PRODUCTION_MAX_KEY_BYTES`; this crate
/// cannot depend on `oteryn-game-server`, so the accepted value is duplicated as a literal.
const MAX_KEY_BYTES: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldObjectError {
    Malformed,
    LimitExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldObjectTarget {
    pub placement: Vec<u8>,
    pub expected_revision: u64,
}

/// The `UseIntentV1.target` oneof. `Item` is field 2 (ITEM-MOVE-WIRE-0 §4.3), accepted only under
/// capability 4. `ItemByDefinition` is field 5 (ITEM-USE-0 §3, the hotkey form): the 1-based index
/// of the item definition in the active content generation, accepted only under capability 15.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UseTarget {
    WorldObject(WorldObjectTarget),
    Item(ItemHandle),
    ItemByDefinition(NonZeroU32),
}

/// A decoded `UseIntentV1`: the target and field 4 `use_with`, the creature a potion is used on
/// (absent means the user). Field 4 is accepted only under capability 15 and only with field 2 or 5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseIntent {
    pub target: UseTarget,
    pub use_with: Option<EntityRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseDisposition {
    Committed = 1,
    NothingToUse = 2,
    Occupied = 3,
    StaleState = 4,
    TooFar = 5,
    Rejected = 6,
    /// Dispositions 7 to 10 are sent only under capability 15 (ITEM-USE-0 §3).
    RequirementNotMet = 7,
    Exhausted = 8,
    Full = 9,
    NoTarget = 10,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldObjectOverlayEntry {
    pub content_generation: [u8; 32],
    pub placement: Vec<u8>,
    pub state: Vec<u8>,
    pub revision: u64,
}

fn push_varint(output: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        output.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    output.push(value as u8);
}

fn push_tag(output: &mut Vec<u8>, field: u64, wire: u64) {
    push_varint(output, (field << 3) | wire);
}

fn push_bytes_field(output: &mut Vec<u8>, field: u64, value: &[u8]) {
    push_tag(output, field, 2);
    push_varint(output, value.len() as u64);
    output.extend_from_slice(value);
}

fn push_varint_field(output: &mut Vec<u8>, field: u64, value: u64) {
    push_tag(output, field, 0);
    push_varint(output, value);
}

fn read_varint(input: &[u8], cursor: &mut usize) -> Result<u64, WorldObjectError> {
    let mut value = 0_u64;
    for shift in (0..70).step_by(7) {
        let byte = *input.get(*cursor).ok_or(WorldObjectError::Malformed)?;
        *cursor += 1;
        if shift == 63 && byte > 1 {
            return Err(WorldObjectError::Malformed);
        }
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(WorldObjectError::Malformed)
}

fn read_bytes<'a>(input: &'a [u8], cursor: &mut usize) -> Result<&'a [u8], WorldObjectError> {
    let len =
        usize::try_from(read_varint(input, cursor)?).map_err(|_| WorldObjectError::Malformed)?;
    let end = cursor
        .checked_add(len)
        .filter(|end| *end <= input.len())
        .ok_or(WorldObjectError::Malformed)?;
    let value = &input[*cursor..end];
    *cursor = end;
    Ok(value)
}

/// Reads exactly one singular enum field 1 (the only field of the use result).
fn read_single_enum(input: &[u8], maximum: usize) -> Result<u64, WorldObjectError> {
    if input.len() > maximum {
        return Err(WorldObjectError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut value = None;
    while cursor < input.len() {
        let key = read_varint(input, &mut cursor)?;
        if key != (1 << 3) || value.is_some() {
            return Err(WorldObjectError::Malformed);
        }
        value = Some(read_varint(input, &mut cursor)?);
    }
    value.ok_or(WorldObjectError::Malformed)
}

fn encode_world_object_target(target: &WorldObjectTarget) -> Result<Vec<u8>, WorldObjectError> {
    if target.placement.len() > MAX_KEY_BYTES {
        return Err(WorldObjectError::LimitExceeded);
    }
    let mut output = Vec::with_capacity(3 + target.placement.len() + 11);
    push_bytes_field(&mut output, 1, &target.placement);
    push_varint_field(&mut output, 2, target.expected_revision);
    Ok(output)
}

/// A standard proto3 encoder omits a scalar or bytes field holding its default value
/// (empty `placement`, `expected_revision` 0), so both fields default on omission;
/// semantic presence (e.g. a real object at its initial revision) is validated after
/// decoding, not inferred from wire-default omission (FND-02 §7).
fn decode_world_object_target(input: &[u8]) -> Result<WorldObjectTarget, WorldObjectError> {
    let mut cursor = 0;
    let (mut placement, mut expected_revision) = (None, None);
    while cursor < input.len() {
        let key = read_varint(input, &mut cursor)?;
        match key {
            0x0a if placement.is_none() => {
                let bytes = read_bytes(input, &mut cursor)?;
                if bytes.len() > MAX_KEY_BYTES {
                    return Err(WorldObjectError::LimitExceeded);
                }
                placement = Some(bytes.to_vec());
            }
            0x10 if expected_revision.is_none() => {
                expected_revision = Some(read_varint(input, &mut cursor)?);
            }
            _ => return Err(WorldObjectError::Malformed),
        }
    }
    Ok(WorldObjectTarget {
        placement: placement.unwrap_or_default(),
        expected_revision: expected_revision.unwrap_or(0),
    })
}

/// `ClientCommand.payload` of command type 2. Only the `world_object` oneof member is
/// registered; any other top-level field (the reserved 2, 3, 4) fails closed. Refuses to
/// emit a `placement` over `MAX_KEY_BYTES` rather than publish a payload a conforming
/// decoder would reject.
pub fn encode_use_intent(target: &WorldObjectTarget) -> Result<Vec<u8>, WorldObjectError> {
    let inner = encode_world_object_target(target)?;
    let mut output = Vec::with_capacity(3 + inner.len());
    push_bytes_field(&mut output, 1, &inner);
    Ok(output)
}

/// USE field 2, the item target: `ClientCommand.payload` of command type 2 for a session that
/// selected capability 4.
pub fn encode_use_item_intent(handle: ItemHandle) -> Vec<u8> {
    let inner = encode_item_target(handle);
    let mut output = Vec::with_capacity(2 + inner.len());
    push_bytes_field(&mut output, 2, &inner);
    output
}

/// `ClientCommand.payload` of command type 2 for a session that selected capability 15: field 2
/// or 5, then field 4 when present. Refuses what the decoder refuses: field 4 with a world-object
/// target, and a `use_with` whose generation is zero.
pub fn encode_item_use_intent(intent: &UseIntent) -> Result<Vec<u8>, WorldObjectError> {
    let mut output = match &intent.target {
        UseTarget::WorldObject(target) if intent.use_with.is_none() => encode_use_intent(target)?,
        UseTarget::WorldObject(_) => return Err(WorldObjectError::Malformed),
        UseTarget::Item(handle) => encode_use_item_intent(*handle),
        UseTarget::ItemByDefinition(index) => {
            let mut inner = Vec::with_capacity(6);
            push_varint_field(&mut inner, 1, u64::from(index.get()));
            let mut output = Vec::with_capacity(2 + inner.len());
            push_bytes_field(&mut output, 5, &inner);
            output
        }
    };
    if let Some(creature) = &intent.use_with {
        if creature.generation == 0 {
            return Err(WorldObjectError::Malformed);
        }
        let mut inner = Vec::with_capacity(MAX_USE_WITH_BYTES);
        push_bytes_field(&mut inner, 1, &creature.identity);
        push_varint_field(&mut inner, 2, creature.generation);
        push_bytes_field(&mut output, 4, &inner);
    }
    Ok(output)
}

/// The world-object target only: a session that did not select capability 4 sends no item target,
/// so field 2 fails closed here like the reserved field 3 and the capability 15 fields 4 and 5.
pub fn decode_use_intent(payload: &[u8]) -> Result<WorldObjectTarget, WorldObjectError> {
    match decode_use_intent_target(&[], payload)?.target {
        UseTarget::WorldObject(target) => Ok(target),
        UseTarget::Item(_) | UseTarget::ItemByDefinition(_) => Err(WorldObjectError::Malformed),
    }
}

/// `ItemByDefinitionV1`: a 1-based index, so zero (or an omitted field) names no item.
fn decode_item_by_definition(input: &[u8]) -> Result<NonZeroU32, WorldObjectError> {
    let mut cursor = 0;
    let mut index = None;
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            0x08 if index.is_none() => {
                let value = u32::try_from(read_varint(input, &mut cursor)?)
                    .map_err(|_| WorldObjectError::Malformed)?;
                index = Some(value);
            }
            _ => return Err(WorldObjectError::Malformed),
        }
    }
    NonZeroU32::new(index.unwrap_or(0)).ok_or(WorldObjectError::Malformed)
}

/// Field 4, an `EntityRefV1`: exactly 16 identity bytes and a non-zero generation.
fn decode_use_with(input: &[u8]) -> Result<EntityRef, WorldObjectError> {
    let mut cursor = 0;
    let (mut identity, mut generation) = (None, None);
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            0x0a if identity.is_none() => {
                let bytes: [u8; ENTITY_IDENTITY_BYTES] = read_bytes(input, &mut cursor)?
                    .try_into()
                    .map_err(|_| WorldObjectError::Malformed)?;
                identity = Some(bytes);
            }
            0x10 if generation.is_none() => generation = Some(read_varint(input, &mut cursor)?),
            _ => return Err(WorldObjectError::Malformed),
        }
    }
    Ok(EntityRef {
        identity: identity.ok_or(WorldObjectError::Malformed)?,
        generation: generation
            .filter(|generation| *generation != 0)
            .ok_or(WorldObjectError::Malformed)?,
    })
}

/// Exactly one target. Field 2 is accepted only when the session selected capability 4, and fields
/// 4 and 5 only when it selected capability 15 (ITEM-USE-0 §3); without them, and always for the
/// reserved field 3, the intent fails closed. Field 4 needs field 2 or 5: alone or with field 1 it
/// fails closed.
pub fn decode_use_intent_target(
    selected_capabilities: &[u32],
    payload: &[u8],
) -> Result<UseIntent, WorldObjectError> {
    if payload.len() > MAX_USE_INTENT_BYTES {
        return Err(WorldObjectError::LimitExceeded);
    }
    let item_targets = selected_capabilities.contains(&CAPABILITY_ITEM_VIEW_MOVE_V1);
    let item_use = item_targets && selected_capabilities.contains(&CAPABILITY_ITEM_USE_V1);
    let mut cursor = 0;
    let (mut target, mut use_with) = (None, None);
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        match key {
            0x22 if item_use && use_with.is_none() => {
                use_with = Some(decode_use_with(read_bytes(payload, &mut cursor)?)?);
            }
            0x2a if item_use && target.is_none() => {
                target = Some(UseTarget::ItemByDefinition(decode_item_by_definition(
                    read_bytes(payload, &mut cursor)?,
                )?));
            }
            0x0a if target.is_none() => {
                target = Some(UseTarget::WorldObject(decode_world_object_target(
                    read_bytes(payload, &mut cursor)?,
                )?));
            }
            0x12 if item_targets && target.is_none() => {
                let handle =
                    decode_item_target(read_bytes(payload, &mut cursor)?).map_err(|error| {
                        match error {
                            crate::item_view::ItemViewWireError::LimitExceeded => {
                                WorldObjectError::LimitExceeded
                            }
                            crate::item_view::ItemViewWireError::Malformed => {
                                WorldObjectError::Malformed
                            }
                        }
                    })?;
                target = Some(UseTarget::Item(handle));
            }
            _ => return Err(WorldObjectError::Malformed),
        }
    }
    let target = target.ok_or(WorldObjectError::Malformed)?;
    if use_with.is_some() && matches!(target, UseTarget::WorldObject(_)) {
        return Err(WorldObjectError::Malformed);
    }
    Ok(UseIntent { target, use_with })
}

pub fn encode_use_result(disposition: UseDisposition) -> Vec<u8> {
    let mut output = Vec::with_capacity(2);
    push_varint_field(&mut output, 1, disposition as u64);
    output
}

pub fn decode_use_result(payload: &[u8]) -> Result<UseDisposition, WorldObjectError> {
    match read_single_enum(payload, MAX_USE_RESULT_BYTES)? {
        1 => Ok(UseDisposition::Committed),
        2 => Ok(UseDisposition::NothingToUse),
        3 => Ok(UseDisposition::Occupied),
        4 => Ok(UseDisposition::StaleState),
        5 => Ok(UseDisposition::TooFar),
        6 => Ok(UseDisposition::Rejected),
        7 => Ok(UseDisposition::RequirementNotMet),
        8 => Ok(UseDisposition::Exhausted),
        9 => Ok(UseDisposition::Full),
        10 => Ok(UseDisposition::NoTarget),
        _ => Err(WorldObjectError::Malformed),
    }
}

/// Refuses to emit a `placement` or `state` over `MAX_KEY_BYTES` rather than publish a
/// payload a conforming decoder would reject.
fn encode_world_object_overlay_entry(
    entry: &WorldObjectOverlayEntry,
) -> Result<Vec<u8>, WorldObjectError> {
    if entry.placement.len() > MAX_KEY_BYTES || entry.state.len() > MAX_KEY_BYTES {
        return Err(WorldObjectError::LimitExceeded);
    }
    let mut output =
        Vec::with_capacity(34 + 3 + entry.placement.len() + 3 + entry.state.len() + 11);
    push_bytes_field(&mut output, 1, &entry.content_generation);
    push_bytes_field(&mut output, 2, &entry.placement);
    push_bytes_field(&mut output, 3, &entry.state);
    push_varint_field(&mut output, 4, entry.revision);
    Ok(output)
}

/// `content_generation` has no valid proto3 default here: it must be exactly 32 bytes, so
/// an absent value fails closed. `placement`, `state` and `revision` are ordinary proto3
/// scalar/bytes fields; a standard encoder omits them at their default (empty bytes / 0),
/// so all three default on omission. Semantic presence is validated after decoding, not
/// inferred from wire-default omission (FND-02 §7).
fn decode_world_object_overlay_entry(
    input: &[u8],
) -> Result<WorldObjectOverlayEntry, WorldObjectError> {
    let mut cursor = 0;
    let (mut content_generation, mut placement, mut state, mut revision) = (None, None, None, None);
    while cursor < input.len() {
        let key = read_varint(input, &mut cursor)?;
        match key {
            0x0a if content_generation.is_none() => {
                let bytes: [u8; CONTENT_GENERATION_BYTES] = read_bytes(input, &mut cursor)?
                    .try_into()
                    .map_err(|_| WorldObjectError::Malformed)?;
                content_generation = Some(bytes);
            }
            0x12 if placement.is_none() => {
                let bytes = read_bytes(input, &mut cursor)?;
                if bytes.len() > MAX_KEY_BYTES {
                    return Err(WorldObjectError::LimitExceeded);
                }
                placement = Some(bytes.to_vec());
            }
            0x1a if state.is_none() => {
                let bytes = read_bytes(input, &mut cursor)?;
                if bytes.len() > MAX_KEY_BYTES {
                    return Err(WorldObjectError::LimitExceeded);
                }
                state = Some(bytes.to_vec());
            }
            0x20 if revision.is_none() => {
                revision = Some(read_varint(input, &mut cursor)?);
            }
            _ => return Err(WorldObjectError::Malformed),
        }
    }
    let content_generation = content_generation.ok_or(WorldObjectError::Malformed)?;
    Ok(WorldObjectOverlayEntry {
        content_generation,
        placement: placement.unwrap_or_default(),
        state: state.unwrap_or_default(),
        revision: revision.unwrap_or(0),
    })
}

/// `StateDelta.payload` of domain 2, delta type 1: exactly one changed overlay entry
/// (`WOBJ-RL-02` = 1).
pub fn encode_world_object_overlay_delta(
    entry: &WorldObjectOverlayEntry,
) -> Result<Vec<u8>, WorldObjectError> {
    let inner = encode_world_object_overlay_entry(entry)?;
    let mut output = Vec::with_capacity(3 + inner.len());
    push_bytes_field(&mut output, 1, &inner);
    Ok(output)
}

pub fn decode_world_object_overlay_delta(
    payload: &[u8],
) -> Result<WorldObjectOverlayEntry, WorldObjectError> {
    if payload.len() > MAX_WORLD_OBJECT_OVERLAY_DELTA_BYTES {
        return Err(WorldObjectError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut entry = None;
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        match key {
            0x0a if entry.is_none() => {
                entry = Some(decode_world_object_overlay_entry(read_bytes(
                    payload,
                    &mut cursor,
                )?)?);
            }
            _ => return Err(WorldObjectError::Malformed),
        }
    }
    entry.ok_or(WorldObjectError::Malformed)
}

/// `StateDomainSnapshot.payload` of domain 2, snapshot type 1: every changed overlay in
/// scope, bounded by `WOBJ-RL-03` (`MAX_SNAPSHOT_ENTRIES`).
pub fn encode_world_object_overlay_snapshot(
    entries: &[WorldObjectOverlayEntry],
) -> Result<Vec<u8>, WorldObjectError> {
    if entries.len() > MAX_SNAPSHOT_ENTRIES {
        return Err(WorldObjectError::LimitExceeded);
    }
    let mut output = Vec::new();
    for entry in entries {
        let inner = encode_world_object_overlay_entry(entry)?;
        push_bytes_field(&mut output, 1, &inner);
    }
    if output.len() > MAX_WORLD_OBJECT_OVERLAY_SNAPSHOT_BYTES {
        return Err(WorldObjectError::LimitExceeded);
    }
    Ok(output)
}

pub fn decode_world_object_overlay_snapshot(
    payload: &[u8],
) -> Result<Vec<WorldObjectOverlayEntry>, WorldObjectError> {
    if payload.len() > MAX_WORLD_OBJECT_OVERLAY_SNAPSHOT_BYTES {
        return Err(WorldObjectError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut entries = Vec::new();
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        match key {
            0x0a => {
                if entries.len() >= MAX_SNAPSHOT_ENTRIES {
                    return Err(WorldObjectError::LimitExceeded);
                }
                entries.push(decode_world_object_overlay_entry(read_bytes(
                    payload,
                    &mut cursor,
                )?)?);
            }
            _ => return Err(WorldObjectError::Malformed),
        }
    }
    Ok(entries)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use serde_json::Value;

    const PROTOCOL_REGISTRY: &str =
        include_str!("../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");
    const RESOURCE_REGISTRY: &str =
        include_str!("../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json");

    fn target(placement: &[u8], expected_revision: u64) -> WorldObjectTarget {
        WorldObjectTarget {
            placement: placement.to_vec(),
            expected_revision,
        }
    }

    fn entry(placement: &[u8], state: &[u8], revision: u64) -> WorldObjectOverlayEntry {
        WorldObjectOverlayEntry {
            content_generation: [0xcd; 32],
            placement: placement.to_vec(),
            state: state.to_vec(),
            revision,
        }
    }

    #[test]
    fn use_intent_round_trips_and_requires_exactly_one_target() {
        for value in [
            target(b"door:1", 0),
            target(&[], 0),
            target(&vec![0xaa; MAX_KEY_BYTES], u64::MAX),
        ] {
            let bytes = encode_use_intent(&value).expect("encode");
            assert!(bytes.len() <= MAX_USE_INTENT_BYTES, "{}", bytes.len());
            assert_eq!(decode_use_intent(&bytes), Ok(value));
        }
        let valid = encode_use_intent(&target(b"door:1", 7)).expect("encode");
        for bad in [
            &[][..],           // missing target
            &[0x10, 0x01][..], // unknown top-level field
            &[0x0a][..],       // truncated
        ] {
            assert_eq!(decode_use_intent(bad), Err(WorldObjectError::Malformed));
        }
        // A repeated world_object field.
        let mut duplicate = valid.clone();
        duplicate.extend_from_slice(&valid);
        assert_eq!(
            decode_use_intent(&duplicate),
            Err(WorldObjectError::Malformed)
        );
        // Placement over the key bound.
        let mut oversized_placement = Vec::new();
        push_bytes_field(&mut oversized_placement, 1, &vec![0; MAX_KEY_BYTES + 1]);
        push_varint_field(&mut oversized_placement, 2, 0);
        let mut wrapped = Vec::new();
        push_bytes_field(&mut wrapped, 1, &oversized_placement);
        assert_eq!(
            decode_use_intent(&wrapped),
            Err(WorldObjectError::LimitExceeded)
        );
        // Reserved target field numbers (item/creature/use-with) fail closed.
        for reserved_field in [2_u64, 3, 4] {
            let mut reserved = Vec::new();
            push_bytes_field(&mut reserved, reserved_field, &[0x01]);
            assert_eq!(
                decode_use_intent(&reserved),
                Err(WorldObjectError::Malformed)
            );
        }
        assert_eq!(
            decode_use_intent(&vec![0; MAX_USE_INTENT_BYTES + 1]),
            Err(WorldObjectError::LimitExceeded)
        );
    }

    #[test]
    fn use_item_target_is_accepted_only_under_capability_4() {
        let selected = [CAPABILITY_ITEM_VIEW_MOVE_V1];
        for handle in [1_u64, u64::MAX] {
            let handle = ItemHandle::new(handle).expect("non-zero");
            let bytes = encode_use_item_intent(handle);
            assert!(bytes.len() <= MAX_USE_INTENT_BYTES);
            assert_eq!(
                decode_use_intent_target(&selected, &bytes),
                Ok(plain(UseTarget::Item(handle)))
            );
            // A session without capability 4 that sends field 2 fails closed.
            assert_eq!(
                decode_use_intent_target(&[], &bytes),
                Err(WorldObjectError::Malformed)
            );
            assert_eq!(decode_use_intent(&bytes), Err(WorldObjectError::Malformed));
        }
        let object = encode_use_intent(&target(b"door", 3)).expect("encode");
        assert_eq!(
            decode_use_intent_target(&selected, &object),
            Ok(plain(UseTarget::WorldObject(target(b"door", 3))))
        );
        let item = encode_use_item_intent(ItemHandle::MIN);
        let mut both = object.clone();
        both.extend_from_slice(&item);
        let mut twice = item.clone();
        twice.extend_from_slice(&item);
        let mut zero = Vec::new();
        push_bytes_field(&mut zero, 2, &[]);
        let mut unknown = Vec::new();
        push_bytes_field(&mut unknown, 2, &[0x10, 0x01]);
        let mut oversized = Vec::new();
        push_bytes_field(&mut oversized, 2, &[0; 12]);
        for (payload, error) in [
            (both, WorldObjectError::Malformed),
            (twice, WorldObjectError::Malformed),
            (zero, WorldObjectError::Malformed),
            (unknown, WorldObjectError::Malformed),
            (oversized, WorldObjectError::LimitExceeded),
        ] {
            assert_eq!(decode_use_intent_target(&selected, &payload), Err(error));
        }
        // The reserved field 3, and fields 4 and 5 without capability 15.
        for reserved_field in [3_u64, 4, 5] {
            let mut reserved = Vec::new();
            push_bytes_field(&mut reserved, reserved_field, &[0x08, 0x01]);
            assert_eq!(
                decode_use_intent_target(&selected, &reserved),
                Err(WorldObjectError::Malformed)
            );
        }
    }

    fn plain(target: UseTarget) -> UseIntent {
        UseIntent {
            target,
            use_with: None,
        }
    }

    fn creature(generation: u64) -> EntityRef {
        EntityRef {
            identity: [0x5c; ENTITY_IDENTITY_BYTES],
            generation,
        }
    }

    fn definition(index: u32) -> UseTarget {
        UseTarget::ItemByDefinition(NonZeroU32::new(index).expect("non-zero index"))
    }

    const ITEM_USE: [u32; 2] = [CAPABILITY_ITEM_VIEW_MOVE_V1, CAPABILITY_ITEM_USE_V1];

    #[test]
    fn fields_4_and_5_round_trip_under_capability_15() {
        let handle = ItemHandle::new(u64::MAX).expect("non-zero");
        for intent in [
            plain(UseTarget::Item(handle)),
            plain(definition(1)),
            plain(definition(u32::MAX)),
            UseIntent {
                target: UseTarget::Item(handle),
                use_with: Some(creature(u64::MAX)),
            },
            UseIntent {
                target: definition(u32::MAX),
                use_with: Some(creature(1)),
            },
        ] {
            let bytes = encode_item_use_intent(&intent).expect("encode");
            assert!(bytes.len() <= 44, "{intent:?}");
            assert_eq!(
                decode_use_intent_target(&ITEM_USE, &bytes),
                Ok(intent.clone())
            );
            // Without capability 15 (or without its requirement 4) fields 4 and 5 fail closed.
            let plain_item =
                matches!(intent.target, UseTarget::Item(_)) && intent.use_with.is_none();
            for selected in [
                &[CAPABILITY_ITEM_VIEW_MOVE_V1][..],
                &[CAPABILITY_ITEM_USE_V1],
            ] {
                if plain_item && selected == [CAPABILITY_ITEM_VIEW_MOVE_V1] {
                    // A plain field 2 stays valid under capability 4 alone.
                    assert_eq!(
                        decode_use_intent_target(selected, &bytes),
                        Ok(intent.clone())
                    );
                    continue;
                }
                assert_eq!(
                    decode_use_intent_target(selected, &bytes),
                    Err(WorldObjectError::Malformed),
                    "{selected:?} {bytes:?}"
                );
            }
        }
        // The largest intent with fields 4 and 5 is 44 bytes, far within ITEMUSE0-RL-03.
        let worst = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(handle),
            use_with: Some(creature(u64::MAX)),
        })
        .expect("encode");
        assert_eq!(worst.len(), 44);
        // A world-object target still round trips under capability 15.
        let object = encode_use_intent(&target(b"door", 3)).expect("encode");
        assert_eq!(
            decode_use_intent_target(&ITEM_USE, &object),
            Ok(plain(UseTarget::WorldObject(target(b"door", 3))))
        );
    }

    #[test]
    fn the_use_intent_is_bounded_at_529_bytes_under_capability_15() {
        // A world-object target at the bound: placement 512 bytes, revision u64::MAX.
        let at_bound = encode_use_intent(&target(&[0x61; 512], u64::MAX)).expect("encode");
        assert_eq!(at_bound.len(), MAX_USE_INTENT_BYTES);
        assert!(decode_use_intent_target(&ITEM_USE, &at_bound).is_ok());
        let mut over = at_bound;
        over.push(0);
        assert_eq!(
            decode_use_intent_target(&ITEM_USE, &over),
            Err(WorldObjectError::LimitExceeded)
        );
    }

    #[test]
    fn fields_4_and_5_fail_closed_when_misplaced_or_malformed() {
        let item = encode_use_item_intent(ItemHandle::MIN);
        let object = encode_use_intent(&target(b"door", 3)).expect("encode");
        let field = |number: u64, inner: &[u8]| {
            let mut output = Vec::new();
            push_bytes_field(&mut output, number, inner);
            output
        };
        let entity = |identity: &[u8], generation: Option<u64>| {
            let mut inner = Vec::new();
            push_bytes_field(&mut inner, 1, identity);
            if let Some(generation) = generation {
                push_varint_field(&mut inner, 2, generation);
            }
            field(4, &inner)
        };
        let good = entity(&[0x5c; 16], Some(7));
        let refused = |payload: Vec<u8>| {
            assert_eq!(
                decode_use_intent_target(&ITEM_USE, &payload),
                Err(WorldObjectError::Malformed),
                "{payload:?}"
            );
        };
        // Field 4 alone, with field 1, twice.
        refused(good.clone());
        refused([object.clone(), good.clone()].concat());
        refused([good.clone(), object.clone()].concat());
        refused([item.clone(), good.clone(), good.clone()].concat());
        // Field 4's creature: 15 or 17 identity bytes, generation 0 or missing, no identity,
        // an unknown field.
        refused([item.clone(), entity(&[0x5c; 15], Some(7))].concat());
        refused([item.clone(), entity(&[0x5c; 17], Some(7))].concat());
        refused([item.clone(), entity(&[0x5c; 16], Some(0))].concat());
        refused([item.clone(), entity(&[0x5c; 16], None)].concat());
        refused([item.clone(), field(4, &[0x10, 0x07])].concat());
        refused([item.clone(), field(4, &[0x10, 0x07, 0x18, 0x01])].concat());
        // Field 5: index 0 or omitted, over u32, repeated, an unknown field, twice, with field 1
        // or field 2.
        refused(field(5, &[0x08, 0x00]));
        refused(field(5, &[]));
        refused(field(5, &[0x08, 0x80, 0x80, 0x80, 0x80, 0x10]));
        refused(field(5, &[0x08, 0x01, 0x08, 0x01]));
        refused(field(5, &[0x10, 0x01]));
        refused([field(5, &[0x08, 0x01]), field(5, &[0x08, 0x01])].concat());
        refused([object.clone(), field(5, &[0x08, 0x01])].concat());
        refused([item.clone(), field(5, &[0x08, 0x01])].concat());
        // The reserved field 3.
        refused([item.clone(), field(3, &[0x08, 0x01])].concat());
        // Field 4 after its target in any order is accepted.
        assert_eq!(
            decode_use_intent_target(&ITEM_USE, &[good.clone(), item.clone()].concat()),
            Ok(UseIntent {
                target: UseTarget::Item(ItemHandle::MIN),
                use_with: Some(creature(7)),
            })
        );
        // The encoder refuses what the decoder refuses.
        assert_eq!(
            encode_item_use_intent(&UseIntent {
                target: UseTarget::WorldObject(target(b"door", 3)),
                use_with: Some(creature(7)),
            }),
            Err(WorldObjectError::Malformed)
        );
        assert_eq!(
            encode_item_use_intent(&UseIntent {
                target: UseTarget::Item(ItemHandle::MIN),
                use_with: Some(creature(0)),
            }),
            Err(WorldObjectError::Malformed)
        );
    }

    #[test]
    fn use_result_round_trips_and_refuses_zero_or_unknown() {
        for disposition in [
            UseDisposition::Committed,
            UseDisposition::NothingToUse,
            UseDisposition::Occupied,
            UseDisposition::StaleState,
            UseDisposition::TooFar,
            UseDisposition::Rejected,
            UseDisposition::RequirementNotMet,
            UseDisposition::Exhausted,
            UseDisposition::Full,
            UseDisposition::NoTarget,
        ] {
            let bytes = encode_use_result(disposition);
            assert!(bytes.len() <= MAX_USE_RESULT_BYTES);
            assert_eq!(decode_use_result(&bytes), Ok(disposition));
        }
        assert_eq!(
            decode_use_result(&[0x08, 0x00]),
            Err(WorldObjectError::Malformed)
        );
        assert_eq!(
            decode_use_result(&[0x08, 0x0b]),
            Err(WorldObjectError::Malformed)
        );
        assert_eq!(
            decode_use_result(&[0x08, 0x01, 0x08, 0x02]),
            Err(WorldObjectError::Malformed)
        );
        assert_eq!(
            decode_use_result(&[0x10, 0x01]),
            Err(WorldObjectError::Malformed)
        );
        assert_eq!(
            decode_use_result(&[0; MAX_USE_RESULT_BYTES + 1]),
            Err(WorldObjectError::LimitExceeded)
        );
    }

    #[test]
    fn omitted_proto3_defaults_are_accepted_but_required_fields_stay_malformed() {
        // A standard proto3 encoder omits `placement` (empty bytes) and
        // `expected_revision` (0) when both hold their default value; an object's
        // initial revision is 0 (world_runtime.rs), so this must decode, not fail
        // closed. Semantic presence is validated after decoding (FND-02 §7).
        let empty_inner = Vec::new();
        let mut wrapped = Vec::new();
        push_bytes_field(&mut wrapped, 1, &empty_inner);
        assert_eq!(
            decode_use_intent(&wrapped),
            Ok(WorldObjectTarget {
                placement: Vec::new(),
                expected_revision: 0,
            })
        );
        // Only `expected_revision` omitted (defaults to 0); `placement` present.
        let mut revision_only_omitted = Vec::new();
        push_bytes_field(&mut revision_only_omitted, 1, b"door:1");
        wrapped.clear();
        push_bytes_field(&mut wrapped, 1, &revision_only_omitted);
        assert_eq!(decode_use_intent(&wrapped), Ok(target(b"door:1", 0)));
        // Only `placement` omitted (defaults to empty); `expected_revision` present.
        let mut placement_only_omitted = Vec::new();
        push_varint_field(&mut placement_only_omitted, 2, 9);
        wrapped.clear();
        push_bytes_field(&mut wrapped, 1, &placement_only_omitted);
        assert_eq!(decode_use_intent(&wrapped), Ok(target(&[], 9)));
        // A missing oneof `world_object` (the field itself absent) still fails closed.
        assert_eq!(decode_use_intent(&[]), Err(WorldObjectError::Malformed));

        // The overlay entry: `placement`, `state` and `revision` default on omission,
        // but `content_generation` has no valid default and must still be present and
        // exactly 32 bytes.
        let mut generation_only = Vec::new();
        push_bytes_field(&mut generation_only, 1, &[0x11; 32]);
        let mut delta = Vec::new();
        push_bytes_field(&mut delta, 1, &generation_only);
        assert_eq!(
            decode_world_object_overlay_delta(&delta),
            Ok(WorldObjectOverlayEntry {
                content_generation: [0x11; 32],
                placement: Vec::new(),
                state: Vec::new(),
                revision: 0,
            })
        );
        // A missing `content_generation` still fails closed (no valid default).
        let mut no_generation = Vec::new();
        push_bytes_field(&mut no_generation, 2, b"door:1");
        let mut delta_no_generation = Vec::new();
        push_bytes_field(&mut delta_no_generation, 1, &no_generation);
        assert_eq!(
            decode_world_object_overlay_delta(&delta_no_generation),
            Err(WorldObjectError::Malformed)
        );
        // A missing delta entry (the repeated field carries none) still fails closed.
        assert_eq!(
            decode_world_object_overlay_delta(&[]),
            Err(WorldObjectError::Malformed)
        );
        // `disposition` 0 or absent still fails closed.
        assert_eq!(decode_use_result(&[]), Err(WorldObjectError::Malformed));
        assert_eq!(
            decode_use_result(&[0x08, 0x00]),
            Err(WorldObjectError::Malformed)
        );
    }

    #[test]
    fn encoders_refuse_to_emit_an_oversized_key() {
        // Exactly MAX_KEY_BYTES is accepted; MAX_KEY_BYTES + 1 is refused before
        // any bytes are emitted, so the server never publishes a payload a
        // conforming decoder would reject.
        assert!(encode_use_intent(&target(&vec![0; MAX_KEY_BYTES], 0)).is_ok());
        assert_eq!(
            encode_use_intent(&target(&vec![0; MAX_KEY_BYTES + 1], 0)),
            Err(WorldObjectError::LimitExceeded)
        );

        let ok_entry = entry(&vec![0; MAX_KEY_BYTES], &vec![0; MAX_KEY_BYTES], 0);
        assert!(encode_world_object_overlay_delta(&ok_entry).is_ok());
        assert!(encode_world_object_overlay_snapshot(&[ok_entry]).is_ok());

        let oversized_placement = entry(&vec![0; MAX_KEY_BYTES + 1], &[], 0);
        assert_eq!(
            encode_world_object_overlay_delta(&oversized_placement),
            Err(WorldObjectError::LimitExceeded)
        );
        assert_eq!(
            encode_world_object_overlay_snapshot(&[oversized_placement]),
            Err(WorldObjectError::LimitExceeded)
        );

        let oversized_state = entry(&[], &vec![0; MAX_KEY_BYTES + 1], 0);
        assert_eq!(
            encode_world_object_overlay_delta(&oversized_state),
            Err(WorldObjectError::LimitExceeded)
        );
        assert_eq!(
            encode_world_object_overlay_snapshot(&[oversized_state]),
            Err(WorldObjectError::LimitExceeded)
        );
    }

    #[test]
    fn overlay_delta_round_trips_exactly_one_entry_and_refuses_malformed() {
        for value in [
            entry(b"door:1", b"closed", 0),
            entry(&[], &[], 0),
            entry(
                &vec![0x11; MAX_KEY_BYTES],
                &vec![0x22; MAX_KEY_BYTES],
                u64::MAX,
            ),
        ] {
            let bytes = encode_world_object_overlay_delta(&value).expect("encode");
            assert!(
                bytes.len() <= MAX_WORLD_OBJECT_OVERLAY_DELTA_BYTES,
                "{}",
                bytes.len()
            );
            assert_eq!(decode_world_object_overlay_delta(&bytes), Ok(value));
        }
        let valid =
            encode_world_object_overlay_delta(&entry(b"door:1", b"closed", 1)).expect("encode");
        assert_eq!(
            decode_world_object_overlay_delta(&[]),
            Err(WorldObjectError::Malformed)
        );
        assert_eq!(
            decode_world_object_overlay_delta(&[0x10, 0x01]),
            Err(WorldObjectError::Malformed)
        );
        // A second entry (repeated field).
        let mut duplicate = valid.clone();
        duplicate.extend_from_slice(&valid);
        assert_eq!(
            decode_world_object_overlay_delta(&duplicate),
            Err(WorldObjectError::Malformed)
        );
        // Wrong content_generation width.
        let mut short_generation = Vec::new();
        push_bytes_field(&mut short_generation, 1, &[0; 31]);
        push_bytes_field(&mut short_generation, 2, b"door:1");
        push_bytes_field(&mut short_generation, 3, b"closed");
        push_varint_field(&mut short_generation, 4, 1);
        let mut wrapped = Vec::new();
        push_bytes_field(&mut wrapped, 1, &short_generation);
        assert_eq!(
            decode_world_object_overlay_delta(&wrapped),
            Err(WorldObjectError::Malformed)
        );
        assert_eq!(
            decode_world_object_overlay_delta(&vec![0; MAX_WORLD_OBJECT_OVERLAY_DELTA_BYTES + 1]),
            Err(WorldObjectError::LimitExceeded)
        );
    }

    #[test]
    fn overlay_snapshot_round_trips_within_bound_and_refuses_over_bound() {
        let entries: Vec<_> = (0..3)
            .map(|index| entry(format!("door:{index}").as_bytes(), b"closed", index as u64))
            .collect();
        let bytes = encode_world_object_overlay_snapshot(&entries).expect("encode");
        assert!(bytes.len() <= MAX_WORLD_OBJECT_OVERLAY_SNAPSHOT_BYTES);
        assert_eq!(
            decode_world_object_overlay_snapshot(&bytes),
            Ok(entries.clone())
        );
        // An empty snapshot (no changed overlays) is valid.
        let empty = encode_world_object_overlay_snapshot(&[]).expect("encode empty");
        assert_eq!(decode_world_object_overlay_snapshot(&empty), Ok(vec![]));

        // max entries accepted, max+1 refused.
        let max_entries: Vec<_> = (0..MAX_SNAPSHOT_ENTRIES)
            .map(|index| entry(format!("door:{index}").as_bytes(), b"c", index as u64))
            .collect();
        assert!(encode_world_object_overlay_snapshot(&max_entries).is_ok());
        let over_entries: Vec<_> = (0..=MAX_SNAPSHOT_ENTRIES)
            .map(|index| entry(format!("door:{index}").as_bytes(), b"c", index as u64))
            .collect();
        assert_eq!(
            encode_world_object_overlay_snapshot(&over_entries),
            Err(WorldObjectError::LimitExceeded)
        );

        assert_eq!(
            decode_world_object_overlay_snapshot(&[0x10, 0x01]),
            Err(WorldObjectError::Malformed)
        );
        assert_eq!(
            decode_world_object_overlay_snapshot(&vec![
                0;
                MAX_WORLD_OBJECT_OVERLAY_SNAPSHOT_BYTES + 1
            ]),
            Err(WorldObjectError::LimitExceeded)
        );
    }

    #[test]
    fn registries_bind_the_accepted_use_wire_ids_and_limits() {
        let protocol: Value = serde_json::from_str(PROTOCOL_REGISTRY).expect("protocol registry");
        let commands = protocol["command_types"].as_array().expect("command_types");
        let use_intent = commands
            .iter()
            .find(|command| command["name"] == "USE_INTENT")
            .expect("USE_INTENT registered");
        assert_eq!(use_intent["id"], COMMAND_TYPE_USE_INTENT);
        assert_eq!(use_intent["max_payload_bytes"], MAX_USE_INTENT_BYTES as u64);
        assert_eq!(
            use_intent["max_result_payload_bytes"],
            MAX_USE_RESULT_BYTES as u64
        );

        let domains = protocol["state_domains"].as_array().expect("state_domains");
        let overlay = domains
            .iter()
            .find(|domain| domain["name"] == "WORLD_OBJECT_OVERLAY")
            .expect("WORLD_OBJECT_OVERLAY registered");
        assert_eq!(overlay["id"], STATE_DOMAIN_WORLD_OBJECT_OVERLAY);
        assert_eq!(
            overlay["delta_types"][0]["id"],
            DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1
        );
        assert_eq!(
            overlay["delta_types"][0]["max_payload_bytes"],
            MAX_WORLD_OBJECT_OVERLAY_DELTA_BYTES as u64
        );
        assert_eq!(
            overlay["snapshot_types"][0]["id"],
            SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1
        );
        assert_eq!(
            overlay["snapshot_types"][0]["max_payload_bytes"],
            MAX_WORLD_OBJECT_OVERLAY_SNAPSHOT_BYTES as u64
        );

        let resources: Value = serde_json::from_str(RESOURCE_REGISTRY).expect("resource registry");
        let entries = resources["entries"].as_array().expect("entries");
        let limit = |id: &str| {
            entries
                .iter()
                .find(|entry| entry["id"] == id)
                .and_then(|entry| entry["hard_maximum"].as_u64())
        };
        assert_eq!(limit("WOBJ-RL-01"), Some(1));
        assert_eq!(limit("WOBJ-RL-02"), Some(1));
        assert_eq!(limit("WOBJ-RL-03"), Some(MAX_SNAPSHOT_ENTRIES as u64));
        assert_eq!(limit("ITEMUSE0-RL-03"), Some(MAX_USE_INTENT_BYTES as u64));

        // ITEM-USE-WIRE-1: capability 15 is registered, not offered, requires 4 and owns no
        // command type or domain.
        let capabilities = protocol["capabilities"].as_array().expect("capabilities");
        let matching: Vec<&Value> = capabilities
            .iter()
            .filter(|capability| capability["id"] == CAPABILITY_ITEM_USE_V1)
            .collect();
        assert_eq!(matching.len(), 1);
        let item_use = matching[0];
        assert_eq!(item_use["name"], "ITEM_USE_V1");
        assert_eq!(item_use["offered"], false);
        assert_eq!(
            item_use["requires"],
            serde_json::json!(CAPABILITY_ITEM_USE_V1_REQUIRES)
        );
        assert_eq!(item_use["command_types"], serde_json::json!([]));
        assert_eq!(item_use["state_domains"], serde_json::json!([]));
        assert!(crate::REGISTERED_CAPABILITY_IDS_V1.contains(&CAPABILITY_ITEM_USE_V1));
        let proto =
            include_str!("../../../docs/contracts/protocol-oteryn/v1/world_object_v1.proto");
        for line in [
            "ItemByDefinitionV1 item_by_definition = 5;",
            "oteryn.protocol.v1.world_spatial.EntityRefV1 use_with = 4;",
            "reserved 3;",
            "USE_DISPOSITION_REQUIREMENT_NOT_MET = 7;",
            "USE_DISPOSITION_EXHAUSTED = 8;",
            "USE_DISPOSITION_FULL = 9;",
            "USE_DISPOSITION_NO_TARGET = 10;",
        ] {
            assert!(proto.contains(line), "{line}");
        }
    }
}
