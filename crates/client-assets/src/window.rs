//! Bounded, pinned map-window and appearance resolution, independent of rendering.

use crate::world::{
    Budget, MANIFEST_PATH, MANIFEST_SHA256, MAX_REGION_BYTES, REGION_TILES, SECTOR_TILES,
    decode_region, palette_entry, read_capped, read_placement_index,
};
use crate::{AppearanceIndex, AssetStore, Catalog, DrawCell, Hook, Placement, SpriteSheets};
use sha2::{Digest, Sha256};
use std::path::Path;

/// One resolved map item: draw layer, raw appearance elevation and cells.
pub type Item = (u8, u32, Vec<DrawCell>);

/// Tiles decoded on each side of the map center.
pub const RADIUS: i32 = 64;
/// The default outfit (citizen).
pub const PLAYER_LOOK_TYPE: u32 = 128;
/// The most tiles in the sectors the bounded window can overlap.
#[allow(clippy::cast_sign_loss)]
const WINDOW_TILES: usize = (((2 * RADIUS + 1) / SECTOR_TILES + 2) * SECTOR_TILES).pow(2) as usize;
/// The most cells one tile, or the player, may draw, so a full view stays inside one sprite batch.
pub const MAX_TILE_DRAWS: usize = 256;
/// The most resolved cells the whole window may hold before the atlas is built.
const MAX_WINDOW_CELLS: usize = 1024 * 1024;

/// Resolved immutable placements and the pinned sheets from which their pixels are read.
pub struct ResolvedWindow {
    pub entries: Vec<((i32, i32), Vec<Item>)>,
    /// Outfit patterns in north, east, south, west order.
    pub player: [Vec<DrawCell>; 4],
    pub sheets: SpriteSheets,
}

impl ResolvedWindow {
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
        let sheets = SpriteSheets::new(store, catalog);
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
                        items.push((layer, resolved.elevation, resolved.cells));
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

        Ok(Self {
            entries,
            player,
            sheets,
        })
    }
}

/// A start-floor placement; the encoded value is a stack count or, for fluids, the subtype.
fn placement(x: i32, y: i32, encoded: Option<u8>, floor: u8) -> Placement {
    let mut placement = Placement::at(x, y, -i32::from(floor));
    placement.count = u32::from(encoded.unwrap_or(1));
    placement.sub_type = u32::from(encoded.unwrap_or(0));
    placement
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
pub fn within_draw_cap(draws: usize) -> Result<(), String> {
    if draws > MAX_TILE_DRAWS {
        return Err(format!("{draws} cells over the {MAX_TILE_DRAWS} cap"));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
