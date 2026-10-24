//! Spell and combat presentation payloads (PRESENT-WIRE-1).
//!
//! Schema: `docs/contracts/protocol-oteryn/v1/presentation_v1.proto`, from SPELL-PRESENT-0 §4, §7,
//! §9, §10 and §13 (ACCEPT-SPELL-PRESENT0-V1). Capability 23 `PRESENTATION_V1`, state domain 19
//! `WORLD_PRESENTATION` and state domain 20 `ACTOR_COOLDOWNS` are registered in
//! `PROTOCOL_OTERYN_V1_REGISTRY.json`, leased by the #1622 control plane. The capability has no
//! command type and requires 6 `WORLD_SPATIAL_ENTITIES`.
//!
//! Decoding is strict, and encoding refuses the same values as a server fault before any byte is
//! emitted. The following all fail closed: zero or unknown enums, an empty oneof, a client id,
//! spell index or group ordinal outside its range, a value of 0, an actor reference with a zero
//! generation, a floor outside 0..=15, batch events out of (decision, emission) order or repeated,
//! a repeated cooldown key, a clock value of 2^42 or more, an unestimated round trip that is not
//! 0, a payload, event or entry over its bound, and unknown or repeated fields.
//!
//! [`select_presentation_batch`] is the §9 overflow order. The slow-client drop
//! (`SPELLPRES0-RL-07`) and the snapshot barrier belong to the emitting connection.

use std::collections::BTreeSet;

pub use crate::charm_wire::CyclopediaWireError as PresentationWireError;
use crate::charm_wire::{
    WireResult, push_message_field, push_nonzero_varint_field, push_varint_field, read_bytes,
    read_uint32, read_varint, set_once,
};
use crate::world_spatial::{ActorPosition, decode_position, encode_position};
use crate::world_spatial_entities::{ENTITY_IDENTITY_BYTES, EntityRef};

/// Registered capability `PRESENTATION_V1`: no command type, state domains 19 and 20, requires 6.
pub const CAPABILITY_PRESENTATION_V1: u32 = 23;
/// The capability `PRESENTATION_V1` requires: 6 `WORLD_SPATIAL_ENTITIES`, whose D85 identities
/// the events name (§7).
pub const CAPABILITY_PRESENTATION_V1_REQUIRES: [u32; 1] = [
    crate::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES, // WORLD_SPATIAL_ENTITIES
];
/// Registered state domain `WORLD_PRESENTATION` (capability 23).
pub const STATE_DOMAIN_WORLD_PRESENTATION: u32 = 19;
/// Delta type 1 `WORLD_PRESENTATION_BATCH_V1`.
pub const DELTA_TYPE_WORLD_PRESENTATION_BATCH_V1: u32 = 1;
/// Snapshot type 1 `WORLD_PRESENTATION_SNAPSHOT_V1`, always empty.
pub const SNAPSHOT_TYPE_WORLD_PRESENTATION_V1: u32 = 1;
/// Registered state domain `ACTOR_COOLDOWNS` (capability 23).
pub const STATE_DOMAIN_ACTOR_COOLDOWNS: u32 = 20;
/// Delta type 1 `ACTOR_COOLDOWNS_DELTA_V1`.
pub const DELTA_TYPE_ACTOR_COOLDOWNS_V1: u32 = 1;
/// Snapshot type 1 `ACTOR_COOLDOWNS_SNAPSHOT_V1`.
pub const SNAPSHOT_TYPE_ACTOR_COOLDOWNS_V1: u32 = 1;

/// `SPELLPRES0-RL-01`: events in one session's batch of one sync unit.
pub const MAX_PRESENTATION_EVENTS: usize = 1_024;
/// `SPELLPRES0-RL-02`: bytes of one event, its batch tag and length included. The worst case is
/// a `value_text` with both references, both 10-byte values and both ordinals at `u32::MAX`: 126
/// bytes.
pub const MAX_PRESENTATION_EVENT_BYTES: usize = 128;
/// `SPELLPRES0-RL-03`: bytes of one batch (1,024 x 128 plus the decision's 64-byte header slack).
pub const MAX_PRESENTATION_BATCH_BYTES: usize = 131_136;
/// The highest 15.30 client effect, missile or sound id.
pub const MAX_CLIENT_ID: u32 = 65_535;
/// The highest SPELL-D1 spell index a cooldown entry carries: a 4-byte id keeps the entry, with its
/// kind and a 6-byte `expires_at_ms`, at `SPELLPRES0-RL-05` (16 bytes). `spell_words` takes any
/// nonzero `uint32`.
pub const MAX_COOLDOWN_SPELL_INDEX: u32 = (1 << 28) - 1;
/// The highest floor a position carries.
pub const MAX_FLOOR: i16 = 15;

/// `SPELLPRES0-RL-04` codec ceiling: the FND-02 repeated-field limit. The registered value is
/// `SPELL-RL-04` plus 16 group slots once composition measures `SPELL-RL-04`.
pub const MAX_COOLDOWN_ENTRIES: usize = crate::MAX_ORDINARY_REPEATED_ENTRIES;
/// `SPELLPRES0-RL-05`: bytes of one cooldown entry, its tag and length included (at most 15).
pub const MAX_COOLDOWN_ENTRY_BYTES: usize = 16;
/// The 16 group slots of `SPELLPRES0-RL-04`: the S9 group ordinal is 1..=16.
pub const MAX_COOLDOWN_GROUP: u32 = 16;
/// §10: `expires_at_ms` and `server_now_ms` stay within six varint bytes.
pub const MAX_CLOCK_MS: u64 = (1 << 42) - 1;
/// `server_now_ms` (7), `rtt_ms` (6) and `rtt_estimated` (2) plus the entries.
pub const MAX_ACTOR_COOLDOWNS_BYTES: usize = 16 + MAX_COOLDOWN_ENTRIES * MAX_COOLDOWN_ENTRY_BYTES;

const _: () =
    assert!(MAX_PRESENTATION_EVENTS * MAX_PRESENTATION_EVENT_BYTES <= MAX_PRESENTATION_BATCH_BYTES);
const _: () = assert!(MAX_PRESENTATION_BATCH_BYTES <= crate::MAX_STATE_DELTA_PAYLOAD_BYTES);
const _: () = assert!(MAX_PRESENTATION_EVENTS <= crate::MAX_ORDINARY_REPEATED_ENTRIES);
const _: () = assert!(MAX_ACTOR_COOLDOWNS_BYTES <= crate::MAX_STATE_DELTA_PAYLOAD_BYTES);

/// §4: who caused the event, chosen per observer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationSource {
    Own = 1,
    Others = 2,
    Creatures = 3,
    Global = 4,
}

/// §4: the `value_text` kind, chosen per observer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationValueKind {
    DamageDealt = 1,
    DamageReceived = 2,
    DamageOthers = 3,
    Healed = 4,
    HealedOthers = 5,
    Mana = 6,
    Experience = 7,
    ExperienceOthers = 8,
}

/// §4 and §5: the Tibia text colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationColor {
    Red = 1,
    LightGreen = 2,
    LightGrey = 3,
    Orange = 4,
    Purple = 5,
    SkyBlue = 6,
    Yellow = 7,
    DarkRed = 8,
    LightBlue = 9,
    Blue = 10,
    PastelRed = 11,
    MayaBlue = 12,
    White = 13,
}

/// A number and its colour; the value is at least 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresentationValue {
    pub value: u64,
    pub color: PresentationColor,
}

/// The oneof of `PresentationEventV1`. Actor references carry a non-zero generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationEventKind {
    MagicEffect {
        position: ActorPosition,
        effect_id: u32,
        source: PresentationSource,
    },
    Missile {
        from: ActorPosition,
        to: ActorPosition,
        missile_id: u32,
        source: PresentationSource,
    },
    SpellWords {
        caster: EntityRef,
        position: ActorPosition,
        spell: u32,
    },
    ValueText {
        kind: PresentationValueKind,
        position: ActorPosition,
        target: EntityRef,
        attacker: Option<EntityRef>,
        primary: PresentationValue,
        secondary: Option<PresentationValue>,
    },
    Sound {
        position: ActorPosition,
        cue_id: u32,
        source: PresentationSource,
        /// 0 when the cue has no secondary cue.
        secondary_cue_id: u32,
    },
}

/// `PresentationEventV1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresentationEvent {
    /// The per-sync-unit ordinal of the decision that emitted it, at least 1.
    pub decision_ordinal: u32,
    /// The ordinal within that decision, 1-based.
    pub emission_ordinal: u32,
    pub kind: PresentationEventKind,
}

impl PresentationEvent {
    fn order_key(&self) -> (u32, u32) {
        (self.decision_ordinal, self.emission_ordinal)
    }
}

/// One event offered to a session's batch, with the §9 overflow facts the emitter knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresentationCandidate {
    /// The observer's own event: its cast, its received damage or heal.
    pub own: bool,
    /// The D87 rank of the event's entity for this observer, lower is nearer; events with no
    /// entity identity rank by distance in the same order.
    pub nearness: u32,
    pub event: PresentationEvent,
}

/// §9: keeps at most `SPELLPRES0-RL-01` events, own events first, then the nearest, ties broken
/// by decision ordinal and then emission ordinal, and returns them in (decision, emission) order.
/// The result never depends on the input order.
#[must_use]
pub fn select_presentation_batch(
    mut candidates: Vec<PresentationCandidate>,
) -> Vec<PresentationEvent> {
    candidates.sort_unstable_by_key(|c| (!c.own, c.nearness, c.event.order_key()));
    candidates.truncate(MAX_PRESENTATION_EVENTS);
    let mut kept: Vec<PresentationEvent> = candidates.into_iter().map(|c| c.event).collect();
    kept.sort_unstable_by_key(PresentationEvent::order_key);
    kept
}

/// `ActorCooldownKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActorCooldownKind {
    Spell = 1,
    Group = 2,
}

/// `ActorCooldownV1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorCooldown {
    pub kind: ActorCooldownKind,
    /// The SPELL-D1 index (1..=2^28 - 1) or the S9 group ordinal (1..=16).
    pub id: u32,
    pub expires_at_ms: u64,
}

/// `ActorCooldownsV1`: the snapshot (type 1) or delta (type 1) of domain 20.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorCooldowns {
    pub server_now_ms: u64,
    /// The smoothed round trip; `None` before the first liveness ack (`rtt_estimated` false).
    pub rtt_ms: Option<u32>,
    pub entries: Vec<ActorCooldown>,
}

fn malformed<T>() -> WireResult<T> {
    Err(PresentationWireError::Malformed)
}

/// Reads the fields of a message: `field(number, wire_type, input, cursor)` consumes one value or
/// refuses the key.
fn read_fields(
    input: &[u8],
    mut field: impl FnMut(u64, u64, &[u8], &mut usize) -> WireResult<()>,
) -> WireResult<()> {
    let mut cursor = 0;
    while cursor < input.len() {
        let key = read_varint(input, &mut cursor)?;
        field(key >> 3, key & 0x07, input, &mut cursor)?;
    }
    Ok(())
}

fn check_position(position: &ActorPosition) -> WireResult<()> {
    if (0..=MAX_FLOOR).contains(&position.floor) {
        Ok(())
    } else {
        malformed()
    }
}

fn check_range(value: u32, maximum: u32) -> WireResult<()> {
    if (1..=maximum).contains(&value) {
        Ok(())
    } else {
        malformed()
    }
}

fn check_actor(entity: &EntityRef) -> WireResult<()> {
    if entity.generation == 0 {
        return malformed();
    }
    Ok(())
}

fn check_value(value: &PresentationValue) -> WireResult<()> {
    if value.value == 0 {
        return malformed();
    }
    Ok(())
}

fn check_event(event: &PresentationEvent) -> WireResult<()> {
    check_range(event.decision_ordinal, u32::MAX)?;
    check_range(event.emission_ordinal, u32::MAX)?;
    match &event.kind {
        PresentationEventKind::MagicEffect {
            position,
            effect_id,
            ..
        } => {
            check_position(position)?;
            check_range(*effect_id, MAX_CLIENT_ID)
        }
        PresentationEventKind::Missile {
            from,
            to,
            missile_id,
            ..
        } => {
            check_position(from)?;
            check_position(to)?;
            check_range(*missile_id, MAX_CLIENT_ID)
        }
        PresentationEventKind::SpellWords {
            caster,
            position,
            spell,
        } => {
            check_actor(caster)?;
            check_position(position)?;
            check_range(*spell, u32::MAX)
        }
        PresentationEventKind::ValueText {
            position,
            target,
            attacker,
            primary,
            secondary,
            ..
        } => {
            check_position(position)?;
            check_actor(target)?;
            attacker.as_ref().map_or(Ok(()), check_actor)?;
            check_value(primary)?;
            secondary.as_ref().map_or(Ok(()), check_value)
        }
        PresentationEventKind::Sound {
            position,
            cue_id,
            secondary_cue_id,
            ..
        } => {
            check_position(position)?;
            check_range(*cue_id, MAX_CLIENT_ID)?;
            if *secondary_cue_id > MAX_CLIENT_ID {
                return malformed();
            }
            Ok(())
        }
    }
}

fn encode_ref(entity: &EntityRef) -> Vec<u8> {
    let mut output = Vec::with_capacity(ENTITY_IDENTITY_BYTES + 13);
    push_message_field(&mut output, 1, &entity.identity);
    push_nonzero_varint_field(&mut output, 2, entity.generation);
    output
}

fn encode_value(value: &PresentationValue) -> Vec<u8> {
    let mut output = Vec::with_capacity(13);
    push_varint_field(&mut output, 1, value.value);
    push_varint_field(&mut output, 2, value.color as u64);
    output
}

fn encode_kind(kind: &PresentationEventKind) -> (u64, Vec<u8>) {
    let mut output = Vec::with_capacity(112);
    let field = match kind {
        PresentationEventKind::MagicEffect {
            position,
            effect_id,
            source,
        } => {
            push_message_field(&mut output, 1, &encode_position(position));
            push_varint_field(&mut output, 2, u64::from(*effect_id));
            push_varint_field(&mut output, 3, *source as u64);
            3
        }
        PresentationEventKind::Missile {
            from,
            to,
            missile_id,
            source,
        } => {
            push_message_field(&mut output, 1, &encode_position(from));
            push_message_field(&mut output, 2, &encode_position(to));
            push_varint_field(&mut output, 3, u64::from(*missile_id));
            push_varint_field(&mut output, 4, *source as u64);
            4
        }
        PresentationEventKind::SpellWords {
            caster,
            position,
            spell,
        } => {
            push_message_field(&mut output, 1, &encode_ref(caster));
            push_message_field(&mut output, 2, &encode_position(position));
            push_varint_field(&mut output, 3, u64::from(*spell));
            5
        }
        PresentationEventKind::ValueText {
            kind,
            position,
            target,
            attacker,
            primary,
            secondary,
        } => {
            push_varint_field(&mut output, 1, *kind as u64);
            push_message_field(&mut output, 2, &encode_position(position));
            push_message_field(&mut output, 3, &encode_ref(target));
            if let Some(attacker) = attacker {
                push_message_field(&mut output, 4, &encode_ref(attacker));
            }
            push_message_field(&mut output, 5, &encode_value(primary));
            if let Some(secondary) = secondary {
                push_message_field(&mut output, 6, &encode_value(secondary));
            }
            6
        }
        PresentationEventKind::Sound {
            position,
            cue_id,
            source,
            secondary_cue_id,
        } => {
            push_message_field(&mut output, 1, &encode_position(position));
            push_varint_field(&mut output, 2, u64::from(*cue_id));
            push_varint_field(&mut output, 3, *source as u64);
            push_nonzero_varint_field(&mut output, 4, u64::from(*secondary_cue_id));
            7
        }
    };
    (field, output)
}

fn encode_event(event: &PresentationEvent) -> WireResult<Vec<u8>> {
    check_event(event)?;
    let mut output = Vec::with_capacity(MAX_PRESENTATION_EVENT_BYTES);
    push_varint_field(&mut output, 1, u64::from(event.decision_ordinal));
    push_varint_field(&mut output, 2, u64::from(event.emission_ordinal));
    let (field, kind) = encode_kind(&event.kind);
    push_message_field(&mut output, field, &kind);
    Ok(output)
}

/// Encodes the domain 19 delta (type 1): 1 to 1,024 events, strictly ascending by (decision,
/// emission). Anything else is a server fault.
pub fn encode_presentation_batch(events: &[PresentationEvent]) -> WireResult<Vec<u8>> {
    if events.is_empty() {
        return malformed();
    }
    if events.len() > MAX_PRESENTATION_EVENTS {
        return Err(PresentationWireError::LimitExceeded);
    }
    if events
        .windows(2)
        .any(|pair| pair[0].order_key() >= pair[1].order_key())
    {
        return malformed();
    }
    let mut output = Vec::with_capacity(events.len() * 64);
    for event in events {
        let start = output.len();
        push_message_field(&mut output, 1, &encode_event(event)?);
        if output.len() - start > MAX_PRESENTATION_EVENT_BYTES {
            return Err(PresentationWireError::LimitExceeded);
        }
    }
    Ok(output)
}

/// The domain 19 snapshot (type 1): always empty.
#[must_use]
pub fn encode_presentation_snapshot() -> Vec<u8> {
    Vec::new()
}

pub fn decode_presentation_snapshot(payload: &[u8]) -> WireResult<()> {
    if payload.is_empty() {
        Ok(())
    } else {
        malformed()
    }
}

fn read_enum<T>(input: &[u8], cursor: &mut usize, map: fn(u32) -> Option<T>) -> WireResult<T> {
    map(read_uint32(input, cursor)?).ok_or(PresentationWireError::Malformed)
}

fn source(value: u32) -> Option<PresentationSource> {
    Some(match value {
        1 => PresentationSource::Own,
        2 => PresentationSource::Others,
        3 => PresentationSource::Creatures,
        4 => PresentationSource::Global,
        _ => return None,
    })
}

fn value_kind(value: u32) -> Option<PresentationValueKind> {
    use PresentationValueKind as K;
    Some(match value {
        1 => K::DamageDealt,
        2 => K::DamageReceived,
        3 => K::DamageOthers,
        4 => K::Healed,
        5 => K::HealedOthers,
        6 => K::Mana,
        7 => K::Experience,
        8 => K::ExperienceOthers,
        _ => return None,
    })
}

fn color(value: u32) -> Option<PresentationColor> {
    use PresentationColor as C;
    Some(match value {
        1 => C::Red,
        2 => C::LightGreen,
        3 => C::LightGrey,
        4 => C::Orange,
        5 => C::Purple,
        6 => C::SkyBlue,
        7 => C::Yellow,
        8 => C::DarkRed,
        9 => C::LightBlue,
        10 => C::Blue,
        11 => C::PastelRed,
        12 => C::MayaBlue,
        13 => C::White,
        _ => return None,
    })
}

fn read_position(input: &[u8], cursor: &mut usize) -> WireResult<ActorPosition> {
    let position = decode_position(read_bytes(input, cursor)?)
        .map_err(|_| PresentationWireError::Malformed)?;
    check_position(&position)?;
    Ok(position)
}

fn read_ref(input: &[u8], cursor: &mut usize) -> WireResult<EntityRef> {
    let (mut identity, mut generation) = (None, None);
    read_fields(
        read_bytes(input, cursor)?,
        |number, wire, input, cursor| match (number, wire) {
            (1, 2) => {
                let bytes: [u8; ENTITY_IDENTITY_BYTES] = read_bytes(input, cursor)?
                    .try_into()
                    .map_err(|_| PresentationWireError::Malformed)?;
                set_once(&mut identity, bytes)
            }
            (2, 0) => set_once(&mut generation, read_varint(input, cursor)?),
            _ => malformed(),
        },
    )?;
    let entity = EntityRef {
        identity: identity.ok_or(PresentationWireError::Malformed)?,
        generation: generation.unwrap_or(0),
    };
    check_actor(&entity)?;
    Ok(entity)
}

fn read_value(input: &[u8], cursor: &mut usize) -> WireResult<PresentationValue> {
    let (mut value, mut tint) = (None, None);
    read_fields(
        read_bytes(input, cursor)?,
        |number, wire, input, cursor| match (number, wire) {
            (1, 0) => set_once(&mut value, read_varint(input, cursor)?),
            (2, 0) => set_once(&mut tint, read_enum(input, cursor, color)?),
            _ => malformed(),
        },
    )?;
    Ok(PresentationValue {
        value: value.ok_or(PresentationWireError::Malformed)?,
        color: tint.ok_or(PresentationWireError::Malformed)?,
    })
}

/// The fields any event kind may carry; each kind takes its own and refuses the rest.
#[derive(Default)]
struct KindFields {
    positions: [Option<ActorPosition>; 2],
    ids: [Option<u32>; 2],
    source: Option<PresentationSource>,
    value_kind: Option<PresentationValueKind>,
    refs: [Option<EntityRef>; 2],
    values: [Option<PresentationValue>; 2],
}

fn decode_kind(field: u64, input: &[u8]) -> WireResult<PresentationEventKind> {
    let mut f = KindFields::default();
    read_fields(input, |number, wire, input, cursor| {
        match (field, number, wire) {
            (3, 1, 2) | (4, 1, 2) | (5, 2, 2) | (6, 2, 2) | (7, 1, 2) => {
                set_once(&mut f.positions[0], read_position(input, cursor)?)
            }
            (4, 2, 2) => set_once(&mut f.positions[1], read_position(input, cursor)?),
            (3, 2, 0) | (4, 3, 0) | (5, 3, 0) | (7, 2, 0) => {
                set_once(&mut f.ids[0], read_uint32(input, cursor)?)
            }
            (7, 4, 0) => set_once(&mut f.ids[1], read_uint32(input, cursor)?),
            (3, 3, 0) | (4, 4, 0) | (7, 3, 0) => {
                set_once(&mut f.source, read_enum(input, cursor, source)?)
            }
            (6, 1, 0) => set_once(&mut f.value_kind, read_enum(input, cursor, value_kind)?),
            (5, 1, 2) | (6, 3, 2) => set_once(&mut f.refs[0], read_ref(input, cursor)?),
            (6, 4, 2) => set_once(&mut f.refs[1], read_ref(input, cursor)?),
            (6, 5, 2) => set_once(&mut f.values[0], read_value(input, cursor)?),
            (6, 6, 2) => set_once(&mut f.values[1], read_value(input, cursor)?),
            _ => malformed(),
        }
    })?;
    let missing = || PresentationWireError::Malformed;
    let position = f.positions[0].ok_or_else(missing);
    let id = f.ids[0].ok_or_else(missing);
    let source = f.source.ok_or_else(missing);
    Ok(match field {
        3 => PresentationEventKind::MagicEffect {
            position: position?,
            effect_id: id?,
            source: source?,
        },
        4 => PresentationEventKind::Missile {
            from: position?,
            to: f.positions[1].ok_or_else(missing)?,
            missile_id: id?,
            source: source?,
        },
        5 => PresentationEventKind::SpellWords {
            caster: f.refs[0].ok_or_else(missing)?,
            position: position?,
            spell: id?,
        },
        6 => PresentationEventKind::ValueText {
            kind: f.value_kind.ok_or_else(missing)?,
            position: position?,
            target: f.refs[0].ok_or_else(missing)?,
            attacker: f.refs[1],
            primary: f.values[0].ok_or_else(missing)?,
            secondary: f.values[1],
        },
        7 => PresentationEventKind::Sound {
            position: position?,
            cue_id: id?,
            source: source?,
            secondary_cue_id: f.ids[1].unwrap_or(0),
        },
        _ => return malformed(),
    })
}

fn decode_event(input: &[u8]) -> WireResult<PresentationEvent> {
    let (mut decision, mut emission, mut kind) = (None, None, None);
    read_fields(input, |number, wire, input, cursor| match (number, wire) {
        (1, 0) => set_once(&mut decision, read_uint32(input, cursor)?),
        (2, 0) => set_once(&mut emission, read_uint32(input, cursor)?),
        (3..=7, 2) => set_once(&mut kind, decode_kind(number, read_bytes(input, cursor)?)?),
        _ => malformed(),
    })?;
    let event = PresentationEvent {
        decision_ordinal: decision.unwrap_or(0),
        emission_ordinal: emission.unwrap_or(0),
        kind: kind.ok_or(PresentationWireError::Malformed)?,
    };
    check_event(&event)?;
    Ok(event)
}

pub fn decode_presentation_batch(payload: &[u8]) -> WireResult<Vec<PresentationEvent>> {
    if payload.len() > MAX_PRESENTATION_BATCH_BYTES {
        return Err(PresentationWireError::LimitExceeded);
    }
    let mut events: Vec<PresentationEvent> = Vec::new();
    let mut cursor = 0;
    while cursor < payload.len() {
        let start = cursor;
        if read_varint(payload, &mut cursor)? != (1 << 3) | 2 {
            return malformed();
        }
        let entry = read_bytes(payload, &mut cursor)?;
        if cursor - start > MAX_PRESENTATION_EVENT_BYTES {
            return Err(PresentationWireError::LimitExceeded);
        }
        if events.len() == MAX_PRESENTATION_EVENTS {
            return Err(PresentationWireError::LimitExceeded);
        }
        let event = decode_event(entry)?;
        if events
            .last()
            .is_some_and(|last| last.order_key() >= event.order_key())
        {
            return malformed();
        }
        events.push(event);
    }
    if events.is_empty() {
        return malformed();
    }
    Ok(events)
}

fn check_cooldowns(cooldowns: &ActorCooldowns) -> WireResult<()> {
    if cooldowns.server_now_ms > MAX_CLOCK_MS {
        return Err(PresentationWireError::LimitExceeded);
    }
    if cooldowns.entries.len() > MAX_COOLDOWN_ENTRIES {
        return Err(PresentationWireError::LimitExceeded);
    }
    let mut keys = BTreeSet::new();
    for entry in &cooldowns.entries {
        let maximum = match entry.kind {
            ActorCooldownKind::Spell => MAX_COOLDOWN_SPELL_INDEX,
            ActorCooldownKind::Group => MAX_COOLDOWN_GROUP,
        };
        check_range(entry.id, maximum)?;
        if entry.expires_at_ms > MAX_CLOCK_MS {
            return Err(PresentationWireError::LimitExceeded);
        }
        if !keys.insert((entry.kind, entry.id)) {
            return malformed();
        }
    }
    Ok(())
}

/// Encodes the domain 20 snapshot (type 1) or delta (type 1), entries in (kind, id) order. A value
/// out of bounds or a repeated key is a server fault.
pub fn encode_actor_cooldowns(cooldowns: &ActorCooldowns) -> WireResult<Vec<u8>> {
    check_cooldowns(cooldowns)?;
    let mut entries = cooldowns.entries.clone();
    entries.sort_unstable_by_key(|entry| (entry.kind, entry.id));
    let mut output = Vec::with_capacity(16 + entries.len() * MAX_COOLDOWN_ENTRY_BYTES);
    push_nonzero_varint_field(&mut output, 1, cooldowns.server_now_ms);
    push_nonzero_varint_field(&mut output, 2, u64::from(cooldowns.rtt_ms.unwrap_or(0)));
    push_nonzero_varint_field(&mut output, 3, u64::from(cooldowns.rtt_ms.is_some()));
    for entry in &entries {
        let mut inner = Vec::with_capacity(MAX_COOLDOWN_ENTRY_BYTES);
        push_varint_field(&mut inner, 1, entry.kind as u64);
        push_varint_field(&mut inner, 2, u64::from(entry.id));
        push_nonzero_varint_field(&mut inner, 3, entry.expires_at_ms);
        push_message_field(&mut output, 4, &inner);
    }
    Ok(output)
}

/// Reads one field 4 entry whose key began at `key_start`; the bound counts every key byte, so an
/// overlong key cannot hide a longer entry.
fn read_cooldown(input: &[u8], cursor: &mut usize, key_start: usize) -> WireResult<ActorCooldown> {
    let inner = read_bytes(input, cursor)?;
    if *cursor - key_start > MAX_COOLDOWN_ENTRY_BYTES {
        return Err(PresentationWireError::LimitExceeded);
    }
    let (mut kind, mut id, mut expires) = (None, None, None);
    read_fields(inner, |number, wire, input, cursor| match (number, wire) {
        (1, 0) => {
            let kind_value = read_enum(input, cursor, |value| match value {
                1 => Some(ActorCooldownKind::Spell),
                2 => Some(ActorCooldownKind::Group),
                _ => None,
            })?;
            set_once(&mut kind, kind_value)
        }
        (2, 0) => set_once(&mut id, read_uint32(input, cursor)?),
        (3, 0) => set_once(&mut expires, read_varint(input, cursor)?),
        _ => malformed(),
    })?;
    Ok(ActorCooldown {
        kind: kind.ok_or(PresentationWireError::Malformed)?,
        id: id.unwrap_or(0),
        expires_at_ms: expires.unwrap_or(0),
    })
}

pub fn decode_actor_cooldowns(payload: &[u8]) -> WireResult<ActorCooldowns> {
    if payload.len() > MAX_ACTOR_COOLDOWNS_BYTES {
        return Err(PresentationWireError::LimitExceeded);
    }
    let (mut now, mut rtt, mut estimated) = (None, None, None);
    let mut entries = Vec::new();
    let mut cursor = 0;
    while cursor < payload.len() {
        let key_start = cursor;
        let key = read_varint(payload, &mut cursor)?;
        match (key >> 3, key & 0x07) {
            (1, 0) => set_once(&mut now, read_varint(payload, &mut cursor)?)?,
            (2, 0) => set_once(&mut rtt, read_uint32(payload, &mut cursor)?)?,
            (3, 0) => match read_varint(payload, &mut cursor)? {
                0 => set_once(&mut estimated, false)?,
                1 => set_once(&mut estimated, true)?,
                _ => return malformed(),
            },
            (4, 2) => {
                if entries.len() == MAX_COOLDOWN_ENTRIES {
                    return Err(PresentationWireError::LimitExceeded);
                }
                entries.push(read_cooldown(payload, &mut cursor, key_start)?);
            }
            _ => return malformed(),
        }
    }
    let rtt = rtt.unwrap_or(0);
    let rtt_ms = match estimated.unwrap_or(false) {
        true => Some(rtt),
        false if rtt == 0 => None,
        false => return malformed(),
    };
    let cooldowns = ActorCooldowns {
        server_now_ms: now.unwrap_or(0),
        rtt_ms,
        entries,
    };
    check_cooldowns(&cooldowns)?;
    Ok(cooldowns)
}

#[cfg(test)]
#[path = "presentation_tests.rs"]
mod tests;
