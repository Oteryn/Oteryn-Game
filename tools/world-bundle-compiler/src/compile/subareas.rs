//! Admission of the existing 15.30 masks. Historical source candidates stay unmodified.
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::{BufReader, Cursor, Write};
use std::path::Path;

use oteryn_world_bundle::subarea::{self, Area, Mask, Run, Table};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{Input, KeyResolver, cell_facts, native_floor};
use crate::{Error, project::LegacyPosition};

const MAX_INPUT: usize = 16 << 20;
const MAX_ASSET: usize = 64 << 10;
const MAX_PIXELS: usize = 1 << 20;
const MAX_BMP: usize = MAX_PIXELS * 4 + 122;

fn fail(what: impl std::fmt::Display) -> Error {
    Error::Format(format!("subarea admission: {what}"))
}

fn read(path: &Path, limit: usize) -> Result<Vec<u8>, Error> {
    let file = std::fs::File::open(path).map_err(fail)?;
    if file.metadata().map_err(fail)?.len() > limit as u64 {
        return Err(Error::Limit("subarea input bytes".into()));
    }
    use std::io::Read;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(fail)?;
    if bytes.len() > limit {
        return Err(Error::Limit("subarea input bytes".into()));
    }
    Ok(bytes)
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[derive(Clone, Debug, Default)]
pub struct Source {
    table: Table,
    tiles: BTreeMap<LegacyPosition, u32>,
}

#[derive(Deserialize)]
struct Candidate {
    source_id: u32,
    raw_parent_source_ids: Vec<u32>,
    native_membership_admitted: bool,
    raw_mask_record_sha256: String,
    raw_mask_locator: Locator,
    compressed_asset: Asset,
    bmp: Bmp,
    membership: Membership,
}
#[derive(Deserialize)]
struct Locator {
    repository_path: String,
    record_index: usize,
}
#[derive(Deserialize)]
struct Asset {
    repository_path: String,
    sha256: String,
    area_source_id: u32,
}
#[derive(Deserialize)]
struct Bmp {
    width: usize,
    height: usize,
    sha256: String,
}
#[derive(Deserialize)]
struct Membership {
    source_anchor: Position,
    count: usize,
    runs_row_begin_end_exclusive: Vec<[usize; 3]>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
struct Position {
    x: u16,
    y: u16,
    z: u8,
}

impl Source {
    /// Decode and verify the source assets before native World admission in `compile`.
    pub fn load(root: &Path) -> Result<Self, Error> {
        let mut source = Self::default();
        let mut parents = BTreeMap::new();
        for directory in ["regions", "cities"] {
            let directory = root.join("content/world/areas").join(directory);
            let mut paths = std::fs::read_dir(directory)
                .map_err(fail)?
                .take(33)
                .map(|entry| entry.map(|e| e.path()).map_err(fail))
                .collect::<Result<Vec<_>, _>>()?;
            if paths.len() > 32 {
                return Err(Error::Limit("subarea area shards".into()));
            }
            paths.sort();
            for path in paths {
                if !path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("areas-") && n.ends_with(".json"))
                {
                    continue;
                }
                let value: Value =
                    serde_json::from_slice(&read(&path, MAX_INPUT)?).map_err(fail)?;
                for a in value["areas"]
                    .as_array()
                    .ok_or_else(|| fail("area shard"))?
                {
                    let text = |v: &Value| {
                        v.as_str()
                            .map(str::to_owned)
                            .ok_or_else(|| fail("area string"))
                    };
                    let provenance = &a["provenance"];
                    if provenance["source"] != "cipsoft/map_area"
                        || provenance["client_version"] != "15.30"
                    {
                        return Err(fail("area provenance"));
                    }
                    let id = provenance["source_id"]
                        .as_u64()
                        .and_then(|id| u32::try_from(id).ok())
                        .ok_or_else(|| fail("area id"))?;
                    let parent = if a["parent"].is_null() {
                        None
                    } else {
                        Some(text(&a["parent"]["key"])?)
                    };
                    if parents.insert(id, parent).is_some() {
                        return Err(fail("duplicate area id"));
                    }
                    source.table.areas.push(Area {
                        id,
                        key: text(&a["identity"]["key"])?,
                        name: text(&a["name"])?,
                        parent: None,
                        source_record_sha256: text(&provenance["source_record_sha256"])?,
                    });
                    if source.table.areas.len() > subarea::MAX_AREAS {
                        return Err(Error::Limit("subarea areas".into()));
                    }
                }
            }
        }
        let ids: BTreeMap<_, _> = source
            .table
            .areas
            .iter()
            .map(|a| (a.key.clone(), a.id))
            .collect();
        for a in &mut source.table.areas {
            a.parent = parents[&a.id]
                .as_ref()
                .map(|p| ids.get(p).copied().ok_or_else(|| fail("unknown parent")))
                .transpose()?;
        }
        source.table.areas.sort_by_key(|a| a.id);
        let bytes = read(
            &root.join("content/cyclopedia/source-membership-candidate.jsonl"),
            MAX_INPUT,
        )?;
        let provenance: Value = serde_json::from_slice(&read(
            &root.join("content/cyclopedia/source-membership-candidate.provenance.json"),
            MAX_INPUT,
        )?)
        .map_err(fail)?;
        source.table.candidate_sha256 = digest(&bytes);
        if provenance["candidate_sha256"] != source.table.candidate_sha256 {
            return Err(fail("candidate digest"));
        }
        source.table.client_version = "15.30".into();
        for line in bytes.split(|b| *b == b'\n').filter(|line| !line.is_empty()) {
            if source.table.masks.len() >= subarea::MAX_MASKS {
                return Err(Error::Limit("subarea masks".into()));
            }
            let c: Candidate = serde_json::from_slice(line).map_err(fail)?;
            let hash = |s: &str| {
                s.len() == 64
                    && s.bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            };
            if !hash(&c.bmp.sha256)
                || !hash(&c.compressed_asset.sha256)
                || !hash(&c.raw_mask_record_sha256)
            {
                return Err(fail("malformed source digest"));
            }
            let area = source
                .table
                .areas
                .iter()
                .find(|a| a.id == c.source_id)
                .ok_or_else(|| fail("mask area"))?;
            if c.native_membership_admitted
                || area.parent.is_none()
                || c.raw_parent_source_ids != area.parent.into_iter().collect::<Vec<_>>()
            {
                return Err(fail(
                    "source candidate is not an unadmitted single-parent subarea",
                ));
            }
            let expected = format!(
                "content/assets/files/subarea-{:04}-{}.bmp.lzma",
                c.source_id, c.bmp.sha256
            );
            if c.compressed_asset.repository_path != expected
                || c.compressed_asset.area_source_id != c.source_id
            {
                return Err(fail("asset locator"));
            }
            let raw_path = Path::new(&c.raw_mask_locator.repository_path);
            if raw_path.parent() != Some(Path::new("imports/cipsoft-staticdata/map/subareas"))
                || !raw_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("subareas-") && n.ends_with(".json"))
            {
                return Err(fail("raw locator"));
            }
            let raw: Value =
                serde_json::from_slice(&read(&root.join(raw_path), MAX_INPUT)?).map_err(fail)?;
            let r = &raw["records"][c.raw_mask_locator.record_index];
            if r["area_id"] != c.source_id
                || r["source_record_sha256"] != c.raw_mask_record_sha256
                || r["image"]["file_sha256"] != c.compressed_asset.sha256
                || r["image"]["name_hash"] != c.bmp.sha256
                || r["field_4"] != c.bmp.width
                || r["field_5"] != c.bmp.height
                || serde_json::from_value::<Position>(r["position"].clone()).map_err(fail)?
                    != c.membership.source_anchor
                || r["layer_kind"] != 0
            {
                return Err(fail("raw mask provenance"));
            }
            let encoded = read(&root.join(expected), MAX_ASSET)?;
            if digest(&encoded) != c.compressed_asset.sha256 {
                return Err(fail("compressed digest"));
            }
            let bmp = decode(&encoded)?;
            if digest(&bmp) != c.bmp.sha256 {
                return Err(fail("BMP digest"));
            }
            let runs = pixels(&bmp, c.bmp.width, c.bmp.height)?;
            if runs != c.membership.runs_row_begin_end_exclusive {
                return Err(fail("source runs differ from decoded pixels"));
            }
            let anchor = c.membership.source_anchor;
            native_floor(anchor.z)?;
            let mut count = 0;
            for [row, begin, end] in runs {
                for column in begin..end {
                    let x = usize::from(anchor.x)
                        .checked_add(column)
                        .and_then(|n| u16::try_from(n).ok())
                        .ok_or_else(|| fail("source x overflow"))?;
                    let y = usize::from(anchor.y)
                        .checked_add(row)
                        .and_then(|n| u16::try_from(n).ok())
                        .ok_or_else(|| fail("source y overflow"))?;
                    if source.tiles.insert((x, y, anchor.z), c.source_id).is_some() {
                        return Err(fail("overlapping source masks"));
                    }
                    count += 1;
                    if source.tiles.len() > subarea::MAX_TILES {
                        return Err(Error::Limit("subarea source tiles".into()));
                    }
                }
            }
            if count != c.membership.count || count == 0 {
                return Err(fail("source count"));
            }
            source.table.masks.push(Mask {
                subarea: c.source_id,
                asset: c.compressed_asset.repository_path,
                compressed_sha256: c.compressed_asset.sha256,
                bmp_sha256: c.bmp.sha256,
                source_record_sha256: c.raw_mask_record_sha256,
                source_tiles: count as u32,
                admitted_tiles: 0,
            });
        }
        source.table.masks.sort_by_key(|m| m.subarea);
        if source
            .table
            .masks
            .windows(2)
            .any(|p| p[0].subarea == p[1].subarea)
        {
            return Err(fail("duplicate mask"));
        }
        Ok(source)
    }
}

struct Bounded(Vec<u8>);
impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > MAX_BMP.saturating_sub(self.0.len()) {
            return Err(std::io::Error::other("subarea BMP cap"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn decode(encoded: &[u8]) -> Result<Vec<u8>, Error> {
    let mut at = encoded
        .iter()
        .position(|b| *b != 0)
        .ok_or_else(|| fail("framing"))?;
    if encoded.get(at..at + 2) != Some(&[0x70, 0x0a]) {
        return Err(fail("framing marker"));
    }
    at += 2;
    for _ in 0..2 {
        for n in 0..5 {
            let b = *encoded.get(at).ok_or_else(|| fail("truncated framing"))?;
            at += 1;
            if b & 0x80 == 0 {
                break;
            }
            if n == 4 {
                return Err(fail("framing varint"));
            }
        }
    }
    let payload = encoded
        .get(at..)
        .filter(|p| p.len() >= 13)
        .ok_or_else(|| fail("LZMA header"))?;
    let mut input = BufReader::new(Cursor::new(payload));
    let mut output = Bounded(Vec::new());
    lzma_rs::lzma_decompress_with_options(
        &mut input,
        &mut output,
        &lzma_rs::decompress::Options {
            unpacked_size: lzma_rs::decompress::UnpackedSize::ReadHeaderButUseProvided(None),
            memlimit: Some(32 << 20),
            allow_incomplete: false,
        },
    )
    .map_err(fail)?;
    Ok(output.0)
}

fn pixels(bmp: &[u8], width: usize, height: usize) -> Result<Vec<[usize; 3]>, Error> {
    if width == 0
        || height == 0
        || width.checked_mul(height).is_none_or(|n| n > MAX_PIXELS)
        || bmp.len() < 54
        || &bmp[..2] != b"BM"
    {
        return Err(fail("BMP size"));
    }
    let u32_at = |at| u32::from_le_bytes(bmp[at..at + 4].try_into().unwrap_or_default());
    let signed_height = u32_at(22) as i32;
    let offset = u32_at(10) as usize;
    if u32_at(2) as usize != bmp.len()
        || u32_at(18) as usize != width
        || signed_height.unsigned_abs() as usize != height
        || bmp[26..30] != [1, 0, 32, 0]
        || ![0, 3].contains(&u32_at(30))
        || offset < 54
        || offset.checked_add(width * height * 4) != Some(bmp.len())
    {
        return Err(fail("BMP shape"));
    }
    let mut runs = Vec::new();
    for row in 0..height {
        let physical = if signed_height < 0 {
            row
        } else {
            height - row - 1
        };
        let start = offset + physical * width * 4;
        let mut column = 0;
        while column < width {
            if bmp[start + column * 4..start + column * 4 + 4] == [0; 4] {
                column += 1;
                continue;
            }
            let begin = column;
            while column < width && bmp[start + column * 4..start + column * 4 + 4] != [0; 4] {
                column += 1;
            }
            runs.push([row, begin, column]);
        }
    }
    Ok(runs)
}

fn neighbours((x, y, z): LegacyPosition) -> impl Iterator<Item = LegacyPosition> {
    [
        x.checked_sub(1).map(|x| (x, y, z)),
        x.checked_add(1).map(|x| (x, y, z)),
        y.checked_sub(1).map(|y| (x, y, z)),
        y.checked_add(1).map(|y| (x, y, z)),
    ]
    .into_iter()
    .flatten()
}

pub(super) fn realize(
    input: &Input<'_>,
    resolver: &dyn KeyResolver,
    report: &super::NpcReport,
) -> Result<Table, Error> {
    let source = &input.families.subareas;
    if source.tiles.is_empty() {
        return Ok(source.table.clone());
    }
    let mut wanted: BTreeSet<_> = source.tiles.keys().copied().collect();
    for &p in source.tiles.keys() {
        wanted.extend(neighbours(p));
    }
    let facts = cell_facts(input, resolver, &wanted)?;
    let walkable = |p: &LegacyPosition| {
        input.world.contains(p.0, p.1, -(p.2 as i8))
            && facts.get(p).is_some_and(|c| {
                c.tile && c.ground == Some(true) && !c.wall && !c.block_solid && !c.unclassified
            })
    };
    let mut entries = BTreeSet::new();
    entries.extend(input.families.teleports.values().copied());
    entries.extend(
        input
            .families
            .routes
            .iter()
            .filter(|r| {
                !report
                    .routes_held
                    .iter()
                    .any(|h| h.service == r.service && h.route == r.route)
            })
            .map(|r| r.destination),
    );
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::new();
    for (&p, &id) in &source.tiles {
        if walkable(&p)
            && (entries.contains(&p)
                || neighbours(p).any(|n| walkable(&n) && source.tiles.get(&n) != Some(&id)))
        {
            seen.insert(p);
            queue.push_back(p);
        }
    }
    while let Some(p) = queue.pop_front() {
        for n in neighbours(p) {
            if source.tiles.get(&n) == source.tiles.get(&p) && walkable(&n) && seen.insert(n) {
                queue.push_back(n);
            }
        }
    }
    let mut table = source.table.clone();
    let mut positions: Vec<_> = seen
        .into_iter()
        .map(|(x, y, z)| (-(z as i8), y, x))
        .collect();
    positions.sort_unstable();
    let mut counts = BTreeMap::<u32, u32>::new();
    for (floor, y, x) in positions {
        let id = source.tiles[&(x, y, (-floor) as u8)];
        *counts.entry(id).or_default() += 1;
        let end = x.checked_add(1).ok_or_else(|| fail("native x overflow"))?;
        if let Some(last) = table
            .runs
            .last_mut()
            .filter(|r| r.floor == floor && r.y == y && r.end == x && r.subarea == id)
        {
            last.end = end;
        } else {
            if table.runs.len() >= subarea::MAX_RUNS {
                return Err(Error::Limit("subarea native runs".into()));
            }
            table.runs.push(Run {
                floor,
                y,
                begin: x,
                end,
                subarea: id,
            });
        }
    }
    for m in &mut table.masks {
        m.admitted_tiles = counts.get(&m.subarea).copied().unwrap_or(0);
    }
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subarea_bmp_orientation_shape_framing_and_allocation_bounds() -> Result<(), Error> {
        let mut bmp = vec![0; 54 + 16];
        bmp[..2].copy_from_slice(b"BM");
        bmp[2..6].copy_from_slice(&70u32.to_le_bytes());
        bmp[10..14].copy_from_slice(&54u32.to_le_bytes());
        bmp[18..22].copy_from_slice(&2u32.to_le_bytes());
        bmp[22..26].copy_from_slice(&2i32.to_le_bytes());
        bmp[26..30].copy_from_slice(&[1, 0, 32, 0]);
        bmp[54..58].copy_from_slice(&[255; 4]);
        assert_eq!(pixels(&bmp, 2, 2)?, vec![[1, 0, 1]]);
        bmp[22..26].copy_from_slice(&(-2i32).to_le_bytes());
        assert_eq!(pixels(&bmp, 2, 2)?, vec![[0, 0, 1]]);
        assert!(pixels(&bmp, MAX_PIXELS + 1, 2).is_err());
        assert!(pixels(&bmp, 3, 2).is_err());
        bmp[10..14].copy_from_slice(&0u32.to_le_bytes());
        assert!(pixels(&bmp, 2, 2).is_err());
        for invalid in [&[][..], &[0; 32][..], &[0x70, 0x0a, 0xff][..]] {
            assert!(decode(invalid).is_err());
        }
        let mut bounded = Bounded(vec![0; MAX_BMP]);
        assert!(bounded.write_all(&[1]).is_err());
        Ok(())
    }
}
