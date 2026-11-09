//! Named spell modifiers from one fenced combat observation, never cast authority.
use crate::charm_wire::CyclopediaWireError as Error;
pub const SELECTOR: u8 = 131;
pub const MAX_ROWS: usize = 1024;
pub const MAX_SOURCES: usize = 32;
pub const MAX_SNAPSHOT_BYTES: usize = 262_144;
pub const PAGE_SIZE: usize = 64;
#[path = "offence/page.rs"]
mod page;
pub use page::{Page, Query};
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum SourceKind {
    Equipment = 1,
    Proficiency = 2,
    Wheel = 3,
}
impl SourceKind {
    pub fn bit(self) -> u8 {
        1 << (self as u8 - 1)
    }
    fn decode(v: u8) -> Result<Self, Error> {
        match v {
            1 => Ok(Self::Equipment),
            2 => Ok(Self::Proficiency),
            3 => Ok(Self::Wheel),
            _ => Err(Error::Malformed),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Effect {
    BaseDamage = 1,
    BaseHealing = 2,
    CriticalChance = 3,
    CriticalDamage = 4,
    LifeLeech = 5,
    ManaLeech = 6,
    Cooldown = 7,
    WheelStage = 8,
    GroupCooldown = 9,
}
impl Effect {
    fn decode(v: u8) -> Result<Self, Error> {
        match v {
            1 => Ok(Self::BaseDamage),
            2 => Ok(Self::BaseHealing),
            3 => Ok(Self::CriticalChance),
            4 => Ok(Self::CriticalDamage),
            5 => Ok(Self::LifeLeech),
            6 => Ok(Self::ManaLeech),
            7 => Ok(Self::Cooldown),
            8 => Ok(Self::WheelStage),
            9 => Ok(Self::GroupCooldown),
            _ => Err(Error::Malformed),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub kind: SourceKind,
    pub slot: u8,
    pub key: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Augment {
    pub source: u8,
    pub spell: String,
    pub effect: Effect,
    pub value: i64,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub known_sources: u8,
    pub sources: Vec<Source>,
    pub rows: Vec<Augment>,
}
impl Snapshot {
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        if self.known_sources > 7 || self.sources.len() > MAX_SOURCES || self.rows.len() > MAX_ROWS
        {
            return Err(Error::LimitExceeded);
        }
        let mut out = vec![1, self.known_sources, self.sources.len() as u8];
        for (i, source) in self.sources.iter().enumerate() {
            if !text_ok(&source.key, 512)
                || self.sources[..i].contains(source)
                || self.known_sources & source.kind.bit() == 0
                || match source.kind {
                    SourceKind::Equipment => !(1..=32).contains(&source.slot),
                    SourceKind::Proficiency => source.slot != 5,
                    SourceKind::Wheel => source.slot != 0,
                }
            {
                return Err(Error::Malformed);
            }
            out.extend_from_slice(&[source.kind as u8, source.slot]);
            text(&mut out, &source.key)?;
        }
        out.extend_from_slice(&(self.rows.len() as u16).to_le_bytes());
        let mut seen = std::collections::BTreeSet::new();
        for row in &self.rows {
            if usize::from(row.source) >= self.sources.len()
                || !text_ok(&row.spell, 128)
                || row.value < 0
                || (row.effect == Effect::WheelStage && !(1..=3).contains(&row.value))
                || !seen.insert((row.source, &row.spell, row.effect))
            {
                return Err(Error::Malformed);
            }
            out.extend_from_slice(&[row.source, row.effect as u8]);
            text(&mut out, &row.spell)?;
            out.extend_from_slice(&row.value.to_le_bytes());
        }
        if out.len() > MAX_SNAPSHOT_BYTES {
            return Err(Error::LimitExceeded);
        }
        Ok(out)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_SNAPSHOT_BYTES {
            return Err(Error::LimitExceeded);
        }
        let mut r = Reader(bytes);
        if r.byte()? != 1 {
            return Err(Error::Malformed);
        }
        let known_sources = r.byte()?;
        let count = usize::from(r.byte()?);
        if count > MAX_SOURCES {
            return Err(Error::LimitExceeded);
        }
        let mut sources = Vec::new();
        for _ in 0..count {
            sources.push(Source {
                kind: SourceKind::decode(r.byte()?)?,
                slot: r.byte()?,
                key: r.text()?,
            });
        }
        let count = usize::from(r.u16()?);
        if count > MAX_ROWS {
            return Err(Error::LimitExceeded);
        }
        let mut rows = Vec::new();
        for _ in 0..count {
            rows.push(Augment {
                source: r.byte()?,
                effect: Effect::decode(r.byte()?)?,
                spell: r.text()?,
                value: i64::from_le_bytes(r.take(8)?.try_into().map_err(|_| Error::Malformed)?),
            });
        }
        let value = Self {
            known_sources,
            sources,
            rows,
        };
        if !r.0.is_empty() || value.encode()?.as_slice() != bytes {
            return Err(Error::Malformed);
        }
        Ok(value)
    }
}
fn text_ok(value: &str, max: usize) -> bool {
    !value.is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}
fn text(out: &mut Vec<u8>, value: &str) -> Result<(), Error> {
    out.extend_from_slice(
        &u16::try_from(value.len())
            .map_err(|_| Error::LimitExceeded)?
            .to_le_bytes(),
    );
    out.extend_from_slice(value.as_bytes());
    Ok(())
}
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let value = self.0.get(..n).ok_or(Error::Malformed)?;
        self.0 = &self.0[n..];
        Ok(value)
    }
    fn byte(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().map_err(|_| Error::Malformed)?,
        ))
    }
    fn text(&mut self) -> Result<String, Error> {
        let n = usize::from(self.u16()?);
        Ok(std::str::from_utf8(self.take(n)?)
            .map_err(|_| Error::Malformed)?
            .into())
    }
}
#[cfg(test)]
#[path = "offence/tests.rs"]
mod tests;
