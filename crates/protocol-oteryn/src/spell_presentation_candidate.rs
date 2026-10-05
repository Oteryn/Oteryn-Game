//! SPELL-PRESENT-0 review candidate payloads. No capability/domain number is allocated here.
//! Candidate local framing revision 1 is closed and bounded; outer FND-02 routing/context is
//! deliberately not invented. A later registry approval must bind these payloads explicitly.
use crate::world_spatial::ActorPosition;
use crate::world_spatial_entities::EntityRef;
use std::collections::BTreeSet;
pub const MAX_EVENTS: usize = 1024;
pub const MAX_EVENT_BYTES: usize = 128;
pub const MAX_BATCH_BYTES: usize = 131_136;
/// FND-02 repeated-field ceiling only. Composition must additionally enforce its actual
/// measured active spell-book limit +16; no SPELL-RL-04 allocation is invented here.
pub const MAX_COOLDOWNS: usize = 4096;
pub const MAX_CLOCK_MS: u64 = (1 << 42) - 1;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Malformed,
    Limit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Own,
    Others,
    Creatures,
    Global,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    DamageDealt,
    DamageReceived,
    DamageOthers,
    Healed,
    HealedOthers,
    Mana,
    Experience,
    ExperienceOthers,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Red,
    LightGreen,
    LightGrey,
    Orange,
    Purple,
    SkyBlue,
    Yellow,
    DarkRed,
    LightBlue,
    Blue,
    PastelRed,
    MayaBlue,
    White,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Value {
    pub value: u64,
    pub color: Color,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    Effect {
        position: ActorPosition,
        id: u16,
        source: Source,
    },
    Missile {
        from: ActorPosition,
        to: ActorPosition,
        id: u16,
        source: Source,
    },
    Words {
        caster: EntityRef,
        position: ActorPosition,
        spell: u32,
    },
    Value {
        kind: ValueKind,
        position: ActorPosition,
        target: EntityRef,
        attacker: Option<EntityRef>,
        primary: Value,
        secondary: Value,
    },
    Sound {
        position: ActorPosition,
        id: u16,
        source: Source,
        secondary: Option<u16>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub decision_ordinal: u64,
    pub emission_ordinal: u16,
    pub body: Body,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Batch {
    pub content: [u8; 32],
    pub sync_unit: u64,
    pub events: Vec<Event>,
}
struct Read<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Read<'a> {
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.at.checked_add(n).ok_or(Error::Limit)?;
        let v = self.bytes.get(self.at..end).ok_or(Error::Malformed)?;
        self.at = end;
        Ok(v)
    }
    fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.bytes(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_le_bytes(
            self.bytes(2)?.try_into().map_err(|_| Error::Malformed)?,
        ))
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(
            self.bytes(4)?.try_into().map_err(|_| Error::Malformed)?,
        ))
    }
    fn u64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_le_bytes(
            self.bytes(8)?.try_into().map_err(|_| Error::Malformed)?,
        ))
    }
    fn pos(&mut self) -> Result<ActorPosition, Error> {
        Ok(ActorPosition {
            x: self.u32()? as i32,
            y: self.u32()? as i32,
            floor: self.u16()? as i16,
        })
    }
    fn entity(&mut self) -> Result<EntityRef, Error> {
        let identity = self.bytes(16)?.try_into().map_err(|_| Error::Malformed)?;
        let generation = self.u64()?;
        if identity == [0; 16] || generation == 0 {
            return Err(Error::Malformed);
        }
        Ok(EntityRef {
            identity,
            generation,
        })
    }
    fn source(&mut self) -> Result<Source, Error> {
        match self.u8()? {
            1 => Ok(Source::Own),
            2 => Ok(Source::Others),
            3 => Ok(Source::Creatures),
            4 => Ok(Source::Global),
            _ => Err(Error::Malformed),
        }
    }
    fn value(&mut self) -> Result<Value, Error> {
        let value = self.u64()?;
        let color = match self.u8()? {
            1 => Color::Red,
            2 => Color::LightGreen,
            3 => Color::LightGrey,
            4 => Color::Orange,
            5 => Color::Purple,
            6 => Color::SkyBlue,
            7 => Color::Yellow,
            8 => Color::DarkRed,
            9 => Color::LightBlue,
            10 => Color::Blue,
            11 => Color::PastelRed,
            12 => Color::MayaBlue,
            13 => Color::White,
            _ => return Err(Error::Malformed),
        };
        Ok(Value { value, color })
    }
    fn end(&self) -> Result<(), Error> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(Error::Malformed)
        }
    }
}
fn pos(out: &mut Vec<u8>, p: ActorPosition) {
    out.extend(p.x.to_le_bytes());
    out.extend(p.y.to_le_bytes());
    out.extend(p.floor.to_le_bytes());
}
fn entity(out: &mut Vec<u8>, e: EntityRef) -> Result<(), Error> {
    if e.identity == [0; 16] || e.generation == 0 {
        return Err(Error::Malformed);
    }
    out.extend(e.identity);
    out.extend(e.generation.to_le_bytes());
    Ok(())
}
fn source(s: Source) -> u8 {
    match s {
        Source::Own => 1,
        Source::Others => 2,
        Source::Creatures => 3,
        Source::Global => 4,
    }
}
fn kind(k: ValueKind) -> u8 {
    match k {
        ValueKind::DamageDealt => 1,
        ValueKind::DamageReceived => 2,
        ValueKind::DamageOthers => 3,
        ValueKind::Healed => 4,
        ValueKind::HealedOthers => 5,
        ValueKind::Mana => 6,
        ValueKind::Experience => 7,
        ValueKind::ExperienceOthers => 8,
    }
}
fn value(out: &mut Vec<u8>, v: Value) {
    out.extend(v.value.to_le_bytes());
    out.push(match v.color {
        Color::Red => 1,
        Color::LightGreen => 2,
        Color::LightGrey => 3,
        Color::Orange => 4,
        Color::Purple => 5,
        Color::SkyBlue => 6,
        Color::Yellow => 7,
        Color::DarkRed => 8,
        Color::LightBlue => 9,
        Color::Blue => 10,
        Color::PastelRed => 11,
        Color::MayaBlue => 12,
        Color::White => 13,
    });
}
fn encode_event(event: &Event) -> Result<Vec<u8>, Error> {
    if event.decision_ordinal == 0 {
        return Err(Error::Malformed);
    }
    let mut out = Vec::with_capacity(MAX_EVENT_BYTES);
    out.extend(event.decision_ordinal.to_le_bytes());
    out.extend(event.emission_ordinal.to_le_bytes());
    match event.body {
        Body::Effect {
            position,
            id,
            source: s,
        } => {
            out.push(1);
            pos(&mut out, position);
            out.extend(id.to_le_bytes());
            out.push(source(s));
        }
        Body::Missile {
            from,
            to,
            id,
            source: s,
        } => {
            out.push(2);
            pos(&mut out, from);
            pos(&mut out, to);
            out.extend(id.to_le_bytes());
            out.push(source(s));
        }
        Body::Words {
            caster,
            position,
            spell,
        } => {
            if spell == 0 {
                return Err(Error::Malformed);
            }
            out.push(3);
            entity(&mut out, caster)?;
            pos(&mut out, position);
            out.extend(spell.to_le_bytes());
        }
        Body::Value {
            kind: k,
            position,
            target,
            attacker,
            primary,
            secondary,
        } => {
            out.push(4);
            out.push(kind(k));
            pos(&mut out, position);
            entity(&mut out, target)?;
            out.push(u8::from(attacker.is_some()));
            if let Some(a) = attacker {
                entity(&mut out, a)?
            }
            value(&mut out, primary);
            value(&mut out, secondary);
        }
        Body::Sound {
            position,
            id,
            source: s,
            secondary,
        } => {
            out.push(5);
            pos(&mut out, position);
            out.extend(id.to_le_bytes());
            out.push(source(s));
            out.push(u8::from(secondary.is_some()));
            if let Some(s) = secondary {
                out.extend(s.to_le_bytes());
            }
        }
    }
    if out.len() + 1 > MAX_EVENT_BYTES {
        return Err(Error::Limit);
    }
    Ok(out)
}
fn decode_event(bytes: &[u8]) -> Result<Event, Error> {
    if bytes.len() + 1 > MAX_EVENT_BYTES {
        return Err(Error::Limit);
    }
    let mut r = Read { bytes, at: 0 };
    let decision_ordinal = r.u64()?;
    let emission_ordinal = r.u16()?;
    if decision_ordinal == 0 {
        return Err(Error::Malformed);
    }
    let body = match r.u8()? {
        1 => Body::Effect {
            position: r.pos()?,
            id: r.u16()?,
            source: r.source()?,
        },
        2 => Body::Missile {
            from: r.pos()?,
            to: r.pos()?,
            id: r.u16()?,
            source: r.source()?,
        },
        3 => {
            let caster = r.entity()?;
            let position = r.pos()?;
            let spell = r.u32()?;
            if spell == 0 {
                return Err(Error::Malformed);
            }
            Body::Words {
                caster,
                position,
                spell,
            }
        }
        4 => {
            let kind = match r.u8()? {
                1 => ValueKind::DamageDealt,
                2 => ValueKind::DamageReceived,
                3 => ValueKind::DamageOthers,
                4 => ValueKind::Healed,
                5 => ValueKind::HealedOthers,
                6 => ValueKind::Mana,
                7 => ValueKind::Experience,
                8 => ValueKind::ExperienceOthers,
                _ => return Err(Error::Malformed),
            };
            let position = r.pos()?;
            let target = r.entity()?;
            let attacker = match r.u8()? {
                0 => None,
                1 => Some(r.entity()?),
                _ => return Err(Error::Malformed),
            };
            Body::Value {
                kind,
                position,
                target,
                attacker,
                primary: r.value()?,
                secondary: r.value()?,
            }
        }
        5 => {
            let position = r.pos()?;
            let id = r.u16()?;
            let source = r.source()?;
            let secondary = match r.u8()? {
                0 => None,
                1 => Some(r.u16()?),
                _ => return Err(Error::Malformed),
            };
            Body::Sound {
                position,
                id,
                source,
                secondary,
            }
        }
        _ => return Err(Error::Malformed),
    };
    r.end()?;
    Ok(Event {
        decision_ordinal,
        emission_ordinal,
        body,
    })
}
pub fn encode_batch(batch: &Batch) -> Result<Vec<u8>, Error> {
    if batch.events.len() > MAX_EVENTS {
        return Err(Error::Limit);
    }
    if batch.content == [0; 32] || batch.sync_unit == 0 {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    out.push(1);
    out.extend(batch.content);
    out.extend(batch.sync_unit.to_le_bytes());
    out.extend((batch.events.len() as u16).to_le_bytes());
    let mut seen = BTreeSet::new();
    for event in &batch.events {
        if !seen.insert((event.decision_ordinal, event.emission_ordinal)) {
            return Err(Error::Malformed);
        }
        let bytes = encode_event(event)?;
        out.push(bytes.len() as u8);
        out.extend(bytes);
    }
    if out.len() > MAX_BATCH_BYTES {
        return Err(Error::Limit);
    }
    Ok(out)
}
pub fn decode_batch(bytes: &[u8]) -> Result<Batch, Error> {
    if bytes.len() > MAX_BATCH_BYTES {
        return Err(Error::Limit);
    }
    let mut r = Read { bytes, at: 0 };
    if r.u8()? != 1 {
        return Err(Error::Malformed);
    }
    let content = r.bytes(32)?.try_into().map_err(|_| Error::Malformed)?;
    let sync_unit = r.u64()?;
    let count = usize::from(r.u16()?);
    if content == [0; 32] || sync_unit == 0 {
        return Err(Error::Malformed);
    }
    if count > MAX_EVENTS {
        return Err(Error::Limit);
    }
    let mut events = Vec::with_capacity(count);
    let mut seen = BTreeSet::new();
    for _ in 0..count {
        let len = usize::from(r.u8()?);
        let event = decode_event(r.bytes(len)?)?;
        if !seen.insert((event.decision_ordinal, event.emission_ordinal)) {
            return Err(Error::Malformed);
        }
        events.push(event);
    }
    r.end()?;
    Ok(Batch {
        content,
        sync_unit,
        events,
    })
}
pub fn decode_empty_snapshot(bytes: &[u8]) -> Result<(), Error> {
    if bytes.is_empty() {
        Ok(())
    } else {
        Err(Error::Malformed)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CooldownKind {
    Spell,
    Group,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cooldown {
    pub kind: CooldownKind,
    pub id: u32,
    pub expires_at_ms: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cooldowns {
    pub server_now_ms: u64,
    pub rtt_ms: u32,
    pub rtt_estimated: bool,
    pub entries: Vec<Cooldown>,
}
fn cooldowns_valid(value: &Cooldowns) -> Result<(), Error> {
    if value.entries.len() > MAX_COOLDOWNS || value.server_now_ms > MAX_CLOCK_MS {
        return Err(Error::Limit);
    }
    if !value.rtt_estimated && value.rtt_ms != 0 {
        return Err(Error::Malformed);
    }
    let mut seen = BTreeSet::new();
    for e in &value.entries {
        if e.expires_at_ms > MAX_CLOCK_MS {
            return Err(Error::Limit);
        }
        if e.id == 0 || (e.kind == CooldownKind::Group && e.id > 16) || !seen.insert((e.kind, e.id))
        {
            return Err(Error::Malformed);
        }
    }
    Ok(())
}
pub fn encode_cooldowns(value: &Cooldowns) -> Result<Vec<u8>, Error> {
    cooldowns_valid(value)?;
    let mut out = vec![1];
    out.extend(&value.server_now_ms.to_le_bytes()[..6]);
    out.extend(value.rtt_ms.to_le_bytes());
    out.push(u8::from(value.rtt_estimated));
    out.extend((value.entries.len() as u16).to_le_bytes());
    for e in &value.entries {
        out.push(if e.kind == CooldownKind::Spell { 1 } else { 2 });
        out.extend(e.id.to_le_bytes());
        out.extend(&e.expires_at_ms.to_le_bytes()[..6]);
    }
    Ok(out)
}
pub fn decode_cooldowns(bytes: &[u8]) -> Result<Cooldowns, Error> {
    if bytes.len() > 14 + MAX_COOLDOWNS * 16 {
        return Err(Error::Limit);
    }
    let mut r = Read { bytes, at: 0 };
    if r.u8()? != 1 {
        return Err(Error::Malformed);
    }
    let mut clock = [0; 8];
    clock[..6].copy_from_slice(r.bytes(6)?);
    let server_now_ms = u64::from_le_bytes(clock);
    let rtt_ms = r.u32()?;
    let rtt_estimated = match r.u8()? {
        0 => false,
        1 => true,
        _ => return Err(Error::Malformed),
    };
    let count = usize::from(r.u16()?);
    if count > MAX_COOLDOWNS {
        return Err(Error::Limit);
    }
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let kind = match r.u8()? {
            1 => CooldownKind::Spell,
            2 => CooldownKind::Group,
            _ => return Err(Error::Malformed),
        };
        let id = r.u32()?;
        clock = [0; 8];
        clock[..6].copy_from_slice(r.bytes(6)?);
        entries.push(Cooldown {
            kind,
            id,
            expires_at_ms: u64::from_le_bytes(clock),
        });
    }
    r.end()?;
    let value = Cooldowns {
        server_now_ms,
        rtt_ms,
        rtt_estimated,
        entries,
    };
    cooldowns_valid(&value)?;
    Ok(value)
}
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    #[test]
    fn bounded_effects_roundtrip_and_closed_framing() {
        let event = Event {
            decision_ordinal: 1,
            emission_ordinal: 0,
            body: Body::Effect {
                position: ActorPosition {
                    x: i32::MIN,
                    y: i32::MAX,
                    floor: i16::MIN,
                },
                id: 0,
                source: Source::Global,
            },
        };
        let b = Batch {
            content: [1; 32],
            sync_unit: 1,
            events: vec![event],
        };
        let bytes = encode_batch(&b).unwrap();
        assert_eq!(decode_batch(&bytes), Ok(b));
        let mut extra = bytes.clone();
        extra.push(0);
        assert_eq!(decode_batch(&extra), Err(Error::Malformed));
        let mut unknown = bytes;
        unknown[0] = 2;
        assert_eq!(decode_batch(&unknown), Err(Error::Malformed));
        assert!(decode_empty_snapshot(&[0]).is_err());
    }
    #[test]
    fn duplicate_events_and_cooldowns_refuse() {
        let e = Cooldown {
            kind: CooldownKind::Spell,
            id: 1,
            expires_at_ms: MAX_CLOCK_MS,
        };
        let c = Cooldowns {
            server_now_ms: MAX_CLOCK_MS,
            rtt_ms: 0,
            rtt_estimated: false,
            entries: vec![e],
        };
        let bytes = encode_cooldowns(&c).unwrap();
        assert_eq!(decode_cooldowns(&bytes), Ok(c.clone()));
        let mut repeated = c;
        repeated.entries.push(e);
        assert_eq!(encode_cooldowns(&repeated), Err(Error::Malformed));
        let mut invalid = bytes;
        invalid[11] = 2;
        assert_eq!(decode_cooldowns(&invalid), Err(Error::Malformed));
    }
    #[test]
    fn full_row_budget_and_clock_bounds() {
        let entries = (1..=MAX_COOLDOWNS as u32)
            .map(|id| Cooldown {
                kind: CooldownKind::Spell,
                id,
                expires_at_ms: 42,
            })
            .collect();
        let mut c = Cooldowns {
            server_now_ms: 1,
            rtt_ms: 8,
            rtt_estimated: true,
            entries,
        };
        assert!(encode_cooldowns(&c).is_ok());
        c.entries.push(Cooldown {
            kind: CooldownKind::Spell,
            id: 999,
            expires_at_ms: 0,
        });
        assert_eq!(encode_cooldowns(&c), Err(Error::Limit));
        c.entries.clear();
        c.server_now_ms = MAX_CLOCK_MS + 1;
        assert_eq!(encode_cooldowns(&c), Err(Error::Limit));
    }
}
