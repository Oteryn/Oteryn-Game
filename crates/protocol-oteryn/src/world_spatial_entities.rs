//! `MOVE-RL-11-VISIBILITY-V1` (VIS-2) typed payloads: the second schema revision of state domain 1
//! `WORLD_SPATIAL_VISIBILITY` (snapshot type 2, delta type 2), gated by the optional capability
//! 6 `WORLD_SPATIAL_ENTITIES`. Schema: `docs/contracts/protocol-oteryn/v1/world_spatial_v1.proto`.
//!
//! A session that did not select the capability keeps receiving the v1 types with its own actor
//! only (`world_spatial`). Decoding is strict: zero or unknown enum values, unknown, repeated or
//! kind-inconsistent fields, duplicate identities and over-bound payloads fail closed.
//!
//! ITEM-MOVE-WIRE-0 §4.1 (D212) adds field 10, the item handle of a corpse or ground item. It is
//! encoded and required only for a session that selected capability 4 `ITEM_VIEW_MOVE_V1` (the
//! `_with_item_handles` codecs); every other session receives the entry unchanged, without it.

use std::collections::BTreeSet;
use std::num::NonZeroU64;

use crate::item_view::{CAPABILITY_ITEM_VIEW_MOVE_V1, ItemHandle};

use crate::world_spatial::{
    ActorPosition, CONTENT_GENERATION_BYTES, StepDirection, WorldSpatialError,
    WorldSpatialObservation, decode_position, decode_world_spatial, encode_position, push_tag,
    push_varint, read_bytes, read_varint,
};

pub const CAPABILITY_WORLD_SPATIAL_ENTITIES: u32 = 6;
pub const DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2: u32 = 2;
pub const SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2: u32 = 2;

/// `MOVE-RL-11` (D87): entities in one snapshot, own actor included.
pub const MAX_SNAPSHOT_ENTITIES: usize = 256;
/// `MOVE-RL-08`: enter, update and leave entries in one delta; a larger change is a snapshot.
pub const MAX_DELTA_ENTRIES: usize = 256;
/// Decision 4.4: bytes of one encoded entity entry.
pub const MAX_ENTITY_ENTRY_BYTES: usize = 128;
/// Decision 4.4: 256 x 128 B + 1,024 B header, for both the snapshot and the delta.
pub const MAX_WORLD_SPATIAL_ENTITIES_PAYLOAD_BYTES: usize = 33_792;
pub const ENTITY_IDENTITY_BYTES: usize = 16;
const MAX_HEALTH_PERCENT: u8 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Player = 1,
    Creature = 2,
    Corpse = 3,
    GroundItem = 4,
    Npc = 5,
}

/// Runtime identity: the interest index identity bytes plus the actor generation (0 for objects).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EntityRef {
    pub identity: [u8; ENTITY_IDENTITY_BYTES],
    pub generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityDetail {
    /// Players, creatures and NPCs.
    Actor {
        direction: StepDirection,
        appearance_ref: u32,
        health_percent: u8,
    },
    /// Corpses and ground items; a stack shows its quantity (at least 1).
    Object {
        item_definition_ref: u32,
        quantity: u32,
        /// The session's handle for the item, sent only under capability 4.
        item_handle: Option<ItemHandle>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldSpatialEntity {
    pub kind: EntityKind,
    pub entity: EntityRef,
    pub position: ActorPosition,
    pub detail: EntityDetail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldSpatialEntitiesSnapshot {
    pub content_generation: [u8; CONTENT_GENERATION_BYTES],
    pub actor_position: ActorPosition,
    /// The session's own player; it must appear in `entities` at `actor_position`.
    pub own_identity: [u8; ENTITY_IDENTITY_BYTES],
    pub entities: Vec<WorldSpatialEntity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldSpatialEntitiesDelta {
    pub content_generation: [u8; CONTENT_GENERATION_BYTES],
    pub actor_position: ActorPosition,
    pub enter: Vec<WorldSpatialEntity>,
    pub update: Vec<WorldSpatialEntity>,
    pub leave: Vec<EntityRef>,
}

fn actor_kind(kind: EntityKind) -> bool {
    matches!(
        kind,
        EntityKind::Player | EntityKind::Creature | EntityKind::Npc
    )
}

fn validate_entity(entity: &WorldSpatialEntity) -> Result<(), WorldSpatialError> {
    match entity.detail {
        EntityDetail::Actor { health_percent, .. }
            if actor_kind(entity.kind)
                && health_percent <= MAX_HEALTH_PERCENT
                && (entity.kind != EntityKind::Npc || health_percent == MAX_HEALTH_PERCENT) =>
        {
            Ok(())
        }
        EntityDetail::Object {
            item_definition_ref,
            quantity,
            ..
        } if !actor_kind(entity.kind)
            && entity.entity.generation == 0
            && item_definition_ref != 0
            && quantity != 0 =>
        {
            Ok(())
        }
        _ => Err(WorldSpatialError::Malformed),
    }
}

fn encode_ref(entity: &EntityRef) -> Vec<u8> {
    let mut output = Vec::with_capacity(ENTITY_IDENTITY_BYTES + 14);
    push_tag(&mut output, 1, 2);
    push_varint(&mut output, ENTITY_IDENTITY_BYTES as u64);
    output.extend_from_slice(&entity.identity);
    if entity.generation != 0 {
        push_tag(&mut output, 2, 0);
        push_varint(&mut output, entity.generation);
    }
    output
}

/// With `item_handles` an object must carry its handle, which is encoded as field 10; without it
/// the handle is left out and the entry is the unchanged D85 entry.
fn encode_entity(
    entity: &WorldSpatialEntity,
    item_handles: bool,
) -> Result<Vec<u8>, WorldSpatialError> {
    validate_entity(entity)?;
    let mut output = Vec::with_capacity(96);
    push_tag(&mut output, 1, 0);
    push_varint(&mut output, entity.kind as u64);
    push_tag(&mut output, 2, 2);
    push_varint(&mut output, ENTITY_IDENTITY_BYTES as u64);
    output.extend_from_slice(&entity.entity.identity);
    if entity.entity.generation != 0 {
        push_tag(&mut output, 3, 0);
        push_varint(&mut output, entity.entity.generation);
    }
    let position = encode_position(&entity.position);
    push_tag(&mut output, 4, 2);
    push_varint(&mut output, position.len() as u64);
    output.extend_from_slice(&position);
    match entity.detail {
        EntityDetail::Actor {
            direction,
            appearance_ref,
            health_percent,
        } => {
            push_tag(&mut output, 5, 0);
            push_varint(&mut output, direction as u64);
            for (field, value) in [(6, appearance_ref), (7, u32::from(health_percent))] {
                if value != 0 {
                    push_tag(&mut output, field, 0);
                    push_varint(&mut output, u64::from(value));
                }
            }
        }
        EntityDetail::Object {
            item_definition_ref,
            quantity,
            item_handle,
        } => {
            push_tag(&mut output, 8, 0);
            push_varint(&mut output, u64::from(item_definition_ref));
            push_tag(&mut output, 9, 0);
            push_varint(&mut output, u64::from(quantity));
            if item_handles {
                let handle = item_handle.ok_or(WorldSpatialError::Malformed)?;
                push_tag(&mut output, 10, 0);
                push_varint(&mut output, handle.get());
            }
        }
    }
    if output.len() > MAX_ENTITY_ENTRY_BYTES {
        return Err(WorldSpatialError::LimitExceeded);
    }
    Ok(output)
}

fn once<T>(slot: &mut Option<T>, value: T) -> Result<(), WorldSpatialError> {
    if slot.replace(value).is_some() {
        return Err(WorldSpatialError::Malformed);
    }
    Ok(())
}

fn identity_bytes(input: &[u8]) -> Result<[u8; ENTITY_IDENTITY_BYTES], WorldSpatialError> {
    input.try_into().map_err(|_| WorldSpatialError::Malformed)
}

fn varint_u32(input: &[u8], cursor: &mut usize) -> Result<u32, WorldSpatialError> {
    u32::try_from(read_varint(input, cursor)?).map_err(|_| WorldSpatialError::Malformed)
}

fn decode_ref(input: &[u8]) -> Result<EntityRef, WorldSpatialError> {
    let mut cursor = 0;
    let (mut identity, mut generation) = (None, None);
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            0x0a => once(
                &mut identity,
                identity_bytes(read_bytes(input, &mut cursor)?)?,
            )?,
            0x10 => once(&mut generation, read_varint(input, &mut cursor)?)?,
            _ => return Err(WorldSpatialError::Malformed),
        }
    }
    Ok(EntityRef {
        identity: identity.ok_or(WorldSpatialError::Malformed)?,
        generation: generation.unwrap_or(0),
    })
}

/// Field 10 is accepted only with `item_handles`, and then required on objects and refused on
/// actors.
fn decode_entity(
    input: &[u8],
    item_handles: bool,
) -> Result<WorldSpatialEntity, WorldSpatialError> {
    if input.len() > MAX_ENTITY_ENTRY_BYTES {
        return Err(WorldSpatialError::LimitExceeded);
    }
    let mut cursor = 0;
    let (mut kind, mut identity, mut generation, mut position) = (None, None, None, None);
    let (mut direction, mut appearance, mut health) = (None, None, None);
    let (mut item, mut quantity, mut handle) = (None, None, None);
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            0x08 => once(&mut kind, varint_u32(input, &mut cursor)?)?,
            0x12 => once(
                &mut identity,
                identity_bytes(read_bytes(input, &mut cursor)?)?,
            )?,
            0x18 => once(&mut generation, read_varint(input, &mut cursor)?)?,
            0x22 => once(
                &mut position,
                decode_position(read_bytes(input, &mut cursor)?)?,
            )?,
            0x28 => once(&mut direction, varint_u32(input, &mut cursor)?)?,
            0x30 => once(&mut appearance, varint_u32(input, &mut cursor)?)?,
            0x38 => once(&mut health, varint_u32(input, &mut cursor)?)?,
            0x40 => once(&mut item, varint_u32(input, &mut cursor)?)?,
            0x48 => once(&mut quantity, varint_u32(input, &mut cursor)?)?,
            0x50 if item_handles => once(&mut handle, read_varint(input, &mut cursor)?)?,
            _ => return Err(WorldSpatialError::Malformed),
        }
    }
    let kind = match kind {
        Some(1) => EntityKind::Player,
        Some(2) => EntityKind::Creature,
        Some(3) => EntityKind::Corpse,
        Some(4) => EntityKind::GroundItem,
        Some(5) => EntityKind::Npc,
        _ => return Err(WorldSpatialError::Malformed),
    };
    let detail = if actor_kind(kind) {
        if item.is_some() || quantity.is_some() || handle.is_some() {
            return Err(WorldSpatialError::Malformed);
        }
        let direction = match direction {
            Some(1) => StepDirection::North,
            Some(2) => StepDirection::East,
            Some(3) => StepDirection::South,
            Some(4) => StepDirection::West,
            _ => return Err(WorldSpatialError::Malformed),
        };
        EntityDetail::Actor {
            direction,
            appearance_ref: appearance.unwrap_or(0),
            health_percent: u8::try_from(health.unwrap_or(0))
                .map_err(|_| WorldSpatialError::Malformed)?,
        }
    } else {
        if direction.is_some() || appearance.is_some() || health.is_some() {
            return Err(WorldSpatialError::Malformed);
        }
        let item_handle = match handle {
            Some(handle) => Some(NonZeroU64::new(handle).ok_or(WorldSpatialError::Malformed)?),
            None if item_handles => return Err(WorldSpatialError::Malformed),
            None => None,
        };
        EntityDetail::Object {
            item_definition_ref: item.ok_or(WorldSpatialError::Malformed)?,
            quantity: quantity.ok_or(WorldSpatialError::Malformed)?,
            item_handle,
        }
    };
    let entity = WorldSpatialEntity {
        kind,
        entity: EntityRef {
            identity: identity.ok_or(WorldSpatialError::Malformed)?,
            generation: generation.unwrap_or(0),
        },
        position: position.ok_or(WorldSpatialError::Malformed)?,
        detail,
    };
    validate_entity(&entity)?;
    Ok(entity)
}

fn push_message(output: &mut Vec<u8>, field: u64, body: &[u8]) {
    push_tag(output, field, 2);
    push_varint(output, body.len() as u64);
    output.extend_from_slice(body);
}

fn push_header(
    output: &mut Vec<u8>,
    content_generation: &[u8; CONTENT_GENERATION_BYTES],
    actor_position: &ActorPosition,
) {
    push_message(output, 1, content_generation);
    push_message(output, 2, &encode_position(actor_position));
}

fn unique_identities<'a>(
    refs: impl Iterator<Item = &'a EntityRef>,
) -> Result<(), WorldSpatialError> {
    let mut seen = BTreeSet::new();
    for reference in refs {
        if !seen.insert(reference.identity) {
            return Err(WorldSpatialError::Malformed);
        }
    }
    Ok(())
}

/// ITEM-MOVE-WIRE-0 §4.1 (#1703 P2 4175400425): a handle names one item, so two objects of one
/// snapshot or delta never carry the same handle.
fn unique_item_handles<'a>(
    entities: impl Iterator<Item = &'a WorldSpatialEntity>,
) -> Result<(), WorldSpatialError> {
    let mut seen = BTreeSet::new();
    for entity in entities {
        if let EntityDetail::Object {
            item_handle: Some(handle),
            ..
        } = entity.detail
            && !seen.insert(handle)
        {
            return Err(WorldSpatialError::Malformed);
        }
    }
    Ok(())
}

pub fn validate_snapshot(snapshot: &WorldSpatialEntitiesSnapshot) -> Result<(), WorldSpatialError> {
    if snapshot.entities.len() > MAX_SNAPSHOT_ENTITIES {
        return Err(WorldSpatialError::LimitExceeded);
    }
    unique_identities(snapshot.entities.iter().map(|entity| &entity.entity))?;
    unique_item_handles(snapshot.entities.iter())?;
    // The own actor is always included, at the position the header states.
    snapshot
        .entities
        .iter()
        .any(|entity| {
            entity.kind == EntityKind::Player
                && entity.entity.identity == snapshot.own_identity
                && entity.position == snapshot.actor_position
        })
        .then_some(())
        .ok_or(WorldSpatialError::Malformed)
}

pub fn validate_delta(delta: &WorldSpatialEntitiesDelta) -> Result<(), WorldSpatialError> {
    if delta.enter.len() + delta.update.len() + delta.leave.len() > MAX_DELTA_ENTRIES {
        return Err(WorldSpatialError::LimitExceeded);
    }
    unique_identities(
        delta
            .enter
            .iter()
            .chain(&delta.update)
            .map(|entity| &entity.entity)
            .chain(&delta.leave),
    )?;
    unique_item_handles(delta.enter.iter().chain(&delta.update))
}

pub fn encode_world_spatial_entities_snapshot(
    snapshot: &WorldSpatialEntitiesSnapshot,
) -> Result<Vec<u8>, WorldSpatialError> {
    encode_snapshot(snapshot, false)
}

/// The snapshot for a session that selected capability 4: every object carries its item handle.
pub fn encode_world_spatial_entities_snapshot_with_item_handles(
    snapshot: &WorldSpatialEntitiesSnapshot,
) -> Result<Vec<u8>, WorldSpatialError> {
    encode_snapshot(snapshot, true)
}

fn encode_snapshot(
    snapshot: &WorldSpatialEntitiesSnapshot,
    item_handles: bool,
) -> Result<Vec<u8>, WorldSpatialError> {
    validate_snapshot(snapshot)?;
    let mut output = Vec::new();
    push_header(
        &mut output,
        &snapshot.content_generation,
        &snapshot.actor_position,
    );
    push_message(&mut output, 3, &snapshot.own_identity);
    for entity in &snapshot.entities {
        push_message(&mut output, 4, &encode_entity(entity, item_handles)?);
    }
    Ok(output)
}

pub fn encode_world_spatial_entities_delta(
    delta: &WorldSpatialEntitiesDelta,
) -> Result<Vec<u8>, WorldSpatialError> {
    encode_delta(delta, false)
}

/// The delta for a session that selected capability 4: every object carries its item handle.
pub fn encode_world_spatial_entities_delta_with_item_handles(
    delta: &WorldSpatialEntitiesDelta,
) -> Result<Vec<u8>, WorldSpatialError> {
    encode_delta(delta, true)
}

fn encode_delta(
    delta: &WorldSpatialEntitiesDelta,
    item_handles: bool,
) -> Result<Vec<u8>, WorldSpatialError> {
    validate_delta(delta)?;
    let mut output = Vec::new();
    push_header(
        &mut output,
        &delta.content_generation,
        &delta.actor_position,
    );
    for (field, list) in [(3, &delta.enter), (4, &delta.update)] {
        for entity in list {
            push_message(&mut output, field, &encode_entity(entity, item_handles)?);
        }
    }
    for reference in &delta.leave {
        push_message(&mut output, 5, &encode_ref(reference));
    }
    Ok(output)
}

type Header = (
    Option<[u8; CONTENT_GENERATION_BYTES]>,
    Option<ActorPosition>,
);

fn decode_header_field(
    key: u64,
    payload: &[u8],
    cursor: &mut usize,
    header: &mut Header,
) -> Result<bool, WorldSpatialError> {
    match key {
        0x0a => {
            let bytes = read_bytes(payload, cursor)?
                .try_into()
                .map_err(|_| WorldSpatialError::Malformed)?;
            once(&mut header.0, bytes)?;
        }
        0x12 => once(
            &mut header.1,
            decode_position(read_bytes(payload, cursor)?)?,
        )?,
        _ => return Ok(false),
    }
    Ok(true)
}

pub fn decode_world_spatial_entities_snapshot(
    payload: &[u8],
) -> Result<WorldSpatialEntitiesSnapshot, WorldSpatialError> {
    decode_snapshot(payload, false)
}

pub fn decode_world_spatial_entities_snapshot_with_item_handles(
    payload: &[u8],
) -> Result<WorldSpatialEntitiesSnapshot, WorldSpatialError> {
    decode_snapshot(payload, true)
}

fn decode_snapshot(
    payload: &[u8],
    item_handles: bool,
) -> Result<WorldSpatialEntitiesSnapshot, WorldSpatialError> {
    if payload.len() > MAX_WORLD_SPATIAL_ENTITIES_PAYLOAD_BYTES {
        return Err(WorldSpatialError::LimitExceeded);
    }
    let (mut cursor, mut header, mut own) = (0, (None, None), None);
    let mut entities = Vec::new();
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        if decode_header_field(key, payload, &mut cursor, &mut header)? {
            continue;
        }
        match key {
            0x1a => once(&mut own, identity_bytes(read_bytes(payload, &mut cursor)?)?)?,
            0x22 => {
                if entities.len() == MAX_SNAPSHOT_ENTITIES {
                    return Err(WorldSpatialError::LimitExceeded);
                }
                entities.push(decode_entity(
                    read_bytes(payload, &mut cursor)?,
                    item_handles,
                )?);
            }
            _ => return Err(WorldSpatialError::Malformed),
        }
    }
    let snapshot = WorldSpatialEntitiesSnapshot {
        content_generation: header.0.ok_or(WorldSpatialError::Malformed)?,
        actor_position: header.1.ok_or(WorldSpatialError::Malformed)?,
        own_identity: own.ok_or(WorldSpatialError::Malformed)?,
        entities,
    };
    validate_snapshot(&snapshot)?;
    Ok(snapshot)
}

pub fn decode_world_spatial_entities_delta(
    payload: &[u8],
) -> Result<WorldSpatialEntitiesDelta, WorldSpatialError> {
    decode_delta(payload, false)
}

pub fn decode_world_spatial_entities_delta_with_item_handles(
    payload: &[u8],
) -> Result<WorldSpatialEntitiesDelta, WorldSpatialError> {
    decode_delta(payload, true)
}

fn decode_delta(
    payload: &[u8],
    item_handles: bool,
) -> Result<WorldSpatialEntitiesDelta, WorldSpatialError> {
    if payload.len() > MAX_WORLD_SPATIAL_ENTITIES_PAYLOAD_BYTES {
        return Err(WorldSpatialError::LimitExceeded);
    }
    let (mut cursor, mut header) = (0, (None, None));
    let (mut enter, mut update, mut leave) = (Vec::new(), Vec::new(), Vec::new());
    let mut entries = 0_usize;
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        if decode_header_field(key, payload, &mut cursor, &mut header)? {
            continue;
        }
        if !matches!(key, 0x1a | 0x22 | 0x2a) {
            return Err(WorldSpatialError::Malformed);
        }
        entries += 1;
        if entries > MAX_DELTA_ENTRIES {
            return Err(WorldSpatialError::LimitExceeded);
        }
        let body = read_bytes(payload, &mut cursor)?;
        match key {
            0x1a => enter.push(decode_entity(body, item_handles)?),
            0x22 => update.push(decode_entity(body, item_handles)?),
            _ => leave.push(decode_ref(body)?),
        }
    }
    let delta = WorldSpatialEntitiesDelta {
        content_generation: header.0.ok_or(WorldSpatialError::Malformed)?,
        actor_position: header.1.ok_or(WorldSpatialError::Malformed)?,
        enter,
        update,
        leave,
    };
    validate_delta(&delta)?;
    Ok(delta)
}

/// What a client decoded from a domain-1 snapshot, by the revision its negotiation selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldSpatialSnapshotView {
    OwnActor(WorldSpatialObservation),
    Entities(WorldSpatialEntitiesSnapshot),
}

/// What a client decoded from a domain-1 delta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldSpatialDeltaView {
    OwnActor(WorldSpatialObservation),
    Entities(WorldSpatialEntitiesDelta),
}

/// Client decode of a domain-1 snapshot. Type 2 is accepted only when the session selected
/// capability 6; otherwise it is an unnegotiated type and fails closed. Its objects carry item
/// handles exactly when the session also selected capability 4.
pub fn decode_world_spatial_snapshot_view(
    selected_capabilities: &[u32],
    snapshot_type: u32,
    payload: &[u8],
) -> Result<WorldSpatialSnapshotView, WorldSpatialError> {
    match snapshot_type {
        crate::world_spatial::SNAPSHOT_TYPE_WORLD_SPATIAL_V1 => {
            decode_world_spatial(payload).map(WorldSpatialSnapshotView::OwnActor)
        }
        SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2
            if selected_capabilities.contains(&CAPABILITY_WORLD_SPATIAL_ENTITIES) =>
        {
            decode_snapshot(
                payload,
                selected_capabilities.contains(&CAPABILITY_ITEM_VIEW_MOVE_V1),
            )
            .map(WorldSpatialSnapshotView::Entities)
        }
        _ => Err(WorldSpatialError::Malformed),
    }
}

/// Client decode of a domain-1 delta, gated like the snapshot.
pub fn decode_world_spatial_delta_view(
    selected_capabilities: &[u32],
    delta_type: u32,
    payload: &[u8],
) -> Result<WorldSpatialDeltaView, WorldSpatialError> {
    match delta_type {
        crate::world_spatial::DELTA_TYPE_WORLD_SPATIAL_V1 => {
            decode_world_spatial(payload).map(WorldSpatialDeltaView::OwnActor)
        }
        DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2
            if selected_capabilities.contains(&CAPABILITY_WORLD_SPATIAL_ENTITIES) =>
        {
            decode_delta(
                payload,
                selected_capabilities.contains(&CAPABILITY_ITEM_VIEW_MOVE_V1),
            )
            .map(WorldSpatialDeltaView::Entities)
        }
        _ => Err(WorldSpatialError::Malformed),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::world_spatial::{
        DELTA_TYPE_WORLD_SPATIAL_V1, SNAPSHOT_TYPE_WORLD_SPATIAL_V1, encode_world_spatial,
    };
    use serde_json::Value;

    const PROTOCOL_REGISTRY: &str =
        include_str!("../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");
    const RESOURCE_REGISTRY: &str =
        include_str!("../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json");

    fn id(n: u32) -> EntityRef {
        let mut identity = [0_u8; ENTITY_IDENTITY_BYTES];
        identity[12..].copy_from_slice(&n.to_be_bytes());
        EntityRef {
            identity,
            generation: 0,
        }
    }

    fn at(x: i32) -> ActorPosition {
        ActorPosition { x, y: 7, floor: 7 }
    }

    fn actor(kind: EntityKind, n: u32) -> WorldSpatialEntity {
        WorldSpatialEntity {
            kind,
            entity: EntityRef {
                generation: u64::MAX,
                ..id(n)
            },
            position: at(n as i32),
            detail: EntityDetail::Actor {
                direction: StepDirection::West,
                appearance_ref: u32::MAX,
                health_percent: 100,
            },
        }
    }

    fn object(kind: EntityKind, n: u32) -> WorldSpatialEntity {
        WorldSpatialEntity {
            kind,
            entity: id(n),
            position: at(n as i32),
            detail: EntityDetail::Object {
                item_definition_ref: u32::MAX,
                quantity: u32::MAX,
                item_handle: None,
            },
        }
    }

    fn snapshot(entities: Vec<WorldSpatialEntity>) -> WorldSpatialEntitiesSnapshot {
        WorldSpatialEntitiesSnapshot {
            content_generation: [0xab; 32],
            actor_position: at(0),
            own_identity: id(0).identity,
            entities,
        }
    }

    fn own() -> WorldSpatialEntity {
        let mut own = actor(EntityKind::Player, 0);
        own.position = at(0);
        own
    }

    fn delta(n_enter: usize, n_update: usize, n_leave: usize) -> WorldSpatialEntitiesDelta {
        let mut next = 1_u32;
        let mut take = |count: usize| {
            (0..count)
                .map(|_| {
                    next += 1;
                    next
                })
                .collect::<Vec<_>>()
        };
        WorldSpatialEntitiesDelta {
            content_generation: [0xab; 32],
            actor_position: at(0),
            enter: take(n_enter)
                .into_iter()
                .map(|n| actor(EntityKind::Creature, n))
                .collect(),
            update: take(n_update)
                .into_iter()
                .map(|n| object(EntityKind::Corpse, n))
                .collect(),
            leave: take(n_leave).into_iter().map(id).collect(),
        }
    }

    #[test]
    fn npc_is_an_actor_entry_within_the_entry_bound_and_never_an_object() {
        let npc = actor(EntityKind::Npc, 9);
        let entry = encode_entity(&npc, false).expect("entry");
        assert!(entry.len() <= MAX_ENTITY_ENTRY_BYTES);
        let mut object_npc = object(EntityKind::Npc, 9);
        assert!(encode_entity(&object_npc, false).is_err());
        object_npc.detail = npc.detail;
        assert!(encode_entity(&object_npc, false).is_ok());
        for health_percent in [0, 99, 101] {
            let mut bad = actor(EntityKind::Npc, 9);
            bad.detail = EntityDetail::Actor {
                direction: StepDirection::West,
                appearance_ref: 1,
                health_percent,
            };
            assert!(encode_entity(&bad, false).is_err(), "{health_percent}");
        }
        // Decode: field 7 below 100 or omitted is rejected; a creature still accepts below 100.
        let wire = |health: Option<u8>, kind: EntityKind| {
            let mut e = actor(kind, 9);
            e.detail = EntityDetail::Actor {
                direction: StepDirection::West,
                appearance_ref: 1,
                health_percent: 100,
            };
            let mut bytes = encode_entity(&e, false).expect("entry");
            let at = bytes
                .windows(2)
                .position(|w| w == [0x38, 100])
                .expect("health");
            bytes.drain(at..at + 2);
            if let Some(h) = health {
                bytes.extend_from_slice(&[0x38, h]);
            }
            bytes
        };
        for health in [Some(99), Some(0), None] {
            assert!(decode_entity(&wire(health, EntityKind::Npc), false).is_err());
        }
        assert!(decode_entity(&wire(Some(100), EntityKind::Npc), false).is_ok());
        assert!(decode_entity(&wire(Some(99), EntityKind::Creature), false).is_ok());
    }

    #[test]
    fn snapshot_round_trips_every_kind_and_256_maximum_entries_fit_the_bound() {
        let mixed = snapshot(vec![
            own(),
            actor(EntityKind::Creature, 1),
            object(EntityKind::Corpse, 2),
            object(EntityKind::GroundItem, 3),
            actor(EntityKind::Npc, 4),
        ]);
        let bytes = encode_world_spatial_entities_snapshot(&mixed).expect("encode");
        assert_eq!(decode_world_spatial_entities_snapshot(&bytes), Ok(mixed));

        let mut entities = vec![own()];
        entities.extend((1..256).map(|n| actor(EntityKind::Creature, n)));
        let full = snapshot(entities);
        let bytes = encode_world_spatial_entities_snapshot(&full).expect("256 entities");
        assert!(
            bytes.len() <= MAX_WORLD_SPATIAL_ENTITIES_PAYLOAD_BYTES,
            "{}",
            bytes.len()
        );
        assert_eq!(
            decode_world_spatial_entities_snapshot(&bytes),
            Ok(full.clone())
        );
        // The largest single entry (all varints at maximum) stays within 128 B.
        assert!(
            encode_entity(&full.entities[1], false)
                .expect("entry")
                .len()
                <= MAX_ENTITY_ENTRY_BYTES
        );

        // max + 1 is refused on encode and on decode.
        let mut over = full;
        over.entities.push(actor(EntityKind::Creature, 256));
        assert_eq!(
            encode_world_spatial_entities_snapshot(&over),
            Err(WorldSpatialError::LimitExceeded)
        );
        let mut raw = bytes;
        let entry = encode_entity(&actor(EntityKind::Creature, 256), false).expect("entry");
        push_message(&mut raw, 4, &entry);
        assert_eq!(
            decode_world_spatial_entities_snapshot(&raw),
            Err(WorldSpatialError::LimitExceeded)
        );
    }

    #[test]
    fn snapshot_requires_the_own_actor_and_unique_identities() {
        for bad in [
            snapshot(vec![actor(EntityKind::Creature, 1)]),
            snapshot(vec![actor(EntityKind::Creature, 0)]), // own identity as a creature
            snapshot(vec![own(), own()]),
            snapshot(vec![WorldSpatialEntity {
                position: at(9),
                ..own()
            }]),
        ] {
            assert_eq!(
                encode_world_spatial_entities_snapshot(&bad),
                Err(WorldSpatialError::Malformed)
            );
        }
    }

    #[test]
    fn delta_round_trips_and_bounds_enter_update_leave_at_256_total() {
        let small = delta(2, 1, 3);
        let bytes = encode_world_spatial_entities_delta(&small).expect("encode");
        assert_eq!(decode_world_spatial_entities_delta(&bytes), Ok(small));

        let full = delta(100, 100, 56);
        let bytes = encode_world_spatial_entities_delta(&full).expect("256 entries");
        assert!(bytes.len() <= MAX_WORLD_SPATIAL_ENTITIES_PAYLOAD_BYTES);
        assert_eq!(decode_world_spatial_entities_delta(&bytes), Ok(full));

        assert_eq!(
            encode_world_spatial_entities_delta(&delta(100, 100, 57)),
            Err(WorldSpatialError::LimitExceeded)
        );
        let mut raw = bytes;
        push_message(&mut raw, 5, &encode_ref(&id(999)));
        assert_eq!(
            decode_world_spatial_entities_delta(&raw),
            Err(WorldSpatialError::LimitExceeded)
        );
        // An identity appearing twice, even across lists, is refused.
        let mut twice = delta(1, 0, 0);
        twice.leave.push(twice.enter[0].entity);
        assert_eq!(
            encode_world_spatial_entities_delta(&twice),
            Err(WorldSpatialError::Malformed)
        );
    }

    #[test]
    fn decoders_fail_closed_on_malformed_entries() {
        let valid = encode_entity(&actor(EntityKind::Player, 1), false).expect("entry");
        let item = encode_entity(&object(EntityKind::GroundItem, 2), false).expect("entry");
        let wrap = |entry: &[u8]| {
            let mut bytes = Vec::new();
            push_header(&mut bytes, &[0xab; 32], &at(0));
            push_message(&mut bytes, 3, &id(0).identity);
            push_message(&mut bytes, 4, &encode_entity(&own(), false).expect("own"));
            push_message(&mut bytes, 4, entry);
            bytes
        };
        assert!(decode_world_spatial_entities_snapshot(&wrap(&valid)).is_ok());
        assert!(decode_world_spatial_entities_snapshot(&wrap(&item)).is_ok());
        let with = |base: &[u8], extra: &[u8]| [base, extra].concat();
        let cases: Vec<Vec<u8>> = vec![
            with(&valid, &[0x08, 0x02]),          // repeated kind
            with(&valid, &[0x50, 0x01]),          // unknown field
            with(&valid, &[0x40, 0x01]),          // item field on an actor
            with(&item, &[0x28, 0x01]),           // direction on an object
            vec![0x08, 0x06],                     // unknown kind
            vec![0x08, 0x00],                     // zero kind
            [&valid[..], &[0x38, 0x65]].concat(), // health 101 after 100: repeated
            valid[..valid.len() - 1].to_vec(),    // truncated
        ];
        for bad in cases {
            assert!(
                decode_world_spatial_entities_snapshot(&wrap(&bad)).is_err(),
                "{bad:?}"
            );
        }
        // Health above 100, missing direction, zero quantity and a short identity.
        let mut entity = actor(EntityKind::Creature, 5);
        entity.detail = EntityDetail::Actor {
            direction: StepDirection::North,
            appearance_ref: 0,
            health_percent: 101,
        };
        assert!(encode_entity(&entity, false).is_err());
        let mut no_quantity = object(EntityKind::Corpse, 5);
        no_quantity.detail = EntityDetail::Object {
            item_definition_ref: 1,
            quantity: 0,
            item_handle: None,
        };
        assert!(encode_entity(&no_quantity, false).is_err());
        assert!(
            encode_entity(
                &WorldSpatialEntity {
                    kind: EntityKind::Corpse,
                    ..actor(EntityKind::Player, 1)
                },
                false
            )
            .is_err()
        );
        assert!(decode_entity(&[0x08, 0x01, 0x12, 0x02, 0, 0], false).is_err());
        assert_eq!(
            decode_world_spatial_entities_snapshot(
                &[0; MAX_WORLD_SPATIAL_ENTITIES_PAYLOAD_BYTES + 1]
            ),
            Err(WorldSpatialError::LimitExceeded)
        );
    }

    #[test]
    fn revisions_are_not_interchangeable_and_the_capability_gates_the_client_decode() {
        let entities = snapshot(vec![own(), actor(EntityKind::Creature, 1)]);
        let v2 = encode_world_spatial_entities_snapshot(&entities).expect("v2");
        // An old client reading the new payload as v1 fails closed.
        assert!(decode_world_spatial(&v2).is_err());
        // A new decoder reading a v1 payload as v2 fails closed.
        let v1 = encode_world_spatial(&WorldSpatialObservation {
            content_generation: [0xab; 32],
            actor_position: at(0),
        });
        assert!(decode_world_spatial_entities_snapshot(&v1).is_err());

        let selected = [CAPABILITY_WORLD_SPATIAL_ENTITIES];
        assert_eq!(
            decode_world_spatial_snapshot_view(&selected, 2, &v2),
            Ok(WorldSpatialSnapshotView::Entities(entities))
        );
        // Without the capability type 2 is an unnegotiated type; type 1 always decodes.
        assert!(decode_world_spatial_snapshot_view(&[], 2, &v2).is_err());
        assert!(decode_world_spatial_snapshot_view(&[1], 2, &v2).is_err());
        assert!(matches!(
            decode_world_spatial_snapshot_view(&[], SNAPSHOT_TYPE_WORLD_SPATIAL_V1, &v1),
            Ok(WorldSpatialSnapshotView::OwnActor(_))
        ));
        assert!(decode_world_spatial_snapshot_view(&selected, 3, &v2).is_err());
        let d = encode_world_spatial_entities_delta(&delta(1, 1, 1)).expect("delta");
        assert!(matches!(
            decode_world_spatial_delta_view(&selected, 2, &d),
            Ok(WorldSpatialDeltaView::Entities(_))
        ));
        assert!(decode_world_spatial_delta_view(&[], 2, &d).is_err());
        assert!(decode_world_spatial_delta_view(&[], DELTA_TYPE_WORLD_SPATIAL_V1, &v1).is_ok());
    }

    fn handled(kind: EntityKind, n: u32, handle: u64) -> WorldSpatialEntity {
        let mut entity = object(kind, n);
        entity.detail = EntityDetail::Object {
            item_definition_ref: u32::MAX,
            quantity: u32::MAX,
            item_handle: NonZeroU64::new(handle),
        };
        entity
    }

    fn without_handles(mut entities: Vec<WorldSpatialEntity>) -> Vec<WorldSpatialEntity> {
        for entity in &mut entities {
            if let EntityDetail::Object { item_handle, .. } = &mut entity.detail {
                *item_handle = None;
            }
        }
        entities
    }

    #[test]
    fn item_handles_are_carried_only_under_capability_4_and_round_trip_both_ways() {
        let with = snapshot(vec![
            own(),
            actor(EntityKind::Creature, 1),
            handled(EntityKind::Corpse, 2, u64::MAX),
            handled(EntityKind::GroundItem, 3, 1),
        ]);
        let plain = snapshot(without_handles(with.entities.clone()));

        // Capability 4: the handle round-trips, and the largest object entry stays within 128 B.
        let bytes =
            encode_world_spatial_entities_snapshot_with_item_handles(&with).expect("encode");
        assert_eq!(
            decode_world_spatial_entities_snapshot_with_item_handles(&bytes),
            Ok(with.clone())
        );
        let entry = encode_entity(&with.entities[2], true).expect("entry");
        assert!(entry.len() <= MAX_ENTITY_ENTRY_BYTES, "{}", entry.len());

        // Without it the entry is the unchanged D85 entry, byte for byte, and decodes as before.
        let old = encode_world_spatial_entities_snapshot(&with).expect("encode");
        assert_eq!(
            old,
            encode_world_spatial_entities_snapshot(&plain).expect("plain")
        );
        assert_eq!(
            decode_world_spatial_entities_snapshot(&old),
            Ok(plain.clone())
        );

        // Each side refuses the other's entry.
        assert_eq!(
            decode_world_spatial_entities_snapshot(&bytes),
            Err(WorldSpatialError::Malformed)
        );
        assert_eq!(
            decode_world_spatial_entities_snapshot_with_item_handles(&old),
            Err(WorldSpatialError::Malformed)
        );
        assert_eq!(
            encode_world_spatial_entities_snapshot_with_item_handles(&plain),
            Err(WorldSpatialError::Malformed)
        );

        // Deltas follow the same rule.
        let mut change = delta(1, 0, 1);
        change.update = vec![handled(EntityKind::Corpse, 7, 42)];
        let bytes = encode_world_spatial_entities_delta_with_item_handles(&change).expect("delta");
        assert_eq!(
            decode_world_spatial_entities_delta_with_item_handles(&bytes),
            Ok(change.clone())
        );
        assert!(decode_world_spatial_entities_delta(&bytes).is_err());
        let old = encode_world_spatial_entities_delta(&change).expect("delta");
        assert!(decode_world_spatial_entities_delta_with_item_handles(&old).is_err());

        // The client view decoders pick the codec from the selected capabilities.
        let both = [
            CAPABILITY_ITEM_VIEW_MOVE_V1,
            CAPABILITY_WORLD_SPATIAL_ENTITIES,
        ];
        let handled_bytes =
            encode_world_spatial_entities_snapshot_with_item_handles(&with).expect("encode");
        assert_eq!(
            decode_world_spatial_snapshot_view(
                &both,
                SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                &handled_bytes
            ),
            Ok(WorldSpatialSnapshotView::Entities(with))
        );
        assert!(
            decode_world_spatial_snapshot_view(
                &[CAPABILITY_WORLD_SPATIAL_ENTITIES],
                SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                &handled_bytes
            )
            .is_err()
        );
        assert!(
            decode_world_spatial_delta_view(&both, DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2, &bytes)
                .is_ok()
        );
    }

    #[test]
    fn item_handle_field_fails_closed_when_zero_repeated_or_on_an_actor() {
        let item = encode_entity(&handled(EntityKind::GroundItem, 2, 9), true).expect("entry");
        let actor_entry = encode_entity(&actor(EntityKind::Creature, 1), true).expect("actor");
        let plain_item = encode_entity(&object(EntityKind::Corpse, 3), false).expect("entry");
        for bad in [
            [&item[..], &[0x50, 0x01]].concat(),        // repeated handle
            [&actor_entry[..], &[0x50, 0x01]].concat(), // handle on an actor
            [&plain_item[..], &[0x50, 0x00]].concat(),  // zero handle
            plain_item.clone(),                         // object without a handle
        ] {
            assert_eq!(
                decode_entity(&bad, true),
                Err(WorldSpatialError::Malformed),
                "{bad:?}"
            );
        }
        assert!(decode_entity(&item, true).is_ok());
        assert!(decode_entity(&actor_entry, true).is_ok());
    }

    #[test]
    fn a_repeated_item_handle_fails_closed_on_encode_and_decode() {
        // Snapshot: two objects naming one handle are refused both ways.
        let twice = snapshot(vec![
            own(),
            handled(EntityKind::Corpse, 20, 5),
            handled(EntityKind::GroundItem, 21, 5),
        ]);
        assert_eq!(
            encode_world_spatial_entities_snapshot_with_item_handles(&twice),
            Err(WorldSpatialError::Malformed)
        );
        let once = snapshot(vec![own(), handled(EntityKind::Corpse, 20, 5)]);
        let mut bytes =
            encode_world_spatial_entities_snapshot_with_item_handles(&once).expect("encode");
        let extra = encode_entity(&handled(EntityKind::GroundItem, 21, 5), true).expect("entry");
        push_message(&mut bytes, 4, &extra);
        assert_eq!(
            decode_world_spatial_entities_snapshot_with_item_handles(&bytes),
            Err(WorldSpatialError::Malformed)
        );

        // Delta: an entering and an updated object naming one handle are refused both ways.
        let mut change = delta(0, 0, 0);
        change.enter = vec![handled(EntityKind::GroundItem, 21, 5)];
        change.update = vec![handled(EntityKind::Corpse, 20, 5)];
        assert_eq!(
            encode_world_spatial_entities_delta_with_item_handles(&change),
            Err(WorldSpatialError::Malformed)
        );
        change.update.clear();
        let mut bytes =
            encode_world_spatial_entities_delta_with_item_handles(&change).expect("delta");
        let extra = encode_entity(&handled(EntityKind::Corpse, 20, 5), true).expect("entry");
        push_message(&mut bytes, 4, &extra);
        assert_eq!(
            decode_world_spatial_entities_delta_with_item_handles(&bytes),
            Err(WorldSpatialError::Malformed)
        );

        // Distinct handles stay valid.
        change.update = vec![handled(EntityKind::Corpse, 20, 6)];
        assert!(encode_world_spatial_entities_delta_with_item_handles(&change).is_ok());
    }

    #[test]
    fn registries_bind_capability_6_the_type_2_payloads_and_the_limits() {
        let protocol: Value = serde_json::from_str(PROTOCOL_REGISTRY).expect("protocol registry");
        let capabilities = protocol["capabilities"].as_array().expect("capabilities");
        let matching: Vec<_> = capabilities
            .iter()
            .filter(|c| c["id"] == CAPABILITY_WORLD_SPATIAL_ENTITIES)
            .collect();
        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0]["name"], "WORLD_SPATIAL_ENTITIES");
        assert_eq!(matching[0]["state_domains"], serde_json::json!([1]));
        assert_eq!(matching[0]["command_types"], serde_json::json!([]));
        assert_eq!(matching[0]["offered"], false);

        let domain = protocol["state_domains"]
            .as_array()
            .expect("state_domains")
            .iter()
            .find(|domain| domain["id"] == 1)
            .expect("domain 1");
        let bound = MAX_WORLD_SPATIAL_ENTITIES_PAYLOAD_BYTES as u64;
        for (key, id, name) in [
            (
                "delta_types",
                DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                "WORLD_SPATIAL_ENTITIES_DELTA_V2",
            ),
            (
                "snapshot_types",
                SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                "WORLD_SPATIAL_ENTITIES_SNAPSHOT_V2",
            ),
        ] {
            let types = domain[key].as_array().expect("types");
            assert_eq!(types.len(), 2, "{key}: v1 and v2 only");
            let v1 = &types[0];
            assert_eq!(
                (v1["id"].as_u64(), v1["max_payload_bytes"].as_u64()),
                (Some(1), Some(64))
            );
            assert!(
                v1.get("capability").is_none(),
                "v1 is never capability-gated"
            );
            let v2 = &types[1];
            assert_eq!(v2["id"], id);
            assert_eq!(v2["name"], name);
            assert_eq!(v2["capability"], CAPABILITY_WORLD_SPATIAL_ENTITIES);
            assert_eq!(v2["max_payload_bytes"].as_u64(), Some(bound));
            assert!(v2["payload_schema"].as_str().is_some_and(|s| {
                s.starts_with("docs/contracts/protocol-oteryn/v1/world_spatial_v1.proto#")
            }));
        }
        assert_eq!(bound, 256 * MAX_ENTITY_ENTRY_BYTES as u64 + 1024);

        let resources: Value = serde_json::from_str(RESOURCE_REGISTRY).expect("resource registry");
        let entries = resources["entries"].as_array().expect("entries");
        let limit = |row: &str| {
            entries
                .iter()
                .find(|e| e["id"] == row)
                .and_then(|e| e["hard_maximum"].as_u64())
        };
        assert_eq!(limit("MOVE-RL-11"), Some(MAX_SNAPSHOT_ENTITIES as u64));
        assert_eq!(limit("MOVE-RL-08"), Some(MAX_DELTA_ENTRIES as u64));
        assert_eq!(limit("MOVE-RL-10"), Some(MAX_SNAPSHOT_ENTITIES as u64));
    }
}
