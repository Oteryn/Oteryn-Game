//! `OTERYN_WORLD_BUNDLE/v1`: writer and fail-closed reader.
//!
//! Layout (little endian), specified in `docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md`:
//! header `"OTWB" | version u16 | reserved u16 | manifest_length u32 | sector_count u32`,
//! canonical JSON manifest, sector table (50-byte rows), zstd frames, 32-byte digest trailer.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::Error;
use crate::sector::{self, Tile, TileLimits};

pub const FORMAT: &str = "OTERYN_WORLD_BUNDLE/v1";
pub const VERSION: u16 = 1;
const MAGIC: &[u8; 4] = b"OTWB";
const HEADER: usize = 16;
const ENTRY: usize = 50;
const DIGEST: usize = 32;
const ZSTD_LEVEL: i32 = 3;

/// Hard maxima of the `MAP01-BUNDLE-*` and `MAP01-TILE-*` rows of the resource limits registry.
pub const MAX_FILE_BYTES: usize = 1 << 30;
pub const MAX_MANIFEST_BYTES: usize = 16 << 20;
pub const MAX_SECTORS: usize = 1 << 20;
pub const MAX_SECTOR_RAW_BYTES: usize = 16 << 20;
pub const MAX_SECTOR_RATIO: usize = 1024;
pub const MAX_TOTAL_RAW_BYTES: usize = 1 << 30;
pub const MAX_TILE_BASE_ENTRIES: usize = 64;
pub const TILE_LIMITS: TileLimits = TileLimits {
    max_entries: 4096,
    max_text_bytes: 4096,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuildClass {
    Production,
    /// Also what a missing value means.
    #[default]
    NonProduction,
    /// Any other value; treated as `non-production` (ADR-0021 §4.2). Never written.
    #[serde(other, skip_serializing)]
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Family {
    Item,
    Terrain,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaletteEntry {
    pub key: String,
    pub family: Family,
    /// The compact id of `key` in the manifest's content revision.
    pub id: u32,
}

/// Declared World extent in native positions: `x` and `y` half-open, floors ascending.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Extent {
    pub min_x: u16,
    pub min_y: u16,
    pub max_x: u16,
    pub max_y: u16,
    pub floors: Vec<i8>,
}

impl Extent {
    pub fn contains(&self, x: u16, y: u16, floor: i8) -> bool {
        (self.min_x..self.max_x).contains(&x)
            && (self.min_y..self.max_y).contains(&y)
            && self.floors.binary_search(&floor).is_ok()
    }
}

/// Compiler inputs copied into the manifest unchanged (ADR-0005 §3, DUR-04 §9).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub project_format_version: String,
    pub world_schema_version: String,
    pub content_revision: String,
    pub content_lock_digest: String,
    pub compiler_version: String,
    pub min_runtime_version: String,
    pub required_capabilities: Vec<String>,
    pub ruleset_compatibility: Vec<String>,
    pub provenance_summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format: String,
    pub min_reader_version: u16,
    pub projection_class: String,
    #[serde(default)]
    pub build_class: BuildClass,
    pub identity: Identity,
    pub world: Extent,
    pub palette: Vec<PaletteEntry>,
    pub draft_areas: Vec<String>,
    pub skipped_provisional_keys: Vec<String>,
}

impl Manifest {
    pub fn is_production(&self) -> bool {
        self.build_class == BuildClass::Production
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sector {
    pub floor: i8,
    pub sx: u16,
    pub sy: u16,
    pub tiles: Vec<Tile>,
}

#[derive(Debug)]
pub struct Bundle {
    pub manifest: Manifest,
    pub sectors: Vec<Sector>,
    pub digest: [u8; 32],
}

/// The placement key of the `ordinal`-th top-level entry of a tile (format document §7).
/// It names an entry only together with the digest of the bundle it was read from.
pub fn placement_key(floor: i8, x: u16, y: u16, ordinal: u8) -> u64 {
    u64::from(x) << 32
        | u64::from(y) << 16
        | u64::from(floor.unsigned_abs()) << 8
        | u64::from(ordinal)
}

fn limit(ok: bool, what: &str) -> Result<(), Error> {
    if ok {
        Ok(())
    } else {
        Err(Error::Limit(what.into()))
    }
}

fn check(ok: bool, what: &str) -> Result<(), Error> {
    if ok {
        Ok(())
    } else {
        Err(Error::Format(what.into()))
    }
}

fn digest_of(body: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(FORMAT.as_bytes());
    hash.update([0]);
    hash.update(body);
    hash.finalize().into()
}

fn validate_manifest(m: &Manifest) -> Result<(), Error> {
    check(
        m.format == FORMAT && m.min_reader_version <= VERSION,
        "unsupported bundle format",
    )?;
    check(
        m.projection_class == "server",
        "projection class must be server",
    )?;
    let w = &m.world;
    check(w.min_x < w.max_x && w.min_y < w.max_y, "empty World bounds")?;
    check(
        !w.floors.is_empty() && w.floors.windows(2).all(|p| p[0] < p[1]),
        "floors unsorted",
    )?;
    check(
        w.floors.iter().all(|f| (-15..=0).contains(f)),
        "floor outside -15..0",
    )?;
    if m.is_production() {
        check(
            m.draft_areas.is_empty(),
            "production bundle carries draft areas",
        )?;
        check(
            m.skipped_provisional_keys.is_empty(),
            "production bundle skipped keys",
        )?;
    }
    Ok(())
}

fn validate_sector(m: &Manifest, s: &Sector) -> Result<(), Error> {
    for tile in &s.tiles {
        check(
            m.world.contains(tile.x, tile.y, s.floor),
            "tile outside the World extent",
        )?;
        let top = tile.items.iter().filter(|item| item.depth == 0).count();
        limit(
            top <= MAX_TILE_BASE_ENTRIES,
            "tile exceeds 64 top-level entries",
        )?;
        for item in &tile.items {
            check(
                (item.palette as usize) < m.palette.len(),
                "palette index out of range",
            )?;
            if let Some((x, y, floor)) = item.attrs.teleport {
                check(
                    m.world.contains(x, y, floor as i8),
                    "teleport outside the World",
                )?;
            }
        }
    }
    Ok(())
}

fn u32_of(value: usize) -> [u8; 4] {
    (value as u32).to_le_bytes()
}

/// Writes a bundle. `sectors` must be strictly ascending by `(floor, sy, sx)`.
pub fn write(manifest: &Manifest, sectors: &[Sector]) -> Result<Vec<u8>, Error> {
    validate_manifest(manifest)?;
    let json = serde_json::to_vec(manifest).map_err(|e| Error::Format(e.to_string()))?;
    limit(json.len() <= MAX_MANIFEST_BYTES, "manifest too large")?;
    limit(sectors.len() <= MAX_SECTORS, "too many sectors")?;
    check(
        sectors.windows(2).all(|p| order(&p[0]) < order(&p[1])),
        "sectors unsorted",
    )?;
    let mut compressor = zstd::bulk::Compressor::new(ZSTD_LEVEL).map_err(zstd_error)?;
    compressor.include_checksum(true).map_err(zstd_error)?;
    compressor.include_contentsize(true).map_err(zstd_error)?;
    let (mut frames, mut total_raw) = (Vec::with_capacity(sectors.len()), 0usize);
    for sector in sectors {
        validate_sector(manifest, sector)?;
        let raw = sector::encode(&sector.tiles)?;
        limit(
            raw.len() <= MAX_SECTOR_RAW_BYTES,
            "sector payload too large",
        )?;
        let frame = compressor.compress(&raw).map_err(zstd_error)?;
        limit(
            raw.len() <= frame.len().saturating_mul(MAX_SECTOR_RATIO),
            "sector ratio too high",
        )?;
        total_raw += raw.len();
        frames.push((raw.len(), frame));
    }
    limit(total_raw <= MAX_TOTAL_RAW_BYTES, "bundle payload too large")?;
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    out.extend_from_slice(&u32_of(json.len()));
    out.extend_from_slice(&u32_of(sectors.len()));
    out.extend_from_slice(&json);
    let mut offset = out.len() + ENTRY * sectors.len();
    for (sector, (raw_length, frame)) in sectors.iter().zip(&frames) {
        out.extend_from_slice(&[sector.floor as u8, 0]);
        out.extend_from_slice(&sector.sx.to_le_bytes());
        out.extend_from_slice(&sector.sy.to_le_bytes());
        out.extend_from_slice(&u32_of(offset));
        out.extend_from_slice(&u32_of(frame.len()));
        out.extend_from_slice(&u32_of(*raw_length));
        out.extend_from_slice(&Sha256::digest(frame));
        offset += frame.len();
    }
    frames
        .iter()
        .for_each(|(_, frame)| out.extend_from_slice(frame));
    let digest = digest_of(&out);
    out.extend_from_slice(&digest);
    limit(out.len() <= MAX_FILE_BYTES, "bundle file too large")?;
    Ok(out)
}

fn order(s: &Sector) -> (i8, u16, u16) {
    (s.floor, s.sy, s.sx)
}

fn zstd_error(error: std::io::Error) -> Error {
    Error::Format(format!("zstd: {error}"))
}

fn le16(data: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([data[at], data[at + 1]])
}

fn le32(data: &[u8], at: usize) -> usize {
    u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]) as usize
}

/// Reads and fully verifies a bundle. Every size is checked before it is allocated.
pub fn read(data: &[u8]) -> Result<Bundle, Error> {
    limit(data.len() <= MAX_FILE_BYTES, "bundle file too large")?;
    check(
        data.len() >= HEADER + DIGEST,
        "bundle shorter than header and trailer",
    )?;
    check(
        &data[..4] == MAGIC && le16(data, 4) == VERSION,
        "not an OTERYN_WORLD_BUNDLE/v1",
    )?;
    check(le16(data, 6) == 0, "reserved header bytes are not zero")?;
    let (body, trailer) = data.split_at(data.len() - DIGEST);
    let digest = digest_of(body);
    check(trailer == digest, "bundle digest mismatch")?;
    let (manifest_length, count) = (le32(data, 8), le32(data, 12));
    limit(manifest_length <= MAX_MANIFEST_BYTES, "manifest too large")?;
    limit(count <= MAX_SECTORS, "too many sectors")?;
    let table = HEADER + manifest_length;
    let mut expected = table + ENTRY * count;
    check(
        expected <= body.len(),
        "sector table runs past the payloads",
    )?;
    let manifest: Manifest = serde_json::from_slice(&data[HEADER..table])
        .map_err(|e| Error::Format(format!("manifest: {e}")))?;
    validate_manifest(&manifest)?;
    let (mut sectors, mut total_raw) = (Vec::with_capacity(count), 0usize);
    for row in 0..count {
        let at = table + ENTRY * row;
        let (floor, sx, sy) = (data[at] as i8, le16(data, at + 2), le16(data, at + 4));
        check(data[at + 1] == 0, "reserved sector byte is not zero")?;
        let (offset, length, raw_length) =
            (le32(data, at + 6), le32(data, at + 10), le32(data, at + 14));
        check(
            offset == expected && length > 0,
            "sector frames must be contiguous",
        )?;
        check(
            body.len() - offset >= length,
            "sector frame runs past the payloads",
        )?;
        limit(
            raw_length <= MAX_SECTOR_RAW_BYTES,
            "sector payload too large",
        )?;
        limit(
            raw_length <= length.saturating_mul(MAX_SECTOR_RATIO),
            "sector ratio too high",
        )?;
        total_raw += raw_length;
        limit(total_raw <= MAX_TOTAL_RAW_BYTES, "bundle payload too large")?;
        check(
            sx < 2048 && sy < 2048,
            "sector outside the u16 coordinate plane",
        )?;
        if let Some(previous) = sectors.last() {
            check(
                order(previous) < (floor, sy, sx),
                "sector table is not ascending",
            )?;
        }
        let frame = &data[offset..offset + length];
        check(
            Sha256::digest(frame).as_slice() == &data[at + 18..at + 50],
            "sector checksum",
        )?;
        let raw = zstd::bulk::decompress(frame, raw_length).map_err(zstd_error)?;
        check(
            raw.len() == raw_length,
            "sector length differs from its table row",
        )?;
        let sector = Sector {
            floor,
            sx,
            sy,
            tiles: sector::decode(&raw, sx, sy, TILE_LIMITS)?,
        };
        validate_sector(&manifest, &sector)?;
        sectors.push(sector);
        expected += length;
    }
    check(expected == body.len(), "bytes after the last sector frame")?;
    Ok(Bundle {
        manifest,
        sectors,
        digest,
    })
}
