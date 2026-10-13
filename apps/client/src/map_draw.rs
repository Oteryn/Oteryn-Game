//! Server-composed map stacks to the existing bounded sprite-page pipeline.
//! Reads only pinned appearances/sheets, never local map placements or bundle definitions.
use crate::map_view::MapView;
use crate::world::{MANIFEST_PATH, MANIFEST_SHA256, SCAN_MARGIN, item_height, stack_lifts};
use oteryn_client_assets::{
    AppearanceIndex, AssetStore, Catalog, Hook, MAX_ENTRY_CELLS, Placement, ResolvedEntry,
    SpriteSheets,
};
use oteryn_renderer::{
    AtlasPage, BatchError, CellKey, PlacedCell, SpriteFrame, TileCoord, TileView,
};
use oteryn_session::{
    ActorPosition, MAX_SNAPSHOT_ENTITIES,
    map_view::{MapTile, in_window, tile_order},
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Visual data already resolved by the actor consumer, not a map definition reference.
#[derive(Debug, Clone)]
pub struct MapActor {
    pub position: ActorPosition,
    pub previous: Option<ActorPosition>,
    /// Source pixels relative to the current tile; supplied by the movement presentation.
    pub offset: [i32; 2],
    pub cells: Vec<PlacedCell>,
}

pub struct MapDraw {
    assets: Option<(AppearanceIndex, SpriteSheets)>,
}

impl std::fmt::Debug for MapDraw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MapDraw")
            .field("assets_loaded", &self.assets.is_some())
            .finish()
    }
}

impl MapDraw {
    #[must_use]
    pub const fn placeholders() -> Self {
        Self { assets: None }
    }

    pub fn load(root: &Path) -> Result<Self, String> {
        let store = AssetStore::open_pinned(
            &root.join("content/assets/files"),
            &root.join(MANIFEST_PATH),
            MANIFEST_SHA256,
        )
        .map_err(|e| format!("assets: {e}"))?;
        let catalog = Catalog::load(&store).map_err(|e| format!("catalog: {e}"))?;
        let index =
            AppearanceIndex::load(&store, &catalog).map_err(|e| format!("appearances: {e}"))?;
        Ok(Self {
            assets: Some((index, SpriteSheets::new(store, catalog))),
        })
    }

    pub fn frame(
        &self,
        map: &MapView,
        actors: &[MapActor],
        page: &mut AtlasPage,
        view: &TileView,
    ) -> Result<SpriteFrame, BatchError> {
        compose(map, actors, page, view, |tile| {
            let Some((index, sheets)) = &self.assets else {
                return vec![None; tile.items.len()];
            };
            let mut hook = Hook::None;
            for item in &tile.items {
                if let Some(appearance) = index.get(u32::from(item.appearance_id))
                    && hook != Hook::South
                    && appearance.hook != Hook::None
                {
                    hook = appearance.hook;
                }
            }
            tile.items
                .iter()
                .map(|item| {
                    index
                        .resolve(
                            sheets,
                            u32::from(item.appearance_id),
                            Placement {
                                count: item.count,
                                sub_type: item.sub_type,
                                x: tile.position.x,
                                y: tile.position.y,
                                floor: i32::from(tile.position.floor),
                                hook,
                            },
                        )
                        .ok()
                })
                .collect()
        })
    }

    /// Callers must overwrite a reused slot with their placeholder pixels on error,
    /// never retain the previous cell's texture contents.
    pub fn cell_rgba(&mut self, key: CellKey) -> Result<Vec<u8>, String> {
        // Sprite zero is reserved here for our owned UI glyphs, never a map appearance.
        if key.sprite_id == 0 && key.cell_y == 0 && key.cell_x < 3 {
            return Ok(crate::world::builtin_rgba()
                .chunks_exact(4096)
                .nth(usize::from(key.cell_x))
                .ok_or("invalid builtin cell")?
                .to_vec());
        }
        let Some((_, sheets)) = &mut self.assets else {
            return Err("sprite assets unavailable".into());
        };
        sheets
            .cell_rgba(key.sprite_id, u32::from(key.cell_x), u32::from(key.cell_y))
            .map_err(|e| e.to_string())
    }
}

pub(crate) fn projected(position: ActorPosition, origin: ActorPosition) -> Option<TileCoord> {
    let shift = i32::from(position.floor) - i32::from(origin.floor);
    Some(TileCoord::new(
        position.x.checked_sub(shift)?,
        position.y.checked_sub(shift)?,
    ))
}

fn walking_source(actor: &MapActor) -> Option<ActorPosition> {
    actor.previous.filter(|previous| {
        previous.floor == actor.position.floor
            && previous
                .x
                .abs_diff(actor.position.x)
                .max(previous.y.abs_diff(actor.position.y))
                <= 1
    })
}

fn actor_phase(actor: &MapActor) -> ActorPosition {
    if let Some(previous) = walking_source(actor)
        && tile_order(&previous) > tile_order(&actor.position)
    {
        previous
    } else {
        actor.position
    }
}

fn layer(entry: Option<&ResolvedEntry>) -> u8 {
    match entry {
        Some(e) if e.ground => 0,
        Some(e) if e.ground_border => 1,
        Some(e) if e.on_bottom => 2,
        Some(e) if e.on_top => 4,
        _ => 3,
    }
}

fn compose(
    map: &MapView,
    actors: &[MapActor],
    page: &mut AtlasPage,
    view: &TileView,
    mut resolve: impl FnMut(&MapTile) -> Vec<Option<ResolvedEntry>>,
) -> Result<SpriteFrame, BatchError> {
    if actors.len() > MAX_SNAPSHOT_ENTITIES {
        return Err(BatchError::CapacityExceeded {
            max: MAX_SNAPSHOT_ENTITIES,
        });
    }
    let mut frame = SpriteFrame::new(MAX_ENTRY_CELLS);
    frame.begin(page);
    let Some(header) = map.header() else {
        return Ok(frame);
    };
    // The wire view includes neighbour anchors whose up-left cells enter the viewport.
    // Keep the actual screen origin/scale; the GPU clips the extra right/bottom margin.
    let expanded = TileView::new(
        view.origin(),
        view.tile_px(),
        view.columns()
            .checked_add(SCAN_MARGIN as u32)
            .ok_or(BatchError::InvalidView)?,
        view.rows()
            .checked_add(SCAN_MARGIN as u32)
            .ok_or(BatchError::InvalidView)?,
    )?;
    let mut positions: BTreeSet<_> = map
        .visible_tiles()
        .map(|t| tile_order(&t.position))
        .collect();
    for actor in actors {
        let position = actor_phase(actor);
        if position.floor <= header.first_visible_floor && in_window(header.origin, position) {
            positions.insert(tile_order(&position));
        }
    }
    // A walking actor belongs to the later creature phase, but must remain under
    // BOTH endpoints' on-top items. Only defer affected tops, never unrelated tiles.
    let mut top_phases: BTreeMap<_, (i16, i32, i32)> = BTreeMap::new();
    for actor in actors {
        let Some(previous) = walking_source(actor) else {
            continue;
        };
        let phase = actor_phase(actor);
        let last = tile_order(&phase);
        if !positions.contains(&last)
            || projected(phase, header.origin).is_none_or(|tile| !expanded.contains(tile))
        {
            continue;
        }
        let first = tile_order(&previous).min(tile_order(&actor.position));
        if first < last {
            top_phases
                .entry(first)
                .and_modify(|phase: &mut _| *phase = (*phase).max(last))
                .or_insert(last);
        }
    }
    let mut on_top: BTreeMap<_, Vec<(TileCoord, Option<ResolvedEntry>, i32)>> = BTreeMap::new();
    for (floor, y, x) in positions {
        let position = ActorPosition { x, y, floor };
        let Some(tile) = projected(position, header.origin) else {
            continue;
        };
        if !expanded.contains(tile) {
            continue;
        }
        let mut items = map.tile(position).map(&mut resolve).unwrap_or_default();
        // Stable phase ordering retains the received stack order inside each phase.
        items.sort_by_key(|entry| layer(entry.as_ref()));
        let (lifts, elevation) = stack_lifts(items.iter().map(|entry| {
            (
                layer(entry.as_ref()),
                entry.as_ref().map_or(0, |e| item_height(e.elevation)),
            )
        }));
        let push = |frame: &mut SpriteFrame,
                    page: &mut AtlasPage,
                    tile: TileCoord,
                    entry: Option<&ResolvedEntry>,
                    lift: i32| {
            let Some(entry) = entry else {
                frame.push_placeholder(page, &expanded, tile, [-lift; 2]);
                return;
            };
            let cells: Vec<_> = entry
                .cells
                .iter()
                .map(|cell| PlacedCell {
                    key: CellKey {
                        sprite_id: cell.sprite_id,
                        cell_x: cell.cell_x as u16,
                        cell_y: cell.cell_y as u16,
                    },
                    offset_x: cell.offset_x - lift,
                    offset_y: cell.offset_y - lift,
                })
                .collect();
            frame.push_entry(page, &expanded, tile, &cells);
        };
        for (entry, &lift) in items
            .iter()
            .zip(&lifts)
            .filter(|(entry, _)| layer(entry.as_ref()) != 4)
        {
            push(&mut frame, page, tile, entry.as_ref(), lift);
        }
        for actor in actors.iter().filter(|actor| actor_phase(actor) == position) {
            let dx = actor.position.x - position.x;
            let dy = actor.position.y - position.y;
            if actor.cells.is_empty() {
                frame.push_placeholder(
                    page,
                    &expanded,
                    tile,
                    [
                        (dx * 32)
                            .saturating_add(actor.offset[0])
                            .saturating_sub(elevation),
                        (dy * 32)
                            .saturating_add(actor.offset[1])
                            .saturating_sub(elevation),
                    ],
                );
                continue;
            }
            if actor.cells.len() > MAX_ENTRY_CELLS {
                frame.push_entry(page, &expanded, tile, &actor.cells);
                continue;
            }
            let cells: Vec<_> = actor
                .cells
                .iter()
                .map(|cell| PlacedCell {
                    key: cell.key,
                    offset_x: cell
                        .offset_x
                        .saturating_add(dx * 32)
                        .saturating_add(actor.offset[0])
                        .saturating_sub(elevation),
                    offset_y: cell
                        .offset_y
                        .saturating_add(dy * 32)
                        .saturating_add(actor.offset[1])
                        .saturating_sub(elevation),
                })
                .collect();
            frame.push_entry(page, &expanded, tile, &cells);
        }
        let key = tile_order(&position);
        let release = top_phases.get(&key).copied().unwrap_or(key);
        for (entry, lift) in items.into_iter().zip(lifts) {
            if layer(entry.as_ref()) == 4 {
                on_top
                    .entry(release)
                    .or_insert_with(Vec::new)
                    .push((tile, entry, lift));
            }
        }
        if let Some(entries) = on_top.remove(&key) {
            for (anchor, entry, lift) in entries {
                push(&mut frame, page, anchor, entry.as_ref(), lift);
            }
        }
    }
    Ok(frame)
}

#[cfg(test)]
mod tests;
