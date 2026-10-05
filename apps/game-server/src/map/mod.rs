//! MAP-LOAD-1: the World's base map, loaded from an `OTERYN_WORLD_BUNDLE/v3` (ADR-0021 §4.1,
//! §4.2, §4.8; MAP-LOAD-PACKET-1 §1.1-§1.4).
//!
//! [`load`] takes only the bundle bytes and the pins the World expects, opens no file, verifies
//! the whole bundle with the shared reader (`oteryn-world-bundle`) and returns a [`WorldBase`]:
//! a compact, read-only model of every tile that every Channel of the World shares by `Arc`.
//! Any failure refuses the whole bundle.

use oteryn_world_bundle::Error;
use oteryn_world_bundle::bundle::{
    self, BUNDLE_BUDGET, BuildClass, DIGEST, Manifest, READ_CAPS, ReadCaps, Sector, TerrainKind,
};
use oteryn_world_bundle::sector::{Budget, SECTOR_SIZE};
use std::fmt;

pub mod overlay;
pub mod view;

/// What a World expects of its bundle (§1.1): the bundle digest it pinned, the schema versions
/// and content revision it runs, and whether it is a production World.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundlePins {
    pub digest: [u8; 32],
    pub project_format_version: String,
    pub world_schema_version: String,
    pub content_revision: String,
    pub production: bool,
}

/// Why a bundle was refused. Each refuses the whole bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// The reader refused the bytes (format document §9).
    Bundle(Error),
    /// The bundle digest is not the pinned one.
    Digest,
    /// `project_format_version` or `world_schema_version` is not the pinned one.
    SchemaVersion,
    /// `content_revision` is not the pinned one.
    ContentRevision,
    /// A production World and a bundle whose `build_class` is not `production` (ADR-0021 §4.2).
    NonProductionBundle,
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bundle(error) => write!(f, "world bundle refused: {error}"),
            Self::Digest => f.write_str("world bundle digest is not the pinned digest"),
            Self::SchemaVersion => f.write_str("world bundle schema version is not the pinned one"),
            Self::ContentRevision => {
                f.write_str("world bundle content revision is not the pinned one")
            }
            Self::NonProductionBundle => {
                f.write_str("a production World refuses a non-production world bundle")
            }
        }
    }
}

impl std::error::Error for LoadError {}

impl From<Error> for LoadError {
    fn from(error: Error) -> Self {
        Self::Bundle(error)
    }
}

/// No ground item on the tile.
const NO_GROUND: u8 = u8::MAX;
const SECTOR_TILES: usize = (SECTOR_SIZE as usize) * (SECTOR_SIZE as usize);

/// One tile: its entries (`first_entry..first_entry + entries` of the entry arrays) and its
/// ground item (§1.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BaseTile {
    first_entry: u32,
    entries: u16,
    /// The ground item's stored ground speed, also for a non-walkable ground; 0 without one.
    ground_speed: u16,
    /// The top-level ordinal of the ground item, or [`NO_GROUND`].
    ground: u8,
    walkable: bool,
}

/// The occupied tiles of one sector: bit `ly * 32 + lx` of `occupied`, the occupied count
/// before each word, and the index of the sector's first tile; a tile's index is `first_tile`
/// plus its rank among the occupied bits.
#[derive(Debug, Clone)]
struct SectorTiles {
    occupied: [u64; SECTOR_TILES / 64],
    rank: [u16; SECTOR_TILES / 64],
    first_tile: u32,
}

/// The ground semantics of one palette entry (format v2 `terrain`, §1.4).
#[derive(Debug, Clone, Copy)]
struct PaletteGround {
    id: u32,
    /// `Some((walkable, ground_speed))` for a `ground` Terrain record, `None` otherwise (a
    /// `null` terrain, or another kind).
    ground: Option<(bool, u16)>,
}

/// The compact, read-only base model of a World's map (ADR-0021 §4.1): every tile's position,
/// the palette-resolved compact ids of its entries in payload order with their depths, its
/// ground item, walkable flag and ground speed. It holds no overlay and is never mutated.
#[derive(Debug)]
pub struct WorldBase {
    digest: [u8; 32],
    content_revision: String,
    /// Ascending sector keys ([`sector_key`]), one per entry of `sectors`.
    keys: Vec<u32>,
    /// `rows[r]..rows[r + 1]` are the `keys` of sector row `r` (`key >> 11`, the floor and `sy`),
    /// so a lookup searches one row instead of the whole table.
    rows: Vec<u32>,
    sectors: Vec<SectorTiles>,
    tiles: Vec<BaseTile>,
    ids: Vec<u32>,
    depths: Vec<u8>,
}

/// The number of sector rows: 16 native floors of 2,048 `sy` each.
const SECTOR_ROWS: usize = 16 << 11;

/// The table order `(floor, sy, sx)` as one ascending `u32`: native floors are `-15..=0` and
/// sector coordinates are below 2,048 (format document §5).
fn sector_key(floor: i8, sx: u16, sy: u16) -> Option<u32> {
    if !(-15..=0).contains(&floor) || sx >= 2048 || sy >= 2048 {
        return None;
    }
    let floor = u32::try_from(i32::from(floor) + 15).ok()?;
    Some(floor << 22 | u32::from(sy) << 11 | u32::from(sx))
}

/// A view of one tile of a [`WorldBase`].
#[derive(Debug, Clone, Copy)]
pub struct TileView<'a> {
    base: &'a WorldBase,
    tile: BaseTile,
}

impl<'a> TileView<'a> {
    fn range(&self) -> std::ops::Range<usize> {
        let first = self.tile.first_entry as usize;
        first..first + usize::from(self.tile.entries)
    }

    /// The compact ids of the tile's entries, in payload order.
    pub fn ids(&self) -> &'a [u32] {
        &self.base.ids[self.range()]
    }

    /// The depth of each entry (0 for a top-level entry), in payload order.
    pub fn depths(&self) -> &'a [u8] {
        &self.base.depths[self.range()]
    }

    /// The ground item (§1.3): its top-level ordinal and compact id.
    pub fn ground(&self) -> Option<(u8, u32)> {
        if self.tile.ground == NO_GROUND {
            return None;
        }
        let ordinal = usize::from(self.tile.ground);
        let at = self
            .depths()
            .iter()
            .enumerate()
            .filter(|(_, depth)| **depth == 0)
            .nth(ordinal)?
            .0;
        Some((self.tile.ground, self.ids()[at]))
    }

    /// Whether the ground item is walkable; `false` without one.
    pub fn walkable(&self) -> bool {
        self.tile.walkable
    }

    /// The ground speed the ground item's record stores, also for a non-walkable ground; 0
    /// without a ground item.
    pub fn stored_ground_speed(&self) -> u16 {
        self.tile.ground_speed
    }

    /// The ground speed a step onto this tile paces with (§1.3): 0 without a ground item or on
    /// a non-walkable one.
    pub fn ground_speed(&self) -> u16 {
        if self.tile.walkable {
            self.tile.ground_speed
        } else {
            0
        }
    }
}

impl WorldBase {
    /// The verified bundle digest.
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }

    pub fn content_revision(&self) -> &str {
        &self.content_revision
    }

    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    pub fn entry_count(&self) -> usize {
        self.ids.len()
    }

    /// The tile at native `floor`, `(x, y)`; `None` for a position the bundle holds no tile at.
    pub fn tile(&self, x: u16, y: u16, floor: i8) -> Option<TileView<'_>> {
        let key = sector_key(floor, x / SECTOR_SIZE, y / SECTOR_SIZE)?;
        let row = (key >> 11) as usize;
        let start = self.rows[row] as usize;
        let found = self.keys[start..self.rows[row + 1] as usize]
            .binary_search(&key)
            .ok()?;
        let sector = &self.sectors[start + found];
        let local =
            usize::from(y % SECTOR_SIZE) * usize::from(SECTOR_SIZE) + usize::from(x % SECTOR_SIZE);
        let (word, bit) = (local / 64, local % 64);
        if sector.occupied[word] & 1 << bit == 0 {
            return None;
        }
        let below =
            u32::from(sector.rank[word]) + (sector.occupied[word] & ((1 << bit) - 1)).count_ones();
        let tile = self.tiles[(sector.first_tile + below) as usize];
        Some(TileView { base: self, tile })
    }

    /// The ground speed a step onto native `floor`, `(x, y)` paces with (§1.3): 0 where the
    /// bundle has no tile, no ground item, or a non-walkable one.
    pub fn ground_speed(&self, x: u16, y: u16, floor: i8) -> u16 {
        self.tile(x, y, floor).map_or(0, |tile| tile.ground_speed())
    }

    /// Every tile as `(floor, x, y, view)`, in table order then tile order.
    pub fn tiles(&self) -> impl Iterator<Item = (i8, u16, u16, TileView<'_>)> + '_ {
        self.keys
            .iter()
            .zip(&self.sectors)
            .flat_map(move |(key, sector)| {
                let floor = ((key >> 22) as i8) - 15;
                let (sy, sx) = ((key >> 11 & 0x7FF) as u16, (key & 0x7FF) as u16);
                let mut index = sector.first_tile as usize;
                (0..SECTOR_TILES)
                    .filter(|local| sector.occupied[local / 64] & 1 << (local % 64) != 0)
                    .map(move |local| {
                        let tile = self.tiles[index];
                        index += 1;
                        let x = sx * SECTOR_SIZE + (local % 32) as u16;
                        let y = sy * SECTOR_SIZE + (local / 32) as u16;
                        (floor, x, y, TileView { base: self, tile })
                    })
            })
    }
}

/// Loads a World's base map from the bundle bytes alone (§1.1): refuses a bundle whose digest,
/// schema versions or content revision is not pinned, or a non-production bundle in a
/// production World, and otherwise any bundle the reader refuses.
pub fn load(data: &[u8], pins: &BundlePins) -> Result<WorldBase, LoadError> {
    load_with(data, pins, LOAD_LIMITS)
}

/// The reader maxima a load applies: the `MAP01-BUNDLE-*` file, sector and raw-byte caps, and
/// the decoded tile and entry budget.
#[derive(Debug, Clone, Copy)]
pub struct LoadLimits {
    pub caps: ReadCaps,
    pub budget: Budget,
}

/// The registered maxima ([`READ_CAPS`], [`BUNDLE_BUDGET`]).
pub const LOAD_LIMITS: LoadLimits = LoadLimits {
    caps: READ_CAPS,
    budget: BUNDLE_BUDGET,
};

/// [`load`] with explicit reader maxima; production callers use [`load`].
pub fn load_with(
    data: &[u8],
    pins: &BundlePins,
    limits: LoadLimits,
) -> Result<WorldBase, LoadError> {
    // The trailer is the digest the reader then verifies; a bundle that does not claim the
    // pinned one is refused before it is parsed.
    let trailer = data
        .len()
        .checked_sub(DIGEST)
        .map(|at| &data[at..])
        .ok_or_else(|| Error::Format("bundle shorter than its digest".into()))?;
    if trailer != pins.digest {
        return Err(LoadError::Digest);
    }
    let mut builder = Builder::default();
    let mut refused = None;
    let visited = bundle::visit_budgeted(data, limits.caps, limits.budget, |manifest, sector| {
        if builder.palette.is_none() {
            if let Err(error) = check_pins(manifest, pins) {
                refused = Some(error);
                return Err(Error::Format("bundle does not match its pins".into()));
            }
            builder.palette = Some(palette(manifest));
        }
        builder.push(&sector)
    });
    if let Some(error) = refused {
        return Err(error);
    }
    let visited = visited?;
    check_pins(&visited.manifest, pins)?;
    if visited.digest != pins.digest {
        return Err(LoadError::Digest);
    }
    builder.shrink();
    // Sector counts are capped far below u32::MAX by the reader (READ_CAPS.sectors).
    let rows = (0..=SECTOR_ROWS)
        .map(|row| {
            builder
                .keys
                .partition_point(|key| ((key >> 11) as usize) < row) as u32
        })
        .collect();
    Ok(WorldBase {
        digest: visited.digest,
        content_revision: visited.manifest.identity.content_revision,
        keys: builder.keys,
        rows,
        sectors: builder.sectors,
        tiles: builder.tiles,
        ids: builder.ids,
        depths: builder.depths,
    })
}

fn check_pins(manifest: &Manifest, pins: &BundlePins) -> Result<(), LoadError> {
    let identity = &manifest.identity;
    if identity.project_format_version != pins.project_format_version
        || identity.world_schema_version != pins.world_schema_version
    {
        return Err(LoadError::SchemaVersion);
    }
    if identity.content_revision != pins.content_revision {
        return Err(LoadError::ContentRevision);
    }
    if pins.production && manifest.build_class != BuildClass::Production {
        return Err(LoadError::NonProductionBundle);
    }
    Ok(())
}

/// The compact id and ground semantics of every palette index, from the `terrain` field only.
fn palette(manifest: &Manifest) -> Vec<PaletteGround> {
    manifest
        .palette
        .iter()
        .map(|entry| PaletteGround {
            id: entry.id,
            ground: entry.terrain.as_ref().and_then(|terrain| {
                (terrain.kind == TerrainKind::Ground)
                    .then_some((terrain.walkable?, terrain.ground_speed?))
            }),
        })
        .collect()
}

#[derive(Default)]
struct Builder {
    palette: Option<Vec<PaletteGround>>,
    keys: Vec<u32>,
    sectors: Vec<SectorTiles>,
    tiles: Vec<BaseTile>,
    ids: Vec<u32>,
    depths: Vec<u8>,
}

impl Builder {
    fn shrink(&mut self) {
        self.keys.shrink_to_fit();
        self.sectors.shrink_to_fit();
        self.tiles.shrink_to_fit();
        self.ids.shrink_to_fit();
        self.depths.shrink_to_fit();
    }

    /// Appends one validated sector. The reader hands sectors in ascending table order with
    /// tiles in ascending local order, every floor and position inside the World extent, every
    /// palette index in range, at most 64 top-level and 4,096 entries per tile, and at most
    /// `MAP01-BUNDLE-TILES` tiles and `MAP01-BUNDLE-ENTRIES` entries in all.
    fn push(&mut self, sector: &Sector) -> Result<(), Error> {
        let malformed = |what: &str| Error::Format(format!("world base: {what}"));
        let palette = self
            .palette
            .as_deref()
            .ok_or_else(|| malformed("no palette"))?;
        let key = sector_key(sector.floor, sector.sx, sector.sy)
            .ok_or_else(|| malformed("sector outside the native plane"))?;
        if self.keys.last().is_some_and(|last| *last >= key) {
            return Err(malformed("sector table is not ascending"));
        }
        let first_tile =
            u32::try_from(self.tiles.len()).map_err(|_| malformed("too many tiles"))?;
        let mut occupied = [0u64; SECTOR_TILES / 64];
        let mut previous = None;
        for tile in &sector.tiles {
            let local = usize::from(tile.y.wrapping_sub(sector.sy * SECTOR_SIZE))
                * usize::from(SECTOR_SIZE)
                + usize::from(tile.x.wrapping_sub(sector.sx * SECTOR_SIZE));
            if local >= SECTOR_TILES || previous.is_some_and(|previous| previous >= local) {
                return Err(malformed("tile outside its sector or out of order"));
            }
            previous = Some(local);
            occupied[local / 64] |= 1 << (local % 64);
            let first_entry =
                u32::try_from(self.ids.len()).map_err(|_| malformed("too many entries"))?;
            let entries =
                u16::try_from(tile.items.len()).map_err(|_| malformed("too many entries"))?;
            let (mut ground, mut walkable, mut ground_speed) = (NO_GROUND, false, 0);
            let mut ordinal = 0u8;
            for item in &tile.items {
                let resolved = palette
                    .get(item.palette as usize)
                    .ok_or_else(|| malformed("palette index out of range"))?;
                self.ids.push(resolved.id);
                self.depths.push(item.depth);
                if item.depth != 0 {
                    continue;
                }
                if ground == NO_GROUND
                    && let Some((walks, speed)) = resolved.ground
                {
                    (ground, walkable, ground_speed) = (ordinal, walks, speed);
                }
                ordinal = ordinal
                    .checked_add(1)
                    .filter(|ordinal| *ordinal != NO_GROUND)
                    .ok_or_else(|| malformed("too many top-level entries"))?;
            }
            self.tiles.push(BaseTile {
                first_entry,
                entries,
                ground_speed,
                ground,
                walkable,
            });
        }
        let mut rank = [0u16; SECTOR_TILES / 64];
        for word in 1..rank.len() {
            // At most SECTOR_TILES (1024) bits are set, so the sum fits a u16.
            rank[word] = rank[word - 1] + occupied[word - 1].count_ones() as u16;
        }
        self.keys.push(key);
        self.sectors.push(SectorTiles {
            occupied,
            rank,
            first_tile,
        });
        Ok(())
    }
}
