//! The real-sprite world around the start position (CLIENT-VIS-1, milestone 1).
//!
//! A bounded square of the floor-7 base map around the Thais temple is decoded once from the
//! B3 region files, every item is resolved through the pinned 15.30 appearances, and the cells
//! they need are copied into one static atlas. Walking only rebuilds quads over it. Full map
//! streaming is milestone 2.
//!
//! The data comes from a local asset root laid out like the repository (`OTERYN_ASSET_DIR`);
//! the installer ships no map or sprite files. Without it, [`World::builtin`] draws only the
//! builtin cells.

use crate::input::StepDir;
use oteryn_client_assets::{
    AppearanceIndex, AssetStore, CELL_PX, Catalog, DrawCell, Hook, Placement, SpriteSheets,
};
use oteryn_renderer::{AtlasImage, BatchError, MAX_ATLAS_DIMENSION};
use oteryn_world_bundle::sector::{self, Budget, TileLimits};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::path::Path;

/// One resolved map item: draw layer, elevation and cells.
type Item = (u8, i32, Vec<DrawCell>);

/// Names the local asset root.
pub const ASSET_DIR_ENV: &str = "OTERYN_ASSET_DIR";
/// The Thais temple, where the offline view starts and the play view is anchored.
pub const START: (i32, i32) = (32369, 32241);
pub const START_FLOOR: u8 = 7;
/// Tiles decoded on each side of [`START`].
pub const RADIUS: i32 = 64;
/// The default outfit (citizen).
pub const PLAYER_LOOK_TYPE: u32 = 128;
/// Outfit directions in pattern order.
const FACINGS: [StepDir; 4] = [StepDir::North, StepDir::East, StepDir::South, StepDir::West];

/// Builtin cells ahead of the sprite cells: opaque black under every tile, the target
/// outline, and the marker glyph for overlay objects.
pub const BLACK_CELL: u16 = 0;
pub const TARGET_CELL: u16 = 1;
pub const MARKER_CELL: u16 = 2;
const BUILTIN_CELLS: usize = 3;

const ATLAS_COLUMNS: usize = (MAX_ATLAS_DIMENSION / CELL_PX) as usize;
const MAX_CELLS: usize = ATLAS_COLUMNS * ATLAS_COLUMNS;
const MAX_INDEX_BYTES: u64 = 8 * 1024 * 1024;
const MAX_REGION_BYTES: u64 = 32 * 1024 * 1024;
const MAX_SECTOR_BYTES: usize = 16 * 1024 * 1024;
const REGION_TILES: i32 = 256;
/// The side of one sector: a region is 8 x 8 sectors.
const SECTOR_TILES: i32 = REGION_TILES / 8;
/// The most tiles in the sectors the bounded window can overlap.
#[allow(clippy::cast_sign_loss)]
const WINDOW_TILES: usize = (((2 * RADIUS + 1) / SECTOR_TILES + 2) * SECTOR_TILES).pow(2) as usize;
/// The most cells one tile, or the player, may draw, so a full view stays inside one
/// sprite batch.
pub const MAX_TILE_DRAWS: usize = 256;
/// The client's cap on how far elevation lifts what is drawn above it.
const MAX_ELEVATION: i32 = 24;

/// One atlas cell drawn at a pixel offset from a tile's top-left, in 32 px source pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Draw {
    pub cell: u16,
    pub offset: [i32; 2],
}

/// What one map tile draws: `under` before a creature standing on it, `over` after it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapTile {
    pub under: Vec<Draw>,
    /// How far a creature on this tile is lifted, in source pixels.
    pub elevation: i32,
    pub over: Vec<Draw>,
}

#[derive(Debug, Clone)]
pub struct World {
    atlas: AtlasImage,
    tiles: HashMap<(i32, i32), MapTile>,
    /// The own player's cells per facing, in [`FACINGS`] order.
    player: [Vec<Draw>; 4],
}

#[derive(Deserialize)]
struct PlacementIndex {
    palette: Vec<PaletteEntry>,
    regions: Vec<RegionEntry>,
}

#[derive(Deserialize)]
struct PaletteEntry {
    source_item_id: u32,
}

#[derive(Deserialize)]
struct RegionEntry {
    path: String,
    sha256: String,
}

impl World {
    /// Only the builtin cells: no map, and the player drawn as the marker glyph.
    pub fn builtin() -> Result<Self, BatchError> {
        let atlas = AtlasImage::new(CELL_PX, BUILTIN_CELLS as u32, 1, builtin_rgba())?;
        Ok(Self {
            atlas,
            tiles: HashMap::new(),
            player: FACINGS.map(|_| {
                vec![Draw {
                    cell: MARKER_CELL,
                    offset: [0, 0],
                }]
            }),
        })
    }

    /// Loads from `OTERYN_ASSET_DIR`; any failure falls back to [`World::builtin`] with the reason.
    pub fn from_env() -> (Result<Self, BatchError>, Option<String>) {
        let loaded = std::env::var_os(ASSET_DIR_ENV)
            .ok_or_else(|| format!("{ASSET_DIR_ENV} is not set"))
            .and_then(|root| Self::load(Path::new(&root)));
        match loaded {
            Ok(world) => (Ok(world), None),
            Err(reason) => (Self::builtin(), Some(reason)),
        }
    }

    /// Decodes the start area from an asset root laid out like the repository.
    pub fn load(root: &Path) -> Result<Self, String> {
        let store = AssetStore::open(
            &root.join("content/assets/files"),
            &root.join("imports/official/client-assets/15.30/manifest.json"),
        )
        .map_err(|error| format!("assets: {error}"))?;
        let catalog = Catalog::load(&store).map_err(|error| format!("catalog: {error}"))?;
        let index = AppearanceIndex::load(&store, &catalog)
            .map_err(|error| format!("appearances: {error}"))?;
        let mut sheets = SpriteSheets::new(store, catalog);
        let placements = root.join("content/world/placements");
        let map: PlacementIndex = serde_json::from_slice(&read_capped(
            &placements.join("index.json"),
            MAX_INDEX_BYTES,
        )?)
        .map_err(|error| format!("placement index: {error}"))?;

        let (lo_x, hi_x, lo_y, hi_y) = (
            START.0 - RADIUS,
            START.0 + RADIUS,
            START.1 - RADIUS,
            START.1 + RADIUS,
        );
        // Every tile's resolved items, then the cells they use.
        let mut entries: Vec<((i32, i32), Vec<Item>)> = Vec::new();
        // One budget for the whole window, so the asset directory cannot make the regions
        // together decode more than the bounded region needs.
        let mut budget = Budget {
            tiles: WINDOW_TILES,
            entries: 1024 * 1024,
        };
        for ry in lo_y / REGION_TILES..=hi_y / REGION_TILES {
            for rx in lo_x / REGION_TILES..=hi_x / REGION_TILES {
                let name = format!("region-z{START_FLOOR:02}-x{rx:03}-y{ry:03}.b3");
                let Some(entry) = map
                    .regions
                    .iter()
                    .find(|entry| entry.path.rsplit('/').next() == Some(name.as_str()))
                else {
                    continue;
                };
                let data = read_capped(&placements.join(&name), MAX_REGION_BYTES)?;
                let digest = Sha256::digest(&data);
                let hex = digest
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>();
                if hex != entry.sha256 {
                    return Err(format!("{name}: sha256 does not match the placement index"));
                }
                for tile in decode_region(
                    &data,
                    (START_FLOOR, rx, ry),
                    (lo_x, hi_x, lo_y, hi_y),
                    &mut budget,
                )
                .map_err(|error| format!("{name}: {error}"))?
                {
                    let (x, y) = (i32::from(tile.x), i32::from(tile.y));
                    if !(lo_x..=hi_x).contains(&x) || !(lo_y..=hi_y).contains(&y) {
                        continue;
                    }
                    let mut stack = Vec::new();
                    let mut hook = Hook::None;
                    for item in tile.items.iter().filter(|item| item.depth == 0) {
                        let palette = palette_entry(&map.palette, item.palette, &name)?;
                        // A hangable entry follows the wall hook of any entry on its tile; a
                        // south hook wins over an east one, as in the reference resolver.
                        if let Some(appearance) = index.get(palette.source_item_id)
                            && hook != Hook::South
                            && appearance.hook != Hook::None
                        {
                            hook = appearance.hook;
                        }
                        stack.push((palette, item));
                    }
                    let mut items = Vec::new();
                    for (palette, item) in stack {
                        let placement = Placement {
                            hook,
                            ..placement(x, y, item.attrs.count)
                        };
                        // An item the pinned appearances cannot draw is skipped, not fatal.
                        let Ok(resolved) =
                            index.resolve(&sheets, palette.source_item_id, placement)
                        else {
                            continue;
                        };
                        let layer = if resolved.ground {
                            0
                        } else if resolved.ground_border {
                            1
                        } else if resolved.on_bottom {
                            2
                        } else if resolved.on_top {
                            4
                        } else {
                            3
                        };
                        items.push((layer, item_height(resolved.elevation), resolved.cells));
                    }
                    // Stable: items of one layer keep their stack order.
                    items.sort_by_key(|(layer, _, _)| *layer);
                    entries.push(((x, y), items));
                }
            }
        }
        let mut player: [Vec<DrawCell>; 4] = Default::default();
        for (direction, cells) in (0..).zip(&mut player) {
            *cells = index
                .resolve_outfit(&sheets, PLAYER_LOOK_TYPE, direction)
                .map_err(|error| format!("outfit {PLAYER_LOOK_TYPE}: {error}"))?
                .cells;
        }

        // Cells in sprite order, so each sheet is decoded once; past the atlas cap they are dropped.
        let mut cells = BTreeMap::new();
        for cell in entries
            .iter()
            .flat_map(|(_, items)| items.iter().flat_map(|(_, _, cells)| cells))
            .chain(player.iter().flatten())
        {
            cells.insert((cell.sprite_id, cell.cell_x, cell.cell_y), 0_u16);
        }
        let mut rgba = builtin_rgba();
        let mut next = BUILTIN_CELLS;
        for (key, slot) in &mut cells {
            if next >= MAX_CELLS {
                break;
            }
            let pixels = sheets
                .cell_rgba(key.0, key.1, key.2)
                .map_err(|error| format!("sprite {}: {error}", key.0))?;
            *slot = next as u16;
            rgba.extend_from_slice(&pixels);
            next += 1;
        }
        let draw = |cell: &DrawCell, lift: i32| {
            let slot = cells.get(&(cell.sprite_id, cell.cell_x, cell.cell_y))?;
            (*slot != 0).then_some(Draw {
                cell: *slot,
                offset: [cell.offset_x - lift, cell.offset_y - lift],
            })
        };
        let mut tiles = HashMap::with_capacity(entries.len());
        for (position, items) in &entries {
            let mut tile = MapTile::default();
            let (lifts, elevation) = stack_lifts(items.iter().map(|(layer, e, _)| (*layer, *e)));
            for ((layer, _, item_cells), lift) in items.iter().zip(lifts) {
                let drawn = item_cells.iter().filter_map(|cell| draw(cell, lift));
                if *layer == 4 {
                    tile.over.extend(drawn);
                } else {
                    tile.under.extend(drawn);
                }
            }
            tile.elevation = elevation;
            within_draw_cap(tile.under.len() + tile.over.len())
                .map_err(|error| format!("tile {position:?}: {error}"))?;
            tiles.insert(*position, tile);
        }
        let player: [Vec<Draw>; 4] =
            player.map(|cells| cells.iter().filter_map(|cell| draw(cell, 0)).collect());
        for cells in &player {
            within_draw_cap(cells.len()).map_err(|error| format!("outfit: {error}"))?;
        }
        let atlas = atlas_from_cells(rgba, next).map_err(|error| format!("atlas: {error}"))?;
        Ok(Self {
            atlas,
            tiles,
            player,
        })
    }

    #[must_use]
    pub const fn atlas(&self) -> &AtlasImage {
        &self.atlas
    }

    /// The map tile at an absolute map position, if it lies in the loaded area.
    #[must_use]
    pub fn tile(&self, x: i32, y: i32) -> Option<&MapTile> {
        self.tiles.get(&(x, y))
    }

    #[must_use]
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// The own player's cells facing `facing`, standing on a tile.
    #[must_use]
    pub fn player(&self, facing: StepDir) -> &[Draw] {
        let index = FACINGS.iter().position(|f| *f == facing).unwrap_or(2);
        &self.player[index]
    }
}

/// Lays `count` 32 px cells, stored one after another, out in rows of [`ATLAS_COLUMNS`].
fn atlas_from_cells(cells: Vec<u8>, count: usize) -> Result<AtlasImage, BatchError> {
    let columns = count.min(ATLAS_COLUMNS);
    let rows = count.div_ceil(columns);
    let px = CELL_PX as usize;
    let mut rgba = vec![0; columns * px * rows * px * 4];
    for (index, cell) in cells.chunks_exact(px * px * 4).enumerate() {
        let (column, row) = (index % columns, index / columns);
        for line in 0..px {
            let at = ((row * px + line) * columns * px + column * px) * 4;
            rgba[at..at + px * 4].copy_from_slice(&cell[line * px * 4..(line + 1) * px * 4]);
        }
    }
    AtlasImage::new(CELL_PX, columns as u32, rows as u32, rgba)
}

/// The builtin cells, one after another: black, a yellow outline, a red square.
fn builtin_rgba() -> Vec<u8> {
    let px = CELL_PX as usize;
    let mut rgba = Vec::with_capacity(BUILTIN_CELLS * px * px * 4);
    for cell in 0..BUILTIN_CELLS {
        for y in 0..px {
            for x in 0..px {
                let edge = x.min(y).min(px - 1 - x).min(px - 1 - y);
                rgba.extend_from_slice(&match cell {
                    0 => [0, 0, 0, 255],
                    1 if edge < 2 => [255, 220, 40, 255],
                    2 if edge >= 10 => [220, 40, 40, 255],
                    _ => [0, 0, 0, 0],
                });
            }
        }
    }
    rgba
}

/// How far each item of a sorted stack is lifted, and the tile's elevation. An item is lifted by
/// its own height (the appearance profile's displacement) on top of the heights below it; a top
/// item does not stand on the stack.
fn stack_lifts(items: impl IntoIterator<Item = (u8, i32)>) -> (Vec<i32>, i32) {
    let mut below = 0;
    let lifts = items
        .into_iter()
        .map(|(layer, height)| {
            if layer == 4 {
                return height;
            }
            let lift = (below + height).min(MAX_ELEVATION);
            below = lift;
            lift
        })
        .collect();
    (lifts, below)
}

/// An appearance's elevation as a stack height, capped like the stack itself so an oversized
/// profile value can neither wrap negative nor overflow the sum below it.
fn item_height(elevation: u32) -> i32 {
    i32::try_from(elevation).map_or(MAX_ELEVATION, |height| height.min(MAX_ELEVATION))
}

/// The palette entry an item names; an index outside the palette fails the whole map closed.
fn palette_entry<'a>(
    palette: &'a [PaletteEntry],
    index: u32,
    region: &str,
) -> Result<&'a PaletteEntry, String> {
    usize::try_from(index)
        .ok()
        .and_then(|index| palette.get(index))
        .ok_or_else(|| format!("{region}: palette index {index} outside the placement index"))
}

/// A start-floor placement; the encoded value is a stack count or, for fluids, the subtype.
fn placement(x: i32, y: i32, encoded: Option<u8>) -> Placement {
    let mut placement = Placement::at(x, y, -i32::from(START_FLOOR));
    placement.count = u32::from(encoded.unwrap_or(1));
    placement.sub_type = u32::from(encoded.unwrap_or(0));
    placement
}

#[cfg(test)]
impl World {
    /// The builtin world with the given map tiles.
    pub(crate) fn builtin_with(
        tiles: impl IntoIterator<Item = ((i32, i32), MapTile)>,
    ) -> Result<Self, BatchError> {
        let mut world = Self::builtin()?;
        world.tiles.extend(tiles);
        Ok(world)
    }
}

fn read_capped(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    let file = std::fs::File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    if bytes.len() as u64 > max {
        return Err(format!("{}: larger than {max} bytes", path.display()));
    }
    Ok(bytes)
}

/// Refuses more than [`MAX_TILE_DRAWS`] cells, so the world falls back to the empty map
/// instead of failing the renderer.
fn within_draw_cap(draws: usize) -> Result<(), String> {
    if draws > MAX_TILE_DRAWS {
        return Err(format!("{draws} cells over the {MAX_TILE_DRAWS} cap"));
    }
    Ok(())
}

/// Decodes an `OTERYN_WORLD_REGION_B3/v1` file the way `world-bundle-compiler`'s `b3` reader
/// does: a 12-byte header, `local u8 | offset u32 | length u32` rows, one zstd frame per sector.
/// The header must name `region` (`z, rx, ry`), the region the file was looked up for. Only
/// sectors that overlap `window` (`lo_x, hi_x, lo_y, hi_y`, inclusive) are decompressed.
fn decode_region(
    data: &[u8],
    region: (u8, i32, i32),
    window: (i32, i32, i32, i32),
    budget: &mut Budget,
) -> Result<Vec<sector::Tile>, String> {
    const HEADER: usize = 12;
    const ROW: usize = 9;
    let le16 = |at: usize| u16::from_le_bytes([data[at], data[at + 1]]);
    let le32 = |at: usize| {
        u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]) as usize
    };
    if data.len() < HEADER || &data[..4] != b"OTRB" || data[4] != 1 {
        return Err("not an OTERYN_WORLD_REGION_B3/v1 file".into());
    }
    let (rx, ry, count) = (le16(6), le16(8), usize::from(le16(10)));
    if count == 0 || count > 64 || rx > 255 || ry > 255 {
        return Err("header out of range".into());
    }
    if (data[5], i32::from(rx), i32::from(ry)) != region {
        return Err("header names another region".into());
    }
    let mut expected = HEADER + ROW * count;
    if data.len() < expected {
        return Err("sector table runs past the end of the file".into());
    }
    let limits = TileLimits {
        max_entries: 4096,
        max_text_bytes: 4096,
    };
    let mut tiles = Vec::new();
    let mut previous = -1;
    for row in 0..count {
        let at = HEADER + ROW * row;
        let (local, offset, length) = (data[at], le32(at + 1), le32(at + 5));
        if i32::from(local) <= previous || local >= 64 {
            return Err("sector table must be strictly ascending within 0..63".into());
        }
        if offset != expected || length == 0 || data.len() - offset < length {
            return Err("sector payloads must be non-empty and contiguous".into());
        }
        previous = i32::from(local);
        expected += length;
        let (sx, sy) = (rx * 8 + u16::from(local) % 8, ry * 8 + u16::from(local) / 8);
        let (x0, y0) = (i32::from(sx) * SECTOR_TILES, i32::from(sy) * SECTOR_TILES);
        let (lo_x, hi_x, lo_y, hi_y) = window;
        if x0 > hi_x || x0 + SECTOR_TILES <= lo_x || y0 > hi_y || y0 + SECTOR_TILES <= lo_y {
            continue;
        }
        let frame = &data[offset..offset + length];
        let capacity = match zstd::zstd_safe::get_frame_content_size(frame) {
            Ok(Some(size)) if size <= MAX_SECTOR_BYTES as u64 => size as usize,
            Ok(Some(_)) => return Err("sector payload over the cap".into()),
            _ => MAX_SECTOR_BYTES,
        };
        let payload = zstd::bulk::decompress(frame, capacity)
            .map_err(|error| format!("sector {local}: {error}"))?;
        tiles.extend(
            sector::decode(&payload, (sx, sy), limits, budget)
                .map_err(|error| format!("sector {local}: {error:?}"))?,
        );
    }
    if expected != data.len() {
        return Err("bytes after the last sector payload".into());
    }
    Ok(tiles)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn the_builtin_world_has_only_the_builtin_cells() -> Result<(), BatchError> {
        let world = World::builtin()?;
        assert_eq!(world.atlas().cell_count() as usize, BUILTIN_CELLS);
        assert_eq!(world.tile_count(), 0);
        assert_eq!(world.player(StepDir::South)[0].cell, MARKER_CELL);
        Ok(())
    }

    #[test]
    fn the_temple_area_resolves_to_real_cells_inside_the_atlas() -> Result<(), String> {
        let world = World::load(&repo())?;
        let cells = world.atlas().cell_count();
        assert!(cells as usize > BUILTIN_CELLS + 100, "{cells} cells");
        assert!(world.tile_count() > 10_000, "{} tiles", world.tile_count());
        let start = world.tile(START.0, START.1).ok_or("no start tile")?;
        assert!(!start.under.is_empty(), "the start tile has ground");
        for tile in world.tiles.values() {
            for draw in tile.under.iter().chain(&tile.over) {
                assert!(draw.cell >= BUILTIN_CELLS as u16 && u32::from(draw.cell) < cells);
                world
                    .atlas()
                    .uv_rect(draw.cell)
                    .map_err(|e| e.to_string())?;
            }
        }
        for facing in FACINGS {
            let player = world.player(facing);
            assert!(!player.is_empty());
            assert!(player.iter().all(|draw| draw.cell >= BUILTIN_CELLS as u16));
        }
        assert_ne!(world.player(StepDir::North), world.player(StepDir::South));
        Ok(())
    }

    #[test]
    fn an_encoded_fluid_value_selects_the_fluid_pattern() -> Result<(), String> {
        let root = repo();
        let store = AssetStore::open(
            &root.join("content/assets/files"),
            &root.join("imports/official/client-assets/15.30/manifest.json"),
        )
        .map_err(|e| e.to_string())?;
        let catalog = Catalog::load(&store).map_err(|e| e.to_string())?;
        let index = AppearanceIndex::load(&store, &catalog).map_err(|e| e.to_string())?;
        let sheets = SpriteSheets::new(store, catalog);
        let (x, y) = START;
        assert_eq!(placement(x, y, Some(8)).sub_type, 8);
        assert_eq!(placement(x, y, None).count, 1);
        // Fluid appearance 2524 is on the start area with encoded values 1 and 8. Sub-types 8
        // and 12 share colour 5; sub-type 1 is colour 1.
        let mut resolved = Vec::new();
        for encoded in [1, 8, 12] {
            let entry = index
                .resolve(&sheets, 2524, placement(x, y, Some(encoded)))
                .map_err(|e| e.to_string())?;
            resolved.push(entry.cells.iter().map(|c| c.sprite_id).collect::<Vec<_>>());
        }
        assert_ne!(resolved[0], resolved[1]);
        assert_eq!(resolved[1], resolved[2]);
        Ok(())
    }

    #[test]
    fn an_item_is_lifted_by_its_own_height_and_the_heights_below_it() {
        // Ground, a height-8 table, an item on it, another height-8 item, a top item of height 8.
        let (lifts, elevation) = stack_lifts([(0, 0), (3, 8), (3, 0), (3, 8), (4, 8)]);
        assert_eq!(lifts, [0, 8, 8, 16, 8]);
        assert_eq!(elevation, 16);
        assert_eq!(item_height(8), 8);
        assert_eq!(item_height(u32::MAX), MAX_ELEVATION);
        assert_eq!(item_height(i32::MAX as u32 + 1), MAX_ELEVATION);
        let (lifts, elevation) = stack_lifts([(3, 16), (3, 16)]);
        assert_eq!(lifts, [16, MAX_ELEVATION]);
        assert_eq!(elevation, MAX_ELEVATION);
    }

    #[test]
    fn a_palette_index_outside_the_placement_index_is_refused() {
        let palette = [PaletteEntry { source_item_id: 7 }];
        let found = palette_entry(&palette, 0, "region");
        assert_eq!(found.map(|entry| entry.source_item_id), Ok(7));
        assert_eq!(
            palette_entry(&palette, 1, "region").err().as_deref(),
            Some("region: palette index 1 outside the placement index")
        );
    }

    #[test]
    fn a_tile_over_the_draw_cap_is_refused() {
        assert!(within_draw_cap(MAX_TILE_DRAWS).is_ok());
        assert!(within_draw_cap(MAX_TILE_DRAWS + 1).is_err());
    }

    #[test]
    fn a_region_whose_header_names_another_region_is_refused() {
        let header = b"OTRB\x01\x06\x7e\x00\x7d\x00\x01\x00";
        let decode = |region| {
            decode_region(
                header,
                region,
                (0, 0, 0, 0),
                &mut Budget {
                    tiles: 1,
                    entries: 1,
                },
            )
        };
        assert_eq!(
            decode((7, 126, 125)).err().as_deref(),
            Some("header names another region")
        );
        assert_eq!(
            decode((6, 126, 124)).err().as_deref(),
            Some("header names another region")
        );
        assert_ne!(
            decode((6, 126, 125)).err().as_deref(),
            Some("header names another region")
        );
    }

    #[test]
    fn a_region_that_does_not_match_its_digest_is_refused() -> Result<(), String> {
        let bad = decode_region(
            b"OTRB\x02\x07\x7e\x00\x7d\x00\x01\x00",
            (7, 126, 125),
            (0, 0, 0, 0),
            &mut Budget {
                tiles: 1,
                entries: 1,
            },
        );
        assert!(bad.is_err());
        let missing = World::load(&repo().join("no-such-root"));
        assert!(missing.is_err());
        Ok(())
    }
}
