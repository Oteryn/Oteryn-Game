//! Spell cast wire and own-actor vitals typed payloads
//! (`OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1` §3: SPELL-D1, SPELL-D2,
//! SPELL-D6, #162 owner acceptance 5867161696; SPELL-D7, owner decision D89 5875958040; SPELL-D8
//! H-3, #162 comment 5884682203; schema in `docs/contracts/protocol-oteryn/v1/actor_spell_v1.proto`).
//! Command type 3 `WORLD_ACTOR_SPELL_CAST_INTENT` and state domain 3 `ACTOR_VITALS` with delta
//! type 1 and snapshot type 1.
//!
//! Decoding is strict: zero or unknown enum values, unknown or repeated fields, over-bound payloads,
//! a spell index of zero, a floor outside the int16 range, a `target_position` present without the
//! `POSITION` intent (or absent with it), a bool other than 0 or 1, a vital above its SPELL-D8 bound
//! and a Harmony above 5 all fail closed. A standard proto3 encoder omits a scalar field holding its
//! default value, so decoding accepts that omission and defaults the field; semantic presence is
//! validated after decoding, not inferred from wire-default omission (FND-02 §7). Encoding is
//! strict too: a vital above its bound is refused as a server fault before any byte is emitted, so
//! this side never produces a payload a conforming decoder would reject.

// The client-side codecs (intent encode, result and vitals decode) are exercised by the round-trip
// tests; the server composes only its own direction (§9 step 2).
#![cfg_attr(not(test), allow(dead_code))]

use std::num::NonZeroU32;

pub const COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT: u32 = 3;
pub const STATE_DOMAIN_ACTOR_VITALS: u32 = 3;
pub const DELTA_TYPE_ACTOR_VITALS_V1: u32 = 1;
pub const SNAPSHOT_TYPE_ACTOR_VITALS_V1: u32 = 1;

/// SPELL-D7 bound. The canonical worst case is 28 bytes: `spell` (1 tag + 5-byte uint32 varint = 6),
/// `target` (1 + 1 = 2), `target_position` (1 tag + 1 length + x 6 + y 6 + int16 floor 4 = 18) and
/// `aim_at_target` (1 + 1 = 2).
pub const MAX_SPELL_CAST_INTENT_BYTES: usize = 32;
/// A single small enum field (1 tag + 1 value byte = 2), with the same slack as
/// `MAX_STEP_RESULT_BYTES` in `world_spatial.rs`.
pub const MAX_SPELL_CAST_RESULT_BYTES: usize = 4;
/// SPELL-D8 byte cap for `ActorVitalsV1`. With the value bounds below the canonical worst case is
/// 4 x 5 + 3 + 2 + 2 = 27 bytes.
pub const MAX_ACTOR_VITALS_BYTES: usize = 32;
/// SPELL-D8: `health`, `max_health`, `mana` and `max_mana` are at most 2^28 - 1.
pub const MAX_VITAL_POOL: u32 = (1 << 28) - 1;
/// SPELL-D8: `soul` is at most 16383.
pub const MAX_SOUL: u32 = 16_383;
/// SPELL-D8: monk Harmony is 0..5.
pub const MAX_HARMONY: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorSpellError {
    /// Not a valid encoding of the schema, or a value outside its declared domain (zero or unknown
    /// enum, spell index 0, floor outside int16, Harmony above 5, bool other than 0 or 1).
    Malformed,
    /// The payload is over its byte bound, or a vital is above its SPELL-D8 value bound.
    LimitExceeded,
}

/// SPELL-D7: a position in the actor's own Channel, in the pinned frame's native coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellTargetPosition {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

/// `SpellTargetIntent` together with `target_position`: the position exists exactly when the
/// intent is `POSITION`, so the invalid combinations cannot be encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellTarget {
    /// `SPELL_TARGET_INTENT_NONE = 1`.
    None,
    /// `SPELL_TARGET_INTENT_ATTACK_TARGET = 2`.
    AttackTarget,
    /// `SPELL_TARGET_INTENT_POSITION = 3` with its `target_position`.
    Position(SpellTargetPosition),
}

impl SpellTarget {
    fn wire_value(self) -> u64 {
        match self {
            Self::None => 1,
            Self::AttackTarget => 2,
            Self::Position(_) => 3,
        }
    }
}

/// `WorldActorSpellCastIntentV1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellCastIntent {
    /// 1-based index into the spell book of the loaded content generation (SPELL-D1).
    pub spell: NonZeroU32,
    pub target: SpellTarget,
    pub aim_at_target: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellCastDisposition {
    Cast = 1,
    CoolingDown = 2,
    LevelTooLow = 3,
    MagicLevelTooLow = 4,
    NotEnoughMana = 5,
    NotEnoughSoul = 6,
    NotAvailable = 7,
    TargetRequired = 8,
    TargetIllegal = 9,
    Rejected = 10,
}

/// `ActorVitalsV1`: the own actor's live vitals (SPELL-D2, SPELL-D8).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActorVitals {
    pub health: u32,
    pub max_health: u32,
    pub mana: u32,
    pub max_mana: u32,
    pub soul: u32,
    pub harmony: u32,
    pub serene: bool,
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

fn push_varint_field(output: &mut Vec<u8>, field: u64, value: u64) {
    push_tag(output, field, 0);
    push_varint(output, value);
}

/// Omits the field at its proto3 default (0), as a standard encoder does.
fn push_nonzero_varint_field(output: &mut Vec<u8>, field: u64, value: u64) {
    if value != 0 {
        push_varint_field(output, field, value);
    }
}

fn push_sint32(output: &mut Vec<u8>, field: u64, value: i32) {
    let zigzag = ((value << 1) ^ (value >> 31)) as u32;
    push_nonzero_varint_field(output, field, u64::from(zigzag));
}

fn read_varint(input: &[u8], cursor: &mut usize) -> Result<u64, ActorSpellError> {
    let mut value = 0_u64;
    for shift in (0..70).step_by(7) {
        let byte = *input.get(*cursor).ok_or(ActorSpellError::Malformed)?;
        *cursor += 1;
        if shift == 63 && byte > 1 {
            return Err(ActorSpellError::Malformed);
        }
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(ActorSpellError::Malformed)
}

fn read_uint32(input: &[u8], cursor: &mut usize) -> Result<u32, ActorSpellError> {
    u32::try_from(read_varint(input, cursor)?).map_err(|_| ActorSpellError::Malformed)
}

fn read_bool(input: &[u8], cursor: &mut usize) -> Result<bool, ActorSpellError> {
    match read_varint(input, cursor)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(ActorSpellError::Malformed),
    }
}

fn read_bytes<'a>(input: &'a [u8], cursor: &mut usize) -> Result<&'a [u8], ActorSpellError> {
    let len =
        usize::try_from(read_varint(input, cursor)?).map_err(|_| ActorSpellError::Malformed)?;
    let end = cursor
        .checked_add(len)
        .filter(|end| *end <= input.len())
        .ok_or(ActorSpellError::Malformed)?;
    let value = &input[*cursor..end];
    *cursor = end;
    Ok(value)
}

/// Stores a singular field once; a repeated field fails closed.
fn set_once<T>(slot: &mut Option<T>, value: T) -> Result<(), ActorSpellError> {
    if slot.is_some() {
        return Err(ActorSpellError::Malformed);
    }
    *slot = Some(value);
    Ok(())
}

/// Reads exactly one singular enum field 1 (the only field of the cast result).
fn read_single_enum(input: &[u8], maximum: usize) -> Result<u64, ActorSpellError> {
    if input.len() > maximum {
        return Err(ActorSpellError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut value = None;
    while cursor < input.len() {
        let key = read_varint(input, &mut cursor)?;
        if key != (1 << 3) {
            return Err(ActorSpellError::Malformed);
        }
        let read = read_varint(input, &mut cursor)?;
        set_once(&mut value, read)?;
    }
    value.ok_or(ActorSpellError::Malformed)
}

fn decode_sint32(value: u64) -> Result<i32, ActorSpellError> {
    let zigzag = u32::try_from(value).map_err(|_| ActorSpellError::Malformed)?;
    Ok(((zigzag >> 1) as i32) ^ -((zigzag & 1) as i32))
}

fn encode_position(position: SpellTargetPosition) -> Vec<u8> {
    let mut output = Vec::with_capacity(16);
    push_sint32(&mut output, 1, position.x);
    push_sint32(&mut output, 2, position.y);
    push_sint32(&mut output, 3, i32::from(position.floor));
    output
}

fn decode_position(input: &[u8]) -> Result<SpellTargetPosition, ActorSpellError> {
    let mut cursor = 0;
    let (mut x, mut y, mut floor) = (None, None, None);
    while cursor < input.len() {
        let key = read_varint(input, &mut cursor)?;
        let slot = match key {
            0x08 => &mut x,
            0x10 => &mut y,
            0x18 => &mut floor,
            _ => return Err(ActorSpellError::Malformed),
        };
        let value = decode_sint32(read_varint(input, &mut cursor)?)?;
        set_once(slot, value)?;
    }
    let floor = i16::try_from(floor.unwrap_or(0)).map_err(|_| ActorSpellError::Malformed)?;
    Ok(SpellTargetPosition {
        x: x.unwrap_or(0),
        y: y.unwrap_or(0),
        floor,
    })
}

/// `ClientCommand.payload` of command type 3. Fields in field-number order; `target_position` is
/// emitted exactly for the `POSITION` intent (an all-zero position as an empty submessage), and
/// `aim_at_target` only when true.
pub fn encode_spell_cast_intent(intent: &SpellCastIntent) -> Vec<u8> {
    let mut output = Vec::with_capacity(MAX_SPELL_CAST_INTENT_BYTES);
    push_varint_field(&mut output, 1, u64::from(intent.spell.get()));
    push_varint_field(&mut output, 2, intent.target.wire_value());
    if let SpellTarget::Position(position) = intent.target {
        let inner = encode_position(position);
        push_tag(&mut output, 3, 2);
        push_varint(&mut output, inner.len() as u64);
        output.extend_from_slice(&inner);
    }
    if intent.aim_at_target {
        push_varint_field(&mut output, 4, 1);
    }
    output
}

pub fn decode_spell_cast_intent(payload: &[u8]) -> Result<SpellCastIntent, ActorSpellError> {
    if payload.len() > MAX_SPELL_CAST_INTENT_BYTES {
        return Err(ActorSpellError::LimitExceeded);
    }
    let mut cursor = 0;
    let (mut spell, mut target, mut position, mut aim) = (None, None, None, None);
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        match key {
            0x08 => {
                let value = read_uint32(payload, &mut cursor)?;
                set_once(&mut spell, value)?;
            }
            0x10 => {
                let value = read_varint(payload, &mut cursor)?;
                set_once(&mut target, value)?;
            }
            0x1a => {
                let value = decode_position(read_bytes(payload, &mut cursor)?)?;
                set_once(&mut position, value)?;
            }
            0x20 => {
                let value = read_bool(payload, &mut cursor)?;
                set_once(&mut aim, value)?;
            }
            _ => return Err(ActorSpellError::Malformed),
        }
    }
    // SPELL-D1: the index is 1-based, so zero (or an omitted field) names no spell.
    let spell = spell
        .and_then(NonZeroU32::new)
        .ok_or(ActorSpellError::Malformed)?;
    // SPELL-D7: `target_position` exactly with `POSITION`; zero or unknown intents fail closed.
    let target = match (target, position) {
        (Some(1), None) => SpellTarget::None,
        (Some(2), None) => SpellTarget::AttackTarget,
        (Some(3), Some(position)) => SpellTarget::Position(position),
        _ => return Err(ActorSpellError::Malformed),
    };
    Ok(SpellCastIntent {
        spell,
        target,
        aim_at_target: aim.unwrap_or(false),
    })
}

pub fn encode_spell_cast_result(disposition: SpellCastDisposition) -> Vec<u8> {
    let mut output = Vec::with_capacity(2);
    push_varint_field(&mut output, 1, disposition as u64);
    output
}

pub fn decode_spell_cast_result(payload: &[u8]) -> Result<SpellCastDisposition, ActorSpellError> {
    match read_single_enum(payload, MAX_SPELL_CAST_RESULT_BYTES)? {
        1 => Ok(SpellCastDisposition::Cast),
        2 => Ok(SpellCastDisposition::CoolingDown),
        3 => Ok(SpellCastDisposition::LevelTooLow),
        4 => Ok(SpellCastDisposition::MagicLevelTooLow),
        5 => Ok(SpellCastDisposition::NotEnoughMana),
        6 => Ok(SpellCastDisposition::NotEnoughSoul),
        7 => Ok(SpellCastDisposition::NotAvailable),
        8 => Ok(SpellCastDisposition::TargetRequired),
        9 => Ok(SpellCastDisposition::TargetIllegal),
        10 => Ok(SpellCastDisposition::Rejected),
        _ => Err(ActorSpellError::Malformed),
    }
}

/// The SPELL-D8 value bounds, shared by both directions.
fn check_vitals(vitals: &ActorVitals) -> Result<(), ActorSpellError> {
    if vitals.harmony > MAX_HARMONY {
        return Err(ActorSpellError::Malformed);
    }
    let pools = [
        vitals.health,
        vitals.max_health,
        vitals.mana,
        vitals.max_mana,
    ];
    if pools.iter().any(|value| *value > MAX_VITAL_POOL) || vitals.soul > MAX_SOUL {
        return Err(ActorSpellError::LimitExceeded);
    }
    Ok(())
}

/// Encodes the domain-3 delta or snapshot payload (identical schemas, distinct registered types).
/// A value above its SPELL-D8 bound is a server fault and is refused before any byte is emitted.
/// Fields at their proto3 default are omitted, so a non-monk's `harmony` 0 and `serene` false
/// never appear on the wire.
pub fn encode_actor_vitals(vitals: &ActorVitals) -> Result<Vec<u8>, ActorSpellError> {
    check_vitals(vitals)?;
    let mut output = Vec::with_capacity(27);
    push_nonzero_varint_field(&mut output, 1, u64::from(vitals.health));
    push_nonzero_varint_field(&mut output, 2, u64::from(vitals.max_health));
    push_nonzero_varint_field(&mut output, 3, u64::from(vitals.mana));
    push_nonzero_varint_field(&mut output, 4, u64::from(vitals.max_mana));
    push_nonzero_varint_field(&mut output, 5, u64::from(vitals.soul));
    push_nonzero_varint_field(&mut output, 6, u64::from(vitals.harmony));
    push_nonzero_varint_field(&mut output, 7, u64::from(vitals.serene));
    Ok(output)
}

pub fn decode_actor_vitals(payload: &[u8]) -> Result<ActorVitals, ActorSpellError> {
    if payload.len() > MAX_ACTOR_VITALS_BYTES {
        return Err(ActorSpellError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut fields: [Option<u32>; 6] = [None; 6];
    let mut serene = None;
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        match key {
            0x08 | 0x10 | 0x18 | 0x20 | 0x28 | 0x30 => {
                // Field numbers 1..=6 index 0..=5.
                let index =
                    usize::try_from((key >> 3) - 1).map_err(|_| ActorSpellError::Malformed)?;
                let value = read_uint32(payload, &mut cursor)?;
                let slot = fields.get_mut(index).ok_or(ActorSpellError::Malformed)?;
                set_once(slot, value)?;
            }
            0x38 => {
                let value = read_bool(payload, &mut cursor)?;
                set_once(&mut serene, value)?;
            }
            _ => return Err(ActorSpellError::Malformed),
        }
    }
    let [health, max_health, mana, max_mana, soul, harmony] = fields.map(|v| v.unwrap_or(0));
    let vitals = ActorVitals {
        health,
        max_health,
        mana,
        max_mana,
        soul,
        harmony,
        serene: serene.unwrap_or(false),
    };
    check_vitals(&vitals)?;
    Ok(vitals)
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
    const SCHEMA: &str =
        include_str!("../../../docs/contracts/protocol-oteryn/v1/actor_spell_v1.proto");

    fn spell(index: u32) -> NonZeroU32 {
        NonZeroU32::new(index).expect("non-zero spell index")
    }

    fn intent(index: u32, target: SpellTarget, aim_at_target: bool) -> SpellCastIntent {
        SpellCastIntent {
            spell: spell(index),
            target,
            aim_at_target,
        }
    }

    fn position(x: i32, y: i32, floor: i16) -> SpellTarget {
        SpellTarget::Position(SpellTargetPosition { x, y, floor })
    }

    /// Hand-computed canonical proto3 bytes (not produced by the encoder under test).
    fn intent_fixtures() -> Vec<(SpellCastIntent, Vec<u8>)> {
        vec![
            // spell 1, NONE: 08 01 | 10 01.
            (
                intent(1, SpellTarget::None, false),
                vec![0x08, 0x01, 0x10, 0x01],
            ),
            // spell 300 = varint AC 02, ATTACK_TARGET, aim_at_target: 20 01.
            (
                intent(300, SpellTarget::AttackTarget, true),
                vec![0x08, 0xac, 0x02, 0x10, 0x02, 0x20, 0x01],
            ),
            // spell 5, POSITION (x 100 -> zigzag 200 = C8 01, y -1 -> zigzag 1, floor 7 -> zigzag
            // 14 = 0E): 1a 07 | 08 C8 01 10 01 18 0E.
            (
                intent(5, position(100, -1, 7), false),
                vec![
                    0x08, 0x05, 0x10, 0x03, 0x1a, 0x07, 0x08, 0xc8, 0x01, 0x10, 0x01, 0x18, 0x0e,
                ],
            ),
            // POSITION at the origin: the present, empty submessage 1a 00.
            (
                intent(1, position(0, 0, 0), false),
                vec![0x08, 0x01, 0x10, 0x03, 0x1a, 0x00],
            ),
            // Worst case, 28 bytes: spell u32::MAX = FF FF FF FF 0F; x i32::MIN -> zigzag
            // 0xFFFFFFFF = FF FF FF FF 0F; y i32::MAX -> zigzag 0xFFFFFFFE = FE FF FF FF 0F;
            // floor i16::MIN -> zigzag 65535 = FF FF 03; inner length 16 = 0x10; aim 20 01.
            (
                intent(u32::MAX, position(i32::MIN, i32::MAX, i16::MIN), true),
                vec![
                    0x08, 0xff, 0xff, 0xff, 0xff, 0x0f, 0x10, 0x03, 0x1a, 0x10, 0x08, 0xff, 0xff,
                    0xff, 0xff, 0x0f, 0x10, 0xfe, 0xff, 0xff, 0xff, 0x0f, 0x18, 0xff, 0xff, 0x03,
                    0x20, 0x01,
                ],
            ),
        ]
    }

    /// Hand-computed canonical proto3 bytes (not produced by the encoder under test).
    fn vitals_fixtures() -> Vec<(ActorVitals, Vec<u8>)> {
        vec![
            // All zero: every field omitted.
            (ActorVitals::default(), vec![]),
            // A non-monk: 150 = 96 01, 55 = 37, 100 = 64; harmony and serene omitted.
            (
                ActorVitals {
                    health: 150,
                    max_health: 150,
                    mana: 55,
                    max_mana: 55,
                    soul: 100,
                    harmony: 0,
                    serene: false,
                },
                vec![
                    0x08, 0x96, 0x01, 0x10, 0x96, 0x01, 0x18, 0x37, 0x20, 0x37, 0x28, 0x64,
                ],
            ),
            // A Serene monk at full Harmony, mana and soul 0 omitted: 30 05 | 38 01.
            (
                ActorVitals {
                    health: 1,
                    max_health: 2,
                    mana: 0,
                    max_mana: 3,
                    soul: 0,
                    harmony: 5,
                    serene: true,
                },
                vec![0x08, 0x01, 0x10, 0x02, 0x20, 0x03, 0x30, 0x05, 0x38, 0x01],
            ),
            // SPELL-D8 worst case, 27 bytes: 2^28 - 1 = FF FF FF 7F, 16383 = FF 7F.
            (
                ActorVitals {
                    health: MAX_VITAL_POOL,
                    max_health: MAX_VITAL_POOL,
                    mana: MAX_VITAL_POOL,
                    max_mana: MAX_VITAL_POOL,
                    soul: MAX_SOUL,
                    harmony: MAX_HARMONY,
                    serene: true,
                },
                vec![
                    0x08, 0xff, 0xff, 0xff, 0x7f, 0x10, 0xff, 0xff, 0xff, 0x7f, 0x18, 0xff, 0xff,
                    0xff, 0x7f, 0x20, 0xff, 0xff, 0xff, 0x7f, 0x28, 0xff, 0x7f, 0x30, 0x05, 0x38,
                    0x01,
                ],
            ),
        ]
    }

    #[test]
    fn spell_cast_intent_matches_independent_fixtures() {
        for (value, bytes) in intent_fixtures() {
            assert_eq!(encode_spell_cast_intent(&value), bytes, "{value:?}");
            assert!(bytes.len() <= MAX_SPELL_CAST_INTENT_BYTES);
            assert_eq!(decode_spell_cast_intent(&bytes), Ok(value), "{bytes:02x?}");
        }
        // The canonical worst case stays inside the SPELL-D7 bound.
        let worst = encode_spell_cast_intent(&intent(
            u32::MAX,
            position(i32::MIN, i32::MIN, i16::MIN),
            true,
        ));
        assert_eq!(worst.len(), 28);
        // Field order is not significant, and explicit proto3 defaults decode as omitted ones.
        assert_eq!(
            decode_spell_cast_intent(&[0x20, 0x01, 0x10, 0x02, 0x08, 0x07]),
            Ok(intent(7, SpellTarget::AttackTarget, true))
        );
        assert_eq!(
            decode_spell_cast_intent(&[0x08, 0x01, 0x10, 0x01, 0x20, 0x00]),
            Ok(intent(1, SpellTarget::None, false))
        );
        assert_eq!(
            decode_spell_cast_intent(&[
                0x08, 0x01, 0x10, 0x03, 0x1a, 0x06, 0x08, 0x00, 0x10, 0x00, 0x18, 0x00
            ]),
            Ok(intent(1, position(0, 0, 0), false))
        );
        // The int16 floor edges are accepted: 32767 -> zigzag 65534 = FE FF 03.
        assert_eq!(
            decode_spell_cast_intent(&[0x08, 0x01, 0x10, 0x03, 0x1a, 0x04, 0x18, 0xfe, 0xff, 0x03]),
            Ok(intent(1, position(0, 0, i16::MAX), false))
        );
    }

    #[test]
    fn spell_cast_intent_refuses_everything_else() {
        let malformed: &[(&str, &[u8])] = &[
            ("empty", &[]),
            ("spell absent", &[0x10, 0x01]),
            ("spell zero", &[0x08, 0x00, 0x10, 0x01]),
            (
                "spell above u32",
                &[0x08, 0x80, 0x80, 0x80, 0x80, 0x10, 0x10, 0x01],
            ),
            ("target absent", &[0x08, 0x01]),
            ("target zero", &[0x08, 0x01, 0x10, 0x00]),
            ("target unknown", &[0x08, 0x01, 0x10, 0x04]),
            ("spell repeated", &[0x08, 0x01, 0x08, 0x02, 0x10, 0x01]),
            ("target repeated", &[0x08, 0x01, 0x10, 0x01, 0x10, 0x01]),
            (
                "aim repeated",
                &[0x08, 0x01, 0x10, 0x01, 0x20, 0x01, 0x20, 0x01],
            ),
            (
                "position repeated",
                &[0x08, 0x01, 0x10, 0x03, 0x1a, 0x00, 0x1a, 0x00],
            ),
            ("unknown field 5", &[0x08, 0x01, 0x10, 0x01, 0x28, 0x01]),
            ("spell wrong wire type", &[0x0a, 0x00, 0x10, 0x01]),
            ("aim not a bool", &[0x08, 0x01, 0x10, 0x01, 0x20, 0x02]),
            ("position with NONE", &[0x08, 0x01, 0x10, 0x01, 0x1a, 0x00]),
            (
                "position with ATTACK_TARGET",
                &[0x08, 0x01, 0x10, 0x02, 0x1a, 0x00],
            ),
            ("POSITION without position", &[0x08, 0x01, 0x10, 0x03]),
            // 32768 -> zigzag 65536 = 80 80 04; -32769 -> zigzag 65537 = 81 80 04.
            (
                "floor above int16",
                &[0x08, 0x01, 0x10, 0x03, 0x1a, 0x04, 0x18, 0x80, 0x80, 0x04],
            ),
            (
                "floor below int16",
                &[0x08, 0x01, 0x10, 0x03, 0x1a, 0x04, 0x18, 0x81, 0x80, 0x04],
            ),
            (
                "x above sint32",
                &[
                    0x08, 0x01, 0x10, 0x03, 0x1a, 0x06, 0x08, 0x80, 0x80, 0x80, 0x80, 0x10,
                ],
            ),
            (
                "position unknown field",
                &[0x08, 0x01, 0x10, 0x03, 0x1a, 0x02, 0x20, 0x01],
            ),
            (
                "position field repeated",
                &[0x08, 0x01, 0x10, 0x03, 0x1a, 0x04, 0x08, 0x02, 0x08, 0x02],
            ),
            (
                "position length past end",
                &[0x08, 0x01, 0x10, 0x03, 0x1a, 0x05, 0x08],
            ),
            ("truncated key", &[0x08]),
            ("truncated varint", &[0x08, 0x81]),
            (
                "varint over 10 bytes",
                &[
                    0x08, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f, 0x10, 0x01,
                ],
            ),
        ];
        for (case, bytes) in malformed {
            assert_eq!(
                decode_spell_cast_intent(bytes),
                Err(ActorSpellError::Malformed),
                "{case}"
            );
        }
        assert_eq!(
            decode_spell_cast_intent(&[0; MAX_SPELL_CAST_INTENT_BYTES + 1]),
            Err(ActorSpellError::LimitExceeded)
        );
    }

    #[test]
    fn spell_cast_result_matches_independent_fixtures_and_refuses_unknown() {
        let fixtures = [
            (SpellCastDisposition::Cast, [0x08, 0x01]),
            (SpellCastDisposition::CoolingDown, [0x08, 0x02]),
            (SpellCastDisposition::LevelTooLow, [0x08, 0x03]),
            (SpellCastDisposition::MagicLevelTooLow, [0x08, 0x04]),
            (SpellCastDisposition::NotEnoughMana, [0x08, 0x05]),
            (SpellCastDisposition::NotEnoughSoul, [0x08, 0x06]),
            (SpellCastDisposition::NotAvailable, [0x08, 0x07]),
            (SpellCastDisposition::TargetRequired, [0x08, 0x08]),
            (SpellCastDisposition::TargetIllegal, [0x08, 0x09]),
            (SpellCastDisposition::Rejected, [0x08, 0x0a]),
        ];
        for (disposition, bytes) in fixtures {
            assert_eq!(encode_spell_cast_result(disposition), bytes);
            assert!(bytes.len() <= MAX_SPELL_CAST_RESULT_BYTES);
            assert_eq!(decode_spell_cast_result(&bytes), Ok(disposition));
        }
        for bad in [
            &[][..],                       // missing disposition
            &[0x08, 0x00][..],             // zero enum
            &[0x08, 0x0b][..],             // unknown enum
            &[0x08, 0x01, 0x08, 0x01][..], // repeated field
            &[0x10, 0x01][..],             // unknown field
            &[0x08][..],                   // truncated
        ] {
            assert_eq!(
                decode_spell_cast_result(bad),
                Err(ActorSpellError::Malformed),
                "{bad:02x?}"
            );
        }
        assert_eq!(
            decode_spell_cast_result(&[0x08, 0x81, 0x80, 0x80, 0x00]),
            Err(ActorSpellError::LimitExceeded)
        );
    }

    #[test]
    fn actor_vitals_matches_independent_fixtures() {
        for (value, bytes) in vitals_fixtures() {
            assert_eq!(encode_actor_vitals(&value), Ok(bytes.clone()), "{value:?}");
            assert!(bytes.len() <= MAX_ACTOR_VITALS_BYTES);
            assert_eq!(decode_actor_vitals(&bytes), Ok(value), "{bytes:02x?}");
        }
        // Explicit proto3 defaults decode as omitted ones, in any field order.
        assert_eq!(
            decode_actor_vitals(&[0x38, 0x00, 0x30, 0x00, 0x08, 0x00]),
            Ok(ActorVitals::default())
        );
        assert_eq!(
            decode_actor_vitals(&[0x30, 0x03, 0x08, 0x0a]),
            Ok(ActorVitals {
                health: 10,
                harmony: 3,
                ..ActorVitals::default()
            })
        );
    }

    #[test]
    fn actor_vitals_bounds_fail_closed_in_both_directions() {
        let at_bounds = ActorVitals {
            health: MAX_VITAL_POOL,
            max_health: MAX_VITAL_POOL,
            mana: MAX_VITAL_POOL,
            max_mana: MAX_VITAL_POOL,
            soul: MAX_SOUL,
            harmony: MAX_HARMONY,
            serene: true,
        };
        assert_eq!(encode_actor_vitals(&at_bounds).map(|b| b.len()), Ok(27));
        // The encoder refuses every value one above its bound as a server fault.
        let over: [(&str, ActorVitals, ActorSpellError); 6] = [
            (
                "health",
                ActorVitals {
                    health: MAX_VITAL_POOL + 1,
                    ..at_bounds
                },
                ActorSpellError::LimitExceeded,
            ),
            (
                "max_health",
                ActorVitals {
                    max_health: MAX_VITAL_POOL + 1,
                    ..at_bounds
                },
                ActorSpellError::LimitExceeded,
            ),
            (
                "mana",
                ActorVitals {
                    mana: MAX_VITAL_POOL + 1,
                    ..at_bounds
                },
                ActorSpellError::LimitExceeded,
            ),
            (
                "max_mana",
                ActorVitals {
                    max_mana: u32::MAX,
                    ..at_bounds
                },
                ActorSpellError::LimitExceeded,
            ),
            (
                "soul",
                ActorVitals {
                    soul: MAX_SOUL + 1,
                    ..at_bounds
                },
                ActorSpellError::LimitExceeded,
            ),
            (
                "harmony",
                ActorVitals {
                    harmony: MAX_HARMONY + 1,
                    ..at_bounds
                },
                ActorSpellError::Malformed,
            ),
        ];
        for (case, value, error) in over {
            assert_eq!(encode_actor_vitals(&value), Err(error), "{case}");
        }
        // The decoder rejects the same values: 2^28 = 80 80 80 80 01, 16384 = 80 80 01.
        let rejected: &[(&str, &[u8], ActorSpellError)] = &[
            (
                "health 2^28",
                &[0x08, 0x80, 0x80, 0x80, 0x80, 0x01],
                ActorSpellError::LimitExceeded,
            ),
            (
                "max_health 2^28",
                &[0x10, 0x80, 0x80, 0x80, 0x80, 0x01],
                ActorSpellError::LimitExceeded,
            ),
            (
                "mana 2^28",
                &[0x18, 0x80, 0x80, 0x80, 0x80, 0x01],
                ActorSpellError::LimitExceeded,
            ),
            (
                "max_mana u32::MAX",
                &[0x20, 0xff, 0xff, 0xff, 0xff, 0x0f],
                ActorSpellError::LimitExceeded,
            ),
            (
                "soul 16384",
                &[0x28, 0x80, 0x80, 0x01],
                ActorSpellError::LimitExceeded,
            ),
            ("harmony 6", &[0x30, 0x06], ActorSpellError::Malformed),
            ("serene 2", &[0x38, 0x02], ActorSpellError::Malformed),
            (
                "health above u32",
                &[0x08, 0x80, 0x80, 0x80, 0x80, 0x10],
                ActorSpellError::Malformed,
            ),
            (
                "repeated field",
                &[0x08, 0x01, 0x08, 0x01],
                ActorSpellError::Malformed,
            ),
            (
                "repeated serene",
                &[0x38, 0x01, 0x38, 0x01],
                ActorSpellError::Malformed,
            ),
            ("unknown field 8", &[0x40, 0x01], ActorSpellError::Malformed),
            ("field 0", &[0x00, 0x01], ActorSpellError::Malformed),
            ("wrong wire type", &[0x0a, 0x00], ActorSpellError::Malformed),
            ("truncated", &[0x08], ActorSpellError::Malformed),
        ];
        for (case, bytes, error) in rejected {
            assert_eq!(decode_actor_vitals(bytes), Err(*error), "{case}");
        }
        assert_eq!(
            decode_actor_vitals(&[0; MAX_ACTOR_VITALS_BYTES + 1]),
            Err(ActorSpellError::LimitExceeded)
        );
    }

    #[test]
    fn registries_bind_the_spell_cast_wire_ids_and_limits() {
        let protocol: Value = serde_json::from_str(PROTOCOL_REGISTRY).expect("protocol registry");
        let commands = protocol["command_types"].as_array().expect("command_types");
        let command = commands
            .iter()
            .find(|command| command["id"] == COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT)
            .expect("WORLD_ACTOR_SPELL_CAST_INTENT registered");
        assert_eq!(command["name"], "WORLD_ACTOR_SPELL_CAST_INTENT");
        assert_eq!(
            command["payload_schema"],
            "docs/contracts/protocol-oteryn/v1/actor_spell_v1.proto#WorldActorSpellCastIntentV1"
        );
        assert_eq!(
            command["result_schema"],
            "docs/contracts/protocol-oteryn/v1/actor_spell_v1.proto#WorldActorSpellCastResultV1"
        );
        assert_eq!(
            command["max_payload_bytes"],
            MAX_SPELL_CAST_INTENT_BYTES as u64
        );
        assert_eq!(
            command["max_result_payload_bytes"],
            MAX_SPELL_CAST_RESULT_BYTES as u64
        );

        let domains = protocol["state_domains"].as_array().expect("state_domains");
        let domain = domains
            .iter()
            .find(|domain| domain["id"] == STATE_DOMAIN_ACTOR_VITALS)
            .expect("ACTOR_VITALS registered");
        assert_eq!(domain["name"], "ACTOR_VITALS");
        assert_eq!(domain["owner"], "current ChannelRuntime");
        for (types, id, name) in [
            (
                "delta_types",
                DELTA_TYPE_ACTOR_VITALS_V1,
                "ACTOR_VITALS_DELTA_V1",
            ),
            (
                "snapshot_types",
                SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                "ACTOR_VITALS_SNAPSHOT_V1",
            ),
        ] {
            let entries = domain[types].as_array().expect(types);
            assert_eq!(entries.len(), 1, "{types}");
            assert_eq!(entries[0]["id"], id);
            assert_eq!(entries[0]["name"], name);
            assert_eq!(
                entries[0]["payload_schema"],
                "docs/contracts/protocol-oteryn/v1/actor_spell_v1.proto#ActorVitalsV1"
            );
            assert_eq!(
                entries[0]["max_payload_bytes"],
                MAX_ACTOR_VITALS_BYTES as u64
            );
        }

        // Every registered schema anchor names a message of the proto file.
        for message in [
            "WorldActorSpellCastIntentV1",
            "WorldActorSpellCastResultV1",
            "ActorVitalsV1",
        ] {
            assert!(
                SCHEMA.contains(&format!("message {message} {{")),
                "{message}"
            );
        }

        let resources: Value = serde_json::from_str(RESOURCE_REGISTRY).expect("resource registry");
        let entries = resources["entries"].as_array().expect("entries");
        let limit = |id: &str| {
            entries
                .iter()
                .find(|entry| entry["id"] == id)
                .and_then(|entry| entry["hard_maximum"].as_u64())
        };
        assert_eq!(limit("SPELL-RL-01"), Some(1));
        assert_eq!(limit("SPELL-RL-02"), Some(2));
        assert_eq!(limit("SPELL-RL-02"), limit("ABILITY01-EFFECT-PLAN-ENTRIES"));
        assert_eq!(limit("SPELL-RL-03"), Some(1));
    }
}
