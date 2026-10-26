//! The real-sprite world around the start position (CLIENT-VIS-1, milestone 1).
//!
//! A bounded square of the actual join floor around its position is decoded once from the
//! B3 region files, every item is resolved through the pinned 15.30 appearances, and the cells
//! they need are copied into one static atlas. Walking only rebuilds quads over it. Full map
//! streaming is milestone 2.
//!
//! The data comes from a local asset root laid out like the repository (`OTERYN_ASSET_DIR`);
//! the installer ships no map or sprite files. Without it, [`World::builtin`] draws only the
//! builtin cells.

use crate::input::StepDir;
use oteryn_client_assets::world::{
    Budget, MANIFEST_PATH, MANIFEST_SHA256, MAX_REGION_BYTES, REGION_TILES, SECTOR_TILES,
    decode_region, palette_entry, read_capped, read_placement_index,
};
use oteryn_client_assets::{
    AppearanceIndex, AssetStore, CELL_PX, Catalog, DrawCell, Hook, Placement, SpriteSheets,
};
use oteryn_renderer::{AtlasImage, BatchError, MAX_ATLAS_DIMENSION};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

/// One resolved map item: draw layer, elevation and cells.
type Item = (u8, i32, Vec<DrawCell>);

/// Names the local asset root.
pub const ASSET_DIR_ENV: &str = "OTERYN_ASSET_DIR";
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
/// The most tiles in the sectors the bounded window can overlap.
#[allow(clippy::cast_sign_loss)]
const WINDOW_TILES: usize = (((2 * RADIUS + 1) / SECTOR_TILES + 2) * SECTOR_TILES).pow(2) as usize;
/// The most cells one tile, or the player, may draw, so a full view stays inside one
/// sprite batch.
pub const MAX_TILE_DRAWS: usize = 256;
/// The most resolved cells the whole window may hold before the atlas is built.
const MAX_WINDOW_CELLS: usize = 1024 * 1024;
/// The client's cap on how far elevation lifts what is drawn above it.
const MAX_ELEVATION: i32 = 24;
/// How far up and left, in source pixels, a tile's cells may reach: the scene scans
/// [`SCAN_MARGIN`] tiles past the view on the right and bottom.
pub const SCAN_MARGIN: i32 = 2;
const MAX_REACH_PX: i32 = SCAN_MARGIN * CELL_PX as i32;

/// Compatibility exports; new callers should use `oteryn_client_domain::world`.
pub use oteryn_client_domain::world::{Draw, MapTile, START, START_FLOOR};

#[derive(Debug, Clone)]
pub struct World {
    center: (i32, i32, i16),
    atlas: AtlasImage,
    tiles: HashMap<(i32, i32), MapTile>,
    /// The own player's cells per facing, in [`FACINGS`] order.
    player: [Vec<Draw>; 4],
}

impl World {
    /// Only the builtin cells: no map, and the player drawn as the marker glyph.
    pub fn builtin() -> Result<Self, BatchError> {
        let atlas = AtlasImage::new(CELL_PX, BUILTIN_CELLS as u32, 1, builtin_rgba())?;
        Ok(Self {
            center: (START.0, START.1, i16::from(START_FLOOR)),
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
    pub fn from_env(center: (i32, i32, i16)) -> (Result<Self, BatchError>, Option<String>) {
        let loaded = std::env::var_os(ASSET_DIR_ENV)
            .ok_or_else(|| format!("{ASSET_DIR_ENV} is not set"))
            .and_then(|root| Self::load_around(Path::new(&root), center));
        match loaded {
            Ok(world) => (Ok(world), None),
            Err(reason) => (Self::builtin(), Some(reason)),
        }
    }

    /// Decodes the start area from an asset root laid out like the repository.
    pub fn load(root: &Path) -> Result<Self, String> {
        Self::load_around(root, (START.0, START.1, i16::from(START_FLOOR)))
    }

    pub const fn center(&self) -> (i32, i32, i16) {
        self.center
    }

    /// Decode the real join position's bounded map window, including its floor.
    pub fn load_around(root: &Path, center: (i32, i32, i16)) -> Result<Self, String> {
        let floor = u8::try_from(center.2)
            .ok()
            .filter(|floor| *floor <= 15)
            .ok_or("map center floor outside the World")?;
        if !(0..=65535).contains(&center.0) || !(0..=65535).contains(&center.1) {
            return Err("map center outside the World".into());
        }
        let store = AssetStore::open_pinned(
            &root.join("content/assets/files"),
            &root.join(MANIFEST_PATH),
            MANIFEST_SHA256,
        )
        .map_err(|error| format!("assets: {error}"))?;
        let catalog = Catalog::load(&store).map_err(|error| format!("catalog: {error}"))?;
        let index = AppearanceIndex::load(&store, &catalog)
            .map_err(|error| format!("appearances: {error}"))?;
        let mut sheets = SpriteSheets::new(store, catalog);
        let placements = root.join("content/world/placements");
        let map = read_placement_index(root)?;

        let (lo_x, hi_x, lo_y, hi_y) = (
            (center.0 - RADIUS).max(0),
            (center.0 + RADIUS).min(65535),
            (center.1 - RADIUS).max(0),
            (center.1 + RADIUS).min(65535),
        );
        // Every tile's resolved items, then the cells they use.
        let mut entries: Vec<((i32, i32), Vec<Item>)> = Vec::new();
        // One budget for the whole window, so the asset directory cannot make the regions
        // together decode more than the bounded region needs.
        let mut window_cells = 0;
        let mut budget = Budget {
            tiles: WINDOW_TILES,
            entries: 1024 * 1024,
        };
        for ry in lo_y / REGION_TILES..=hi_y / REGION_TILES {
            for rx in lo_x / REGION_TILES..=hi_x / REGION_TILES {
                let name = format!("region-z{floor:02}-x{rx:03}-y{ry:03}.b3");
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
                    (floor, rx, ry),
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
                    // Refused as soon as the tile resolves to more cells than it may draw, so an
                    // overfull tile is not held until the whole window is resolved.
                    let mut tile_cells = 0;
                    for (palette, item) in stack {
                        let placement = Placement {
                            hook,
                            ..placement(x, y, item.attrs.count, floor)
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
                        tile_cells += resolved.cells.len();
                        within_draw_cap(tile_cells)
                            .map_err(|error| format!("{name}: tile ({x}, {y}): {error}"))?;
                        window_cells += resolved.cells.len();
                        within_window_cap(window_cells)?;
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
        // A cell reaching further up or left than the scene scans past the view is not drawn.
        let draw = |cell: &DrawCell, lift: i32| {
            let slot = cells.get(&(cell.sprite_id, cell.cell_x, cell.cell_y))?;
            let offset = [cell.offset_x - lift, cell.offset_y - lift];
            (*slot != 0 && within_reach(offset)).then_some(Draw {
                cell: *slot,
                offset,
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
            tiles.insert(*position, tile);
        }
        let player: [Vec<Draw>; 4] =
            player.map(|cells| cells.iter().filter_map(|cell| draw(cell, 0)).collect());
        for cells in &player {
            within_draw_cap(cells.len()).map_err(|error| format!("outfit: {error}"))?;
        }
        let atlas = atlas_from_cells(rgba, next).map_err(|error| format!("atlas: {error}"))?;
        Ok(Self {
            center,
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

/// A start-floor placement; the encoded value is a stack count or, for fluids, the subtype.
fn placement(x: i32, y: i32, encoded: Option<u8>, floor: u8) -> Placement {
    let mut placement = Placement::at(x, y, -i32::from(floor));
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

/// Whether a map cell at `offset` (lift included) stays within [`MAX_REACH_PX`] of its tile,
/// so the scene's scan past the view reaches it.
fn within_reach(offset: [i32; 2]) -> bool {
    offset.iter().all(|axis| *axis >= -MAX_REACH_PX)
}

/// Refuses a window that resolves to more than [`MAX_WINDOW_CELLS`] cells.
fn within_window_cap(cells: usize) -> Result<(), String> {
    if cells > MAX_WINDOW_CELLS {
        return Err(format!("window cells over the {MAX_WINDOW_CELLS} cap"));
    }
    Ok(())
}

/// Refuses more than [`MAX_TILE_DRAWS`] cells, so the world falls back to the empty map
/// instead of failing the renderer.
fn within_draw_cap(draws: usize) -> Result<(), String> {
    if draws > MAX_TILE_DRAWS {
        return Err(format!("{draws} cells over the {MAX_TILE_DRAWS} cap"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "world/tests.rs"]
mod tests;
