//! The NPC family of the World Bundle (format v4, format document §14; decision NPC-PLACE-1
//! §3).
//!
//! One payload holds every written NPC placement of the World: the NPCs ascending by key, each
//! with its placements ascending by `(floor, y, x)`. All integers are LEB128 varints unless
//! noted. The writer and the reader apply the same [`validate`], so the writer never writes what
//! the reader rejects. The frame order is the canonical actor order (NPC-BEHAVIOUR-0 §3.1).

use std::collections::BTreeSet;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::Error;
use crate::bundle::Extent;
use crate::sector::{Reader, TileLimits, put, put_text};
pub use crate::spawn::Direction;

/// `NPCPLACE1-RL-01` (= `NPCBEH0-RL-01`): placements per bundle. It bounds the NPC count too,
/// since every NPC has at least one placement.
pub const MAX_PLACEMENTS: usize = 2_048;
/// `NPCPLACE1-RL-02`: placements per NPC.
pub const MAX_PLACEMENTS_PER_NPC: usize = 16;
/// `NPCPLACE1-RL-03`: raw NPC payload bytes; the frame ratio is the bundle's
/// [`crate::bundle::MAX_SECTOR_RATIO`].
pub const MAX_RAW_BYTES: usize = 1 << 20;
/// Longest NPC key, in bytes.
pub const MAX_KEY_BYTES: usize = 128;
/// Every NPC key starts with this.
pub const KEY_PREFIX: &str = "oteryn:npc.";
/// The content directories `catalogue_sha256` covers, relative to `content/` (decision
/// NPC-PLACE-1 §3.1).
pub const CATALOGUE_DIRS: [&str; 3] = ["npcs/definitions", "dialogues/definitions", "services"];

const KEY_LIMITS: TileLimits = TileLimits {
    max_entries: 0,
    max_text_bytes: MAX_KEY_BYTES,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    /// Native floor, one of the manifest's `world.floors`.
    pub floor: i8,
    pub x: u16,
    pub y: u16,
    pub direction: Direction,
}

impl Placement {
    /// The frame order of the placements of one NPC.
    pub fn order(&self) -> (i8, u16, u16) {
        (self.floor, self.y, self.x)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Npc {
    /// The NPC definition key, e.g. `oteryn:npc.cipfried`.
    pub key: String,
    /// Strictly ascending by [`Placement::order`]; at least one.
    pub placements: Vec<Placement>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Table {
    /// Strictly ascending by key.
    pub npcs: Vec<Npc>,
}

impl Table {
    pub fn placement_count(&self) -> usize {
        self.npcs.iter().map(|npc| npc.placements.len()).sum()
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

/// Whether `key` is a valid NPC key: printable ASCII, at most 128 bytes, `oteryn:npc.` and a
/// non-empty name.
pub fn key_ok(key: &str) -> bool {
    key.len() > KEY_PREFIX.len()
        && key.len() <= MAX_KEY_BYTES
        && key.starts_with(KEY_PREFIX)
        && key.bytes().all(|b| (0x21..=0x7E).contains(&b))
}

/// Checks a table against the World extent and the `NPCPLACE1-RL-*` rows.
pub fn validate(table: &Table, world: &Extent) -> Result<(), Error> {
    limit(
        table.npcs.len() <= MAX_PLACEMENTS,
        "NPCs over NPCPLACE1-RL-01",
    )?;
    check(
        table.npcs.iter().all(|npc| key_ok(&npc.key))
            && table.npcs.windows(2).all(|p| p[0].key < p[1].key),
        "NPC keys are not valid, ascending and unique",
    )?;
    let mut placements = 0usize;
    let mut cells = BTreeSet::new();
    for npc in &table.npcs {
        limit(
            npc.placements.len() <= MAX_PLACEMENTS_PER_NPC,
            "placements per NPC over NPCPLACE1-RL-02",
        )?;
        check(!npc.placements.is_empty(), "NPC without placements")?;
        placements += npc.placements.len();
        limit(
            placements <= MAX_PLACEMENTS,
            "NPC placements over NPCPLACE1-RL-01",
        )?;
        check(
            npc.placements
                .windows(2)
                .all(|p| p[0].order() < p[1].order()),
            "NPC placements are not ascending by (floor, y, x)",
        )?;
        for placement in &npc.placements {
            check(
                world.contains(placement.x, placement.y, placement.floor),
                "NPC placement outside the World",
            )?;
            check(
                cells.insert(placement.order()),
                "two NPC placements share a cell",
            )?;
        }
    }
    Ok(())
}

/// Encodes a table. The caller has validated it.
pub fn encode(table: &Table) -> Vec<u8> {
    let mut out = Vec::new();
    put(&mut out, table.npcs.len() as u64);
    for npc in &table.npcs {
        put_text(&mut out, &npc.key);
        put(&mut out, npc.placements.len() as u64);
        for placement in &npc.placements {
            out.push(placement.floor as u8);
            put(&mut out, placement.y.into());
            put(&mut out, placement.x.into());
            out.push(placement.direction as u8);
        }
    }
    out
}

fn direction(byte: u8) -> Result<Direction, Error> {
    match byte {
        0 => Ok(Direction::North),
        1 => Ok(Direction::East),
        2 => Ok(Direction::South),
        3 => Ok(Direction::West),
        other => Err(Error::Format(format!("NPC direction {other} is unknown"))),
    }
}

/// Decodes and validates a payload. Every count is checked against its limit and against the
/// bytes left before anything is reserved for it, since each element needs at least one byte.
pub fn decode(raw: &[u8], world: &Extent) -> Result<Table, Error> {
    let mut r = Reader { buf: raw, pos: 0 };
    let count = |r: &mut Reader<'_>, max: usize, what: &str| -> Result<usize, Error> {
        let n = r.varint()?;
        limit(n <= max as u64, what)?;
        check(
            n as usize <= r.buf.len() - r.pos,
            "NPC payload is truncated",
        )?;
        Ok(n as usize)
    };
    let npc_count = count(&mut r, MAX_PLACEMENTS, "NPCs over NPCPLACE1-RL-01")?;
    let mut npcs = Vec::with_capacity(npc_count);
    let mut placements = 0usize;
    for _ in 0..npc_count {
        let key = r.text(KEY_LIMITS)?;
        let n = count(
            &mut r,
            MAX_PLACEMENTS_PER_NPC,
            "placements per NPC over NPCPLACE1-RL-02",
        )?;
        placements += n;
        limit(
            placements <= MAX_PLACEMENTS,
            "NPC placements over NPCPLACE1-RL-01",
        )?;
        let mut list = Vec::with_capacity(n);
        for _ in 0..n {
            let floor = r.byte()? as i8;
            let y = r.u16("NPC placement y")?;
            let x = r.u16("NPC placement x")?;
            list.push(Placement {
                floor,
                x,
                y,
                direction: direction(r.byte()?)?,
            });
        }
        npcs.push(Npc {
            key,
            placements: list,
        });
    }
    check(r.pos == raw.len(), "bytes after the NPC payload")?;
    let table = Table { npcs };
    validate(&table, world)?;
    Ok(table)
}

/// Lowercase hex of `bytes`.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Whether `value` is 64 lowercase hex digits.
pub fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// `catalogue_sha256` over `files`, each a path relative to `content/` (`/`-separated) and its
/// bytes (decision NPC-PLACE-1 §3.1): in byte order of the path, each as the path length (u64
/// big-endian), the path, the byte length (u64 big-endian) and the bytes. The caller passes
/// exactly the files under [`CATALOGUE_DIRS`].
pub fn catalogue_digest<P: AsRef<str>, B: AsRef<[u8]>>(files: &[(P, B)]) -> String {
    let mut sorted: Vec<_> = files
        .iter()
        .map(|(path, bytes)| (path.as_ref().as_bytes(), bytes.as_ref()))
        .collect();
    sorted.sort_unstable_by(|a, b| a.0.cmp(b.0));
    let mut hash = Sha256::new();
    for (path, bytes) in sorted {
        hash.update((path.len() as u64).to_be_bytes());
        hash.update(path);
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
    }
    hex(&hash.finalize())
}

/// `catalogue_sha256` of the content tree at `content`: every regular file under
/// [`CATALOGUE_DIRS`] (a missing directory contributes nothing), hashed by
/// [`catalogue_digest`]. The compiler and the game server both call this.
pub fn catalogue_sha256(content: &Path) -> std::io::Result<String> {
    let mut files = Vec::new();
    for dir in CATALOGUE_DIRS {
        collect(content, &content.join(dir), &mut files)?;
    }
    Ok(catalogue_digest(&files))
}

fn collect(content: &Path, dir: &Path, files: &mut Vec<(String, Vec<u8>)>) -> std::io::Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    for entry in entries {
        let path = entry?.path();
        let kind = std::fs::metadata(&path)?;
        if kind.is_dir() {
            collect(content, &path, files)?;
        } else if kind.is_file() {
            let relative = path
                .strip_prefix(content)
                .map_err(|_| std::io::Error::other("catalogue file outside the content tree"))?;
            let name = relative
                .components()
                .map(|part| {
                    part.as_os_str()
                        .to_str()
                        .ok_or_else(|| std::io::Error::other("catalogue path is not UTF-8"))
                })
                .collect::<Result<Vec<_>, _>>()?
                .join("/");
            files.push((name, std::fs::read(&path)?));
        }
    }
    Ok(())
}
