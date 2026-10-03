//! Analyser fact batch codecs and the bounded pending queue (ANALYSER-WIRE-1).
//!
//! Schema: `docs/contracts/protocol-oteryn/v1/analyser_v1.proto`, from ANALYSERS-0 §4 and §6.
//! Capability 10 `ANALYSER_V1` and state domain 15 `ACTOR_ANALYSER` are leased by the #1622
//! control plane; the server does not offer the capability before ANALYSER-EMIT-1.
//!
//! Decoding is strict, and encoding refuses the same values as a server fault before any byte is
//! emitted: an empty or doubled oneof, zero or unknown enums, zero values, counts or item types,
//! a race above the Bestiary bound or on a non-creature source, a healing element, an empty batch,
//! any value over its bound, and unknown or repeated fields all fail closed.

use std::collections::VecDeque;
use std::num::{NonZeroU16, NonZeroU32};

use crate::bestiary::MAX_BESTIARY_RACE;
pub use crate::charm_wire::CyclopediaWireError as AnalyserWireError;
use crate::charm_wire::{
    WireResult, push_message_field, push_nonzero_varint_field, push_varint_field, read_bytes,
    read_uint32_fields, read_varint, set_once,
};

/// Registered capability `ANALYSER_V1`: state domain 15, no command type.
pub const CAPABILITY_ANALYSER_V1: u32 = 10;
/// Registered state domain `ACTOR_ANALYSER` (capability 10).
pub const STATE_DOMAIN_ACTOR_ANALYSER: u32 = 15;
/// Delta type 1: one batch of facts ([`encode_analyser_facts`]).
pub const DELTA_TYPE_ANALYSER_FACTS_V1: u32 = 1;
/// Snapshot type 1: always empty; the envelope carries the revision.
pub const SNAPSHOT_TYPE_ANALYSER_V1: u32 = 1;

/// `ANALYSERS0-RL-01`: facts per batch.
pub const MAX_ANALYSER_BATCH_FACTS: usize = 64;
/// `ANALYSERS0-RL-02`: items per `kill_loot`.
pub const MAX_ANALYSER_LOOT_ITEMS: usize = 32;
/// `ANALYSERS0-RL-03`: bytes per batch. The batch builder also stops before this bound.
pub const MAX_ANALYSER_BATCH_BYTES: usize = 4_096;
/// The largest `AnalyserFactV1`, measured: a `kill_loot` with race 1 + 2, 32 items of
/// 1 + 1 + (1 + 5) + (1 + 3) and gold 1 + 10 is 398 bytes, plus its oneof tag and 2-byte length.
pub const MAX_ANALYSER_FACT_BYTES: usize = 401;
/// The ANALYSERS-0 §6 pending cap per session; above it the oldest facts are dropped.
pub const MAX_ANALYSER_PENDING_FACTS: usize = 1_024;

/// `AnalyserElement`: the content damage types, healing excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalyserElement {
    Physical = 1,
    Fire = 2,
    Earth = 3,
    Energy = 4,
    Ice = 5,
    Holy = 6,
    Death = 7,
    LifeDrain = 8,
    ManaDrain = 9,
    Drowning = 10,
    Untyped = 11,
}

impl AnalyserElement {
    fn from_wire(value: u32) -> WireResult<Self> {
        Ok(match value {
            1 => Self::Physical,
            2 => Self::Fire,
            3 => Self::Earth,
            4 => Self::Energy,
            5 => Self::Ice,
            6 => Self::Holy,
            7 => Self::Death,
            8 => Self::LifeDrain,
            9 => Self::ManaDrain,
            10 => Self::Drowning,
            11 => Self::Untyped,
            _ => return Err(AnalyserWireError::Malformed),
        })
    }
}

/// A Bestiary race index, `1..=`[`MAX_BESTIARY_RACE`]; `None` for a creature without one.
pub type AnalyserRace = Option<NonZeroU32>;

/// `AnalyserItemCountV1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalyserItemCount {
    pub item_type: NonZeroU32,
    pub count: NonZeroU16,
}

/// `AnalyserImpactV1`: damage the character dealt or healing it gave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalyserImpact {
    Damage {
        value: NonZeroU32,
        element: AnalyserElement,
    },
    Healing(NonZeroU32),
}

/// `AnalyserDamageSource`; only a creature carries a race.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalyserDamageSource {
    Creature(AnalyserRace),
    Player,
    None,
}

/// `AnalyserFactV1`: exactly one fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalyserFact {
    Experience {
        raw: u64,
        gained: u64,
    },
    KillLoot {
        race: AnalyserRace,
        corpse_items: Vec<AnalyserItemCount>,
        gold: u64,
    },
    SupplyUsed(AnalyserItemCount),
    Impact(AnalyserImpact),
    DamageInput {
        value: NonZeroU32,
        element: AnalyserElement,
        source: AnalyserDamageSource,
    },
    /// Facts lost to the pending cap since the last batch.
    Dropped(NonZeroU32),
}

/// A wire race: 0 is none, above [`MAX_BESTIARY_RACE`] fails closed.
fn race_from_wire(value: u64) -> WireResult<AnalyserRace> {
    if value > u64::from(MAX_BESTIARY_RACE) {
        return Err(AnalyserWireError::LimitExceeded);
    }
    Ok(u32::try_from(value).ok().and_then(NonZeroU32::new))
}

fn race_to_wire(race: AnalyserRace) -> u64 {
    race.map_or(0, |race| u64::from(race.get()))
}

fn nonzero(value: u32) -> WireResult<NonZeroU32> {
    NonZeroU32::new(value).ok_or(AnalyserWireError::Malformed)
}

fn encode_item(item: &AnalyserItemCount) -> Vec<u8> {
    let mut output = Vec::with_capacity(10);
    push_varint_field(&mut output, 1, u64::from(item.item_type.get()));
    push_varint_field(&mut output, 2, u64::from(item.count.get()));
    output
}

fn decode_item(input: &[u8]) -> WireResult<AnalyserItemCount> {
    let [item_type, count] = read_uint32_fields::<2>(input)?;
    Ok(AnalyserItemCount {
        item_type: nonzero(item_type)?,
        count: u16::try_from(count)
            .map_err(|_| AnalyserWireError::LimitExceeded)
            .and_then(|count| NonZeroU16::new(count).ok_or(AnalyserWireError::Malformed))?,
    })
}

/// Encodes one `AnalyserFactV1`. A fact outside the bounds is a server fault.
pub fn encode_analyser_fact(fact: &AnalyserFact) -> WireResult<Vec<u8>> {
    let mut inner = Vec::new();
    let field = match fact {
        AnalyserFact::Experience { raw, gained } => {
            push_nonzero_varint_field(&mut inner, 1, *raw);
            push_nonzero_varint_field(&mut inner, 2, *gained);
            1
        }
        AnalyserFact::KillLoot {
            race,
            corpse_items,
            gold,
        } => {
            race_from_wire(race_to_wire(*race))?;
            if corpse_items.len() > MAX_ANALYSER_LOOT_ITEMS {
                return Err(AnalyserWireError::LimitExceeded);
            }
            push_nonzero_varint_field(&mut inner, 1, race_to_wire(*race));
            for item in corpse_items {
                push_message_field(&mut inner, 2, &encode_item(item));
            }
            push_nonzero_varint_field(&mut inner, 3, *gold);
            2
        }
        AnalyserFact::SupplyUsed(item) => {
            inner = encode_item(item);
            3
        }
        AnalyserFact::Impact(impact) => {
            let (kind, value, element) = match impact {
                AnalyserImpact::Damage { value, element } => (1, value, *element as u64),
                AnalyserImpact::Healing(value) => (2, value, 0),
            };
            push_varint_field(&mut inner, 1, kind);
            push_varint_field(&mut inner, 2, u64::from(value.get()));
            push_nonzero_varint_field(&mut inner, 3, element);
            4
        }
        AnalyserFact::DamageInput {
            value,
            element,
            source,
        } => {
            let (source, race) = match source {
                AnalyserDamageSource::Creature(race) => (1, *race),
                AnalyserDamageSource::Player => (2, None),
                AnalyserDamageSource::None => (3, None),
            };
            race_from_wire(race_to_wire(race))?;
            push_varint_field(&mut inner, 1, u64::from(value.get()));
            push_varint_field(&mut inner, 2, *element as u64);
            push_varint_field(&mut inner, 3, source);
            push_nonzero_varint_field(&mut inner, 4, race_to_wire(race));
            5
        }
        AnalyserFact::Dropped(count) => {
            push_varint_field(&mut inner, 1, u64::from(count.get()));
            6
        }
    };
    let mut output = Vec::with_capacity(inner.len() + 3);
    push_message_field(&mut output, field, &inner);
    Ok(output)
}

/// Decodes one `AnalyserFactV1`.
pub fn decode_analyser_fact(payload: &[u8]) -> WireResult<AnalyserFact> {
    if payload.len() > MAX_ANALYSER_FACT_BYTES {
        return Err(AnalyserWireError::LimitExceeded);
    }
    let mut fact = None;
    let mut cursor = 0;
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        if key & 0x07 != 2 || !(1..=6).contains(&(key >> 3)) {
            return Err(AnalyserWireError::Malformed);
        }
        let inner = read_bytes(payload, &mut cursor)?;
        set_once(&mut fact, decode_variant(key >> 3, inner)?)?;
    }
    fact.ok_or(AnalyserWireError::Malformed)
}

/// Reads `AnalyserExperienceV1` (varints 1 and 2) or, with `loot`, `AnalyserKillLootV1` (varints 1
/// and 3, items in 2). An omitted varint reads as 0.
fn read_varints_and_items(
    inner: &[u8],
    loot: bool,
) -> WireResult<([u64; 3], Vec<AnalyserItemCount>)> {
    let (mut varints, mut items) = ([None; 3], Vec::new());
    let mut cursor = 0;
    while cursor < inner.len() {
        let slot = match (read_varint(inner, &mut cursor)?, loot) {
            (0x08, _) => 0,
            (0x10, false) => 1,
            (0x18, true) => 2,
            (0x12, true) if items.len() < MAX_ANALYSER_LOOT_ITEMS => {
                items.push(decode_item(read_bytes(inner, &mut cursor)?)?);
                continue;
            }
            (0x12, true) => return Err(AnalyserWireError::LimitExceeded),
            _ => return Err(AnalyserWireError::Malformed),
        };
        set_once(&mut varints[slot], read_varint(inner, &mut cursor)?)?;
    }
    Ok((varints.map(|value| value.unwrap_or(0)), items))
}

fn decode_variant(field: u64, inner: &[u8]) -> WireResult<AnalyserFact> {
    Ok(match field {
        1 => {
            let ([raw, gained, _], _) = read_varints_and_items(inner, false)?;
            AnalyserFact::Experience { raw, gained }
        }
        2 => {
            let ([race, _, gold], corpse_items) = read_varints_and_items(inner, true)?;
            AnalyserFact::KillLoot {
                race: race_from_wire(race)?,
                corpse_items,
                gold,
            }
        }
        3 => AnalyserFact::SupplyUsed(decode_item(inner)?),
        4 => {
            let [kind, value, element] = read_uint32_fields::<3>(inner)?;
            let value = nonzero(value)?;
            AnalyserFact::Impact(match (kind, element) {
                (1, element) => AnalyserImpact::Damage {
                    value,
                    element: AnalyserElement::from_wire(element)?,
                },
                (2, 0) => AnalyserImpact::Healing(value),
                _ => return Err(AnalyserWireError::Malformed),
            })
        }
        5 => {
            let [value, element, source, race] = read_uint32_fields::<4>(inner)?;
            AnalyserFact::DamageInput {
                value: nonzero(value)?,
                element: AnalyserElement::from_wire(element)?,
                source: match (source, race) {
                    (1, race) => AnalyserDamageSource::Creature(race_from_wire(race.into())?),
                    (2, 0) => AnalyserDamageSource::Player,
                    (3, 0) => AnalyserDamageSource::None,
                    _ => return Err(AnalyserWireError::Malformed),
                },
            }
        }
        _ => {
            let [count] = read_uint32_fields::<1>(inner)?;
            AnalyserFact::Dropped(nonzero(count)?)
        }
    })
}

/// Encodes a delta type 1 batch: `1..=`[`MAX_ANALYSER_BATCH_FACTS`] facts within
/// [`MAX_ANALYSER_BATCH_BYTES`].
pub fn encode_analyser_facts(facts: &[AnalyserFact]) -> WireResult<Vec<u8>> {
    if facts.is_empty() {
        return Err(AnalyserWireError::Malformed);
    }
    if facts.len() > MAX_ANALYSER_BATCH_FACTS {
        return Err(AnalyserWireError::LimitExceeded);
    }
    let mut output = Vec::new();
    for fact in facts {
        push_message_field(&mut output, 1, &encode_analyser_fact(fact)?);
    }
    if output.len() > MAX_ANALYSER_BATCH_BYTES {
        return Err(AnalyserWireError::LimitExceeded);
    }
    Ok(output)
}

pub fn decode_analyser_facts(payload: &[u8]) -> WireResult<Vec<AnalyserFact>> {
    if payload.len() > MAX_ANALYSER_BATCH_BYTES {
        return Err(AnalyserWireError::LimitExceeded);
    }
    let mut facts = Vec::new();
    let mut cursor = 0;
    while cursor < payload.len() {
        if read_varint(payload, &mut cursor)? != 0x0a {
            return Err(AnalyserWireError::Malformed);
        }
        if facts.len() == MAX_ANALYSER_BATCH_FACTS {
            return Err(AnalyserWireError::LimitExceeded);
        }
        facts.push(decode_analyser_fact(read_bytes(payload, &mut cursor)?)?);
    }
    (!facts.is_empty())
        .then_some(facts)
        .ok_or(AnalyserWireError::Malformed)
}

/// The snapshot payload is empty; anything else fails closed.
pub fn decode_analyser_snapshot(payload: &[u8]) -> WireResult<()> {
    payload
        .is_empty()
        .then_some(())
        .ok_or(AnalyserWireError::Malformed)
}

/// One session's facts not yet sent, at most [`MAX_ANALYSER_PENDING_FACTS`]; above the cap the
/// oldest is dropped and counted, and the next batch opens with `dropped {count}`.
#[derive(Debug, Default)]
pub struct AnalyserPendingQueue {
    /// Encoded `AnalyserFactV1` messages, oldest first.
    facts: VecDeque<Vec<u8>>,
    dropped: u32,
}

impl AnalyserPendingQueue {
    pub fn len(&self) -> usize {
        self.facts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.facts.is_empty() && self.dropped == 0
    }

    /// Queues one fact; a fact outside the bounds is refused and nothing changes.
    pub fn push(&mut self, fact: &AnalyserFact) -> WireResult<()> {
        let encoded = encode_analyser_fact(fact)?;
        if self.facts.len() == MAX_ANALYSER_PENDING_FACTS {
            self.facts.pop_front();
            self.dropped = self.dropped.saturating_add(1);
        }
        self.facts.push_back(encoded);
        Ok(())
    }

    /// Takes the next delta type 1 payload: `dropped` first if any, then the oldest facts while
    /// the batch stays within [`MAX_ANALYSER_BATCH_FACTS`] and [`MAX_ANALYSER_BATCH_BYTES`]. The
    /// rest stay queued for the next sync unit. `None` when nothing is pending.
    pub fn take_batch(&mut self) -> Option<Vec<u8>> {
        let mut output = Vec::new();
        let mut count = 0;
        if let Some(dropped) = NonZeroU32::new(std::mem::take(&mut self.dropped)) {
            let fact = encode_analyser_fact(&AnalyserFact::Dropped(dropped)).ok()?;
            push_message_field(&mut output, 1, &fact);
            count = 1;
        }
        while let Some(next) = self.facts.front() {
            // Each entry is a tag, a length of at most 2 bytes and the fact.
            if count == MAX_ANALYSER_BATCH_FACTS
                || output.len() + 3 + next.len() > MAX_ANALYSER_BATCH_BYTES
            {
                break;
            }
            push_message_field(&mut output, 1, next);
            self.facts.pop_front();
            count += 1;
        }
        (count > 0).then_some(output)
    }
}

#[cfg(test)]
#[path = "analyser_tests.rs"]
mod tests;
