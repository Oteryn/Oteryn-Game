//! The spawn family of the World Bundle (format v3, format document §13; CREATURE-AI-0 §6.1).
//!
//! One payload holds every realizable spawn source of the World: a creature table, then the
//! sources ascending by key, each with its points in source order. All integers are LEB128
//! varints unless noted. The writer and the reader apply the same [`validate`], so the writer
//! never writes what the reader rejects.

use crate::Error;
use crate::bundle::Extent;
use crate::sector::{Reader, TileLimits, put, put_text};

/// `CREATUREAI0-RL-01`: spawn points per World map.
pub const MAX_POINTS: usize = 131_072;
/// `CREATUREAI0-RL-02`: spawn sources per World map.
pub const MAX_SOURCES: usize = 65_536;
/// `CREATUREAI0-RL-03`: points per spawn source.
pub const MAX_POINTS_PER_SOURCE: usize = 64;
/// `CREATUREAI0-RL-13`: respawn delay, in milliseconds.
pub const MIN_RESPAWN_MS: u32 = 1_000;
pub const MAX_RESPAWN_MS: u32 = 86_400_000;
/// Longest source or creature key, in bytes.
pub const MAX_KEY_BYTES: usize = 128;
/// Largest raw spawn payload: derived from the three counts above and [`MAX_KEY_BYTES`]
/// (65,536 keys of 128 bytes plus 131,072 points of at most 16 bytes is under 11 MiB).
pub const MAX_RAW_BYTES: usize = 16 << 20;

const KEY_LIMITS: TileLimits = TileLimits {
    max_entries: 0,
    max_text_bytes: MAX_KEY_BYTES,
};

/// The spawn period of a creature definition (`spawn_eligibility.period`). A point whose
/// creature has another period than `All` is compiled and is outside the activation set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Period {
    All,
    Night,
}

impl Period {
    fn byte(self) -> u8 {
        self as u8
    }

    fn from_byte(byte: u8) -> Result<Self, Error> {
        match byte {
            0 => Ok(Self::All),
            1 => Ok(Self::Night),
            other => Err(Error::Format(format!("spawn period {other} is unknown"))),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    fn byte(self) -> u8 {
        self as u8
    }

    fn from_byte(byte: u8) -> Result<Self, Error> {
        match byte {
            0 => Ok(Self::North),
            1 => Ok(Self::East),
            2 => Ok(Self::South),
            3 => Ok(Self::West),
            other => Err(Error::Format(format!("spawn direction {other} is unknown"))),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Creature {
    /// The creature definition key, e.g. `oteryn:creature.rat`.
    pub key: String,
    pub period: Period,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    /// Index into the creature table.
    pub creature: u32,
    pub x: u16,
    pub y: u16,
    pub direction: Direction,
    pub respawn_ms: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    pub key: String,
    /// Native floor of the centre and of every point.
    pub floor: i8,
    pub x: u16,
    pub y: u16,
    pub points: Vec<Point>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Table {
    /// Strictly ascending by key.
    pub creatures: Vec<Creature>,
    /// Strictly ascending by key; the points of a source keep their authored order.
    pub sources: Vec<Source>,
}

impl Table {
    pub fn point_count(&self) -> usize {
        self.sources.iter().map(|source| source.points.len()).sum()
    }
}

fn check(ok: bool, what: &str) -> Result<(), Error> {
    if ok {
        Ok(())
    } else {
        Err(Error::Format(what.into()))
    }
}

fn limit(ok: bool, what: &str) -> Result<(), Error> {
    if ok {
        Ok(())
    } else {
        Err(Error::Limit(what.into()))
    }
}

fn key_ok(key: &str) -> bool {
    !key.is_empty() && key.len() <= MAX_KEY_BYTES && key.bytes().all(|b| (0x21..=0x7E).contains(&b))
}

/// Checks a table against the World extent and the `CREATUREAI0-RL-*` rows.
pub fn validate(table: &Table, world: &Extent) -> Result<(), Error> {
    limit(
        table.sources.len() <= MAX_SOURCES,
        "spawn sources over RL-02",
    )?;
    limit(
        table.creatures.len() <= MAX_POINTS,
        "more spawn creatures than RL-01 points",
    )?;
    check(
        table.creatures.iter().all(|c| key_ok(&c.key))
            && table.creatures.windows(2).all(|p| p[0].key < p[1].key),
        "spawn creatures are not valid, ascending and unique",
    )?;
    check(
        table.sources.iter().all(|s| key_ok(&s.key))
            && table.sources.windows(2).all(|p| p[0].key < p[1].key),
        "spawn sources are not valid, ascending and unique",
    )?;
    let mut points = 0usize;
    for source in &table.sources {
        limit(
            source.points.len() <= MAX_POINTS_PER_SOURCE,
            "spawn points per source over RL-03",
        )?;
        check(!source.points.is_empty(), "spawn source without points")?;
        points += source.points.len();
        limit(points <= MAX_POINTS, "spawn points over RL-01")?;
        check(
            world.floors.binary_search(&source.floor).is_ok(),
            "spawn source floor outside the World",
        )?;
        for point in &source.points {
            check(
                (point.creature as usize) < table.creatures.len(),
                "spawn creature index out of range",
            )?;
            check(
                world.contains(point.x, point.y, source.floor),
                "spawn point outside the World",
            )?;
            limit(
                (MIN_RESPAWN_MS..=MAX_RESPAWN_MS).contains(&point.respawn_ms),
                "respawn delay outside RL-13",
            )?;
        }
    }
    Ok(())
}

fn zigzag(delta: i32) -> u64 {
    ((delta << 1) ^ (delta >> 31)) as u32 as u64
}

fn unzigzag(value: u64) -> i64 {
    (value >> 1) as i64 ^ -((value & 1) as i64)
}

/// Encodes a table. The caller has validated it.
pub fn encode(table: &Table) -> Vec<u8> {
    let mut out = Vec::new();
    put(&mut out, table.creatures.len() as u64);
    for creature in &table.creatures {
        put_text(&mut out, &creature.key);
        out.push(creature.period.byte());
    }
    put(&mut out, table.sources.len() as u64);
    for source in &table.sources {
        put_text(&mut out, &source.key);
        out.push(source.floor as u8);
        put(&mut out, source.x.into());
        put(&mut out, source.y.into());
        put(&mut out, source.points.len() as u64);
        for point in &source.points {
            put(&mut out, point.creature.into());
            put(&mut out, zigzag(i32::from(point.x) - i32::from(source.x)));
            put(&mut out, zigzag(i32::from(point.y) - i32::from(source.y)));
            out.push(point.direction.byte());
            put(&mut out, point.respawn_ms.into());
        }
    }
    out
}

/// Decodes and validates a payload. Every count is checked against the bytes left before
/// anything is reserved for it, since each element needs at least one byte.
pub fn decode(raw: &[u8], world: &Extent) -> Result<Table, Error> {
    let mut r = Reader { buf: raw, pos: 0 };
    let left = |r: &Reader<'_>| r.buf.len() - r.pos;
    let count = |r: &mut Reader<'_>, max: usize, what: &str| -> Result<usize, Error> {
        let n = r.bounded(max as u64, what)? as usize;
        check(n <= left(r), "spawn payload is truncated")?;
        Ok(n)
    };
    let creature_count = count(&mut r, MAX_POINTS, "spawn creature count")?;
    let mut creatures = Vec::with_capacity(creature_count);
    for _ in 0..creature_count {
        let key = r.text(KEY_LIMITS)?;
        creatures.push(Creature {
            key,
            period: Period::from_byte(r.byte()?)?,
        });
    }
    let source_count = count(&mut r, MAX_SOURCES, "spawn source count")?;
    let mut sources = Vec::with_capacity(source_count);
    let mut points = 0usize;
    for _ in 0..source_count {
        let key = r.text(KEY_LIMITS)?;
        let floor = r.byte()? as i8;
        let (x, y) = (r.u16("spawn centre x")?, r.u16("spawn centre y")?);
        let n = count(&mut r, MAX_POINTS_PER_SOURCE, "spawn points per source")?;
        points += n;
        limit(points <= MAX_POINTS, "spawn points over RL-01")?;
        let mut list = Vec::with_capacity(n);
        for _ in 0..n {
            let creature = r.bounded(u64::from(u32::MAX), "spawn creature index")? as u32;
            let at = |centre: u16, delta: u64| -> Result<u16, Error> {
                u16::try_from(i64::from(centre) + unzigzag(delta))
                    .map_err(|_| Error::Format("spawn point outside the u16 plane".into()))
            };
            let px = at(x, r.varint()?)?;
            let py = at(y, r.varint()?)?;
            let direction = Direction::from_byte(r.byte()?)?;
            let respawn_ms = r.bounded(u64::from(u32::MAX), "respawn delay")? as u32;
            list.push(Point {
                creature,
                x: px,
                y: py,
                direction,
                respawn_ms,
            });
        }
        sources.push(Source {
            key,
            floor,
            x,
            y,
            points: list,
        });
    }
    check(r.pos == raw.len(), "bytes after the spawn payload")?;
    let table = Table { creatures, sources };
    validate(&table, world)?;
    Ok(table)
}
