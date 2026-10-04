//! `OTERYN_WORLD_BUNDLE/v2`: writer and fail-closed reader.
//!
//! Layout (little endian), specified in `docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md`:
//! header `"OTWB" | version u16 | reserved u16 | manifest_length u32 | sector_count u32`,
//! canonical JSON manifest, sector table (50-byte rows), zstd frames, 32-byte digest trailer.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::Error;
use crate::sector::{self, Budget, SECTOR_SIZE, Tile, TileLimits};

pub const FORMAT: &str = "OTERYN_WORLD_BUNDLE/v2";
pub const VERSION: u16 = 2;
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
/// `MAP01-BUNDLE-TILES` and `MAP01-BUNDLE-ENTRIES`: decoded totals of one bundle.
pub const BUNDLE_BUDGET: Budget = Budget {
    tiles: 1 << 25,
    entries: 1 << 26,
};

/// The `compiler_version` this build writes: the crate version and the zstd library version,
/// because both decide the output bytes (format document §6).
pub fn compiler_version() -> String {
    let zstd = zstd::zstd_safe::version_number();
    format!(
        "oteryn-world-bundle-compiler/{} zstd/{}.{}.{}",
        env!("CARGO_PKG_VERSION"),
        zstd / 10000,
        zstd / 100 % 100,
        zstd % 100
    )
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuildClass {
    Production,
    /// Also what a missing value means.
    #[default]
    NonProduction,
    /// Any other string; treated as `non-production` (ADR-0021 §4.2). Never written.
    #[serde(skip_serializing)]
    Unknown,
}

/// Deserialized from a JSON string only: a non-string value (`{"production":null}`, a number,
/// `null`) is an error, never a build class (format document §8).
impl<'de> Deserialize<'de> for BuildClass {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match String::deserialize(deserializer)?.as_str() {
            "production" => Self::Production,
            "non-production" => Self::NonProduction,
            _ => Self::Unknown,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Family {
    Item,
    Terrain,
}

/// Deserialized from a JSON string only; any other string or value is an error.
impl<'de> Deserialize<'de> for Family {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match String::deserialize(deserializer)?.as_str() {
            "item" => Ok(Self::Item),
            "terrain" => Ok(Self::Terrain),
            other => Err(serde::de::Error::custom(format!(
                "unknown family `{other}`"
            ))),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaletteEntry {
    pub key: String,
    pub family: Family,
    /// The compact id of `key` in the manifest's content revision.
    pub id: u32,
    /// Terrain semantics (format v2): `null` for a WorldObject route or a plain Item, required
    /// for a Terrain route. The member itself is never optional.
    #[serde(deserialize_with = "required")]
    pub terrain: Option<Terrain>,
}

/// The kind of a Terrain record (format v2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TerrainKind {
    Ground,
    Border,
    Wall,
    Roof,
    Field,
}

/// The largest `ground_speed` of a ground record (format v2).
pub const MAX_GROUND_SPEED: u16 = 1000;

/// The terrain semantics of a Terrain-routed palette entry (format v2): `walkable` and
/// `ground_speed` are set for `ground` and `null` for every other kind. A walkable ground has a
/// speed of at least 1; a non-walkable ground may have any speed in `0..=1000`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Terrain {
    pub kind: TerrainKind,
    #[serde(deserialize_with = "required")]
    pub walkable: Option<bool>,
    #[serde(deserialize_with = "required")]
    pub ground_speed: Option<u16>,
}

impl Terrain {
    /// Whether the members fit the kind and their ranges.
    pub fn is_valid(&self) -> bool {
        match (self.kind, self.walkable, self.ground_speed) {
            (TerrainKind::Ground, Some(walkable), Some(speed)) => {
                speed <= MAX_GROUND_SPEED && !(walkable && speed == 0)
            }
            (TerrainKind::Ground, ..) => false,
            (_, walkable, speed) => walkable.is_none() && speed.is_none(),
        }
    }
}

/// Makes a member that may be `null` mandatory: serde treats a missing `Option` as `None`
/// unless a `deserialize_with` is given.
fn required<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    Option::deserialize(deserializer)
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
    pub compiler_version: String,
    #[serde(default)]
    pub build_class: BuildClass,
    pub identity: Identity,
    pub world: Extent,
    pub palette: Vec<PaletteEntry>,
    pub draft_areas: Vec<String>,
    pub skipped_provisional_keys: Vec<String>,
    /// Placement keys of the top-level entries whose zero-destination teleport attribute was
    /// dropped (ADR-0021 §4.5); they are never materialized (§4.4). Ascending and unique.
    pub dropped_teleports: Vec<u64>,
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
/// It names an entry only together with the digest of the bundle it was read from. `None`
/// for a floor outside `-15..0` or an ordinal of 64 or more.
pub fn placement_key(floor: i8, x: u16, y: u16, ordinal: u8) -> Option<u64> {
    if !(-15..=0).contains(&floor) || usize::from(ordinal) >= MAX_TILE_BASE_ENTRIES {
        return None;
    }
    Some(
        u64::from(x) << 32
            | u64::from(y) << 16
            | u64::from(floor.unsigned_abs()) << 8
            | u64::from(ordinal),
    )
}

/// Whether `key` names a top-level entry of `sectors` (ascending by `(floor, sy, sx)`).
fn names_entry(sectors: &[Sector], key: u64) -> bool {
    let (x, y) = ((key >> 32) as u16, (key >> 16) as u16);
    let (floor, ordinal) = (-(((key >> 8) & 0xFF) as i16), (key & 0xFF) as usize);
    if key >> 48 != 0 || floor < -15 || ordinal >= MAX_TILE_BASE_ENTRIES {
        return false;
    }
    let at = (floor as i8, y / SECTOR_SIZE, x / SECTOR_SIZE);
    let Ok(sector) = sectors.binary_search_by_key(&at, order) else {
        return false;
    };
    // Tiles of a sector are ascending by `(y, x)` (§5), so each key costs two binary searches.
    let tiles = &sectors[sector].tiles;
    tiles
        .binary_search_by_key(&(y, x), |tile| (tile.y, tile.x))
        .is_ok_and(|at| {
            tiles[at]
                .items
                .iter()
                .filter(|item| item.depth == 0)
                .count()
                > ordinal
        })
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
        m.format == FORMAT && m.min_reader_version == VERSION,
        "unsupported bundle format",
    )?;
    check(
        m.projection_class == "server",
        "projection class must be server",
    )?;
    for entry in &m.palette {
        check(
            entry.family != Family::Terrain || entry.terrain.is_some(),
            "a Terrain palette entry has no terrain semantics",
        )?;
        check(
            entry.terrain.is_none_or(|terrain| terrain.is_valid()),
            "palette terrain is malformed or out of range",
        )?;
    }
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
    for list in [&m.draft_areas, &m.skipped_provisional_keys] {
        check(
            list.windows(2).all(|p| p[0] < p[1]),
            "manifest key list is not sorted and unique",
        )?;
    }
    check(
        m.dropped_teleports.windows(2).all(|p| p[0] < p[1]),
        "dropped teleports are not ascending and unique",
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
    check(!s.tiles.is_empty(), "empty sector")?;
    check(
        m.world.floors.binary_search(&s.floor).is_ok(),
        "sector floor outside the World",
    )?;
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
    let mut budget = BUNDLE_BUDGET;
    for sector in sectors {
        validate_sector(manifest, sector)?;
        let raw = sector::encode(&sector.tiles)?;
        // Write only what the reader reads back identically, within the same limits.
        let decoded = sector::decode(&raw, (sector.sx, sector.sy), TILE_LIMITS, &mut budget)?;
        check(decoded == sector.tiles, "sector does not round-trip")?;
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
    for key in &manifest.dropped_teleports {
        check(
            names_entry(sectors, *key),
            "dropped teleport key names no top-level entry",
        )?;
    }
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

/// Exactly one zstd frame (no skippable frame), with the content checksum flag and a declared
/// content size equal to the table's `raw_length` (format document §5).
fn canonical_frame(frame: &[u8], raw_length: usize) -> bool {
    frame.len() > 4
        && frame[..4] == [0x28, 0xB5, 0x2F, 0xFD]
        && frame[4] & 0x04 != 0
        && zstd::zstd_safe::find_frame_compressed_size(frame) == Ok(frame.len())
        && matches!(zstd::zstd_safe::get_frame_content_size(frame), Ok(Some(n)) if n == raw_length as u64)
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

/// The hard maxima [`read`] enforces. Tests pass reduced values to hit each boundary cheaply.
#[derive(Clone, Copy, Debug)]
pub struct ReadCaps {
    pub file_bytes: usize,
    pub sectors: usize,
    pub sector_raw_bytes: usize,
    pub total_raw_bytes: usize,
}

pub const READ_CAPS: ReadCaps = ReadCaps {
    file_bytes: MAX_FILE_BYTES,
    sectors: MAX_SECTORS,
    sector_raw_bytes: MAX_SECTOR_RAW_BYTES,
    total_raw_bytes: MAX_TOTAL_RAW_BYTES,
};

/// Reads and fully verifies a bundle. Every size is checked before it is allocated.
pub fn read(data: &[u8]) -> Result<Bundle, Error> {
    read_with(data, READ_CAPS)
}

/// [`read`] with explicit maxima; production callers use [`read`].
pub fn read_with(data: &[u8], caps: ReadCaps) -> Result<Bundle, Error> {
    limit(data.len() <= caps.file_bytes, "bundle file too large")?;
    check(
        data.len() >= HEADER + DIGEST,
        "bundle shorter than header and trailer",
    )?;
    check(
        &data[..4] == MAGIC && le16(data, 4) == VERSION,
        "not an OTERYN_WORLD_BUNDLE/v2",
    )?;
    check(le16(data, 6) == 0, "reserved header bytes are not zero")?;
    let (body, trailer) = data.split_at(data.len() - DIGEST);
    let digest = digest_of(body);
    check(trailer == digest, "bundle digest mismatch")?;
    let (manifest_length, count) = (le32(data, 8), le32(data, 12));
    limit(manifest_length <= MAX_MANIFEST_BYTES, "manifest too large")?;
    limit(count <= caps.sectors, "too many sectors")?;
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
    let mut budget = BUNDLE_BUDGET;
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
            raw_length <= caps.sector_raw_bytes,
            "sector payload too large",
        )?;
        limit(
            raw_length <= length.saturating_mul(MAX_SECTOR_RATIO),
            "sector ratio too high",
        )?;
        total_raw += raw_length;
        limit(
            total_raw <= caps.total_raw_bytes,
            "bundle payload too large",
        )?;
        check(
            manifest.world.floors.binary_search(&floor).is_ok(),
            "sector floor outside the World",
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
        check(
            canonical_frame(frame, raw_length),
            "sector is not one canonical zstd frame",
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
            tiles: sector::decode(&raw, (sx, sy), TILE_LIMITS, &mut budget)?,
        };
        validate_sector(&manifest, &sector)?;
        sectors.push(sector);
        expected += length;
    }
    check(expected == body.len(), "bytes after the last sector frame")?;
    for key in &manifest.dropped_teleports {
        check(
            names_entry(&sectors, *key),
            "dropped teleport key names no top-level entry",
        )?;
    }
    Ok(Bundle {
        manifest,
        sectors,
        digest,
    })
}
