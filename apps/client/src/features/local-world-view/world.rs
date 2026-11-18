//! Appearance projection of server tiles. Local assets supply pixels, never placements.
use super::*;
use oteryn_client_assets::world::{MANIFEST_PATH, MANIFEST_SHA256};
use oteryn_client_assets::{AppearanceIndex, AssetStore, Catalog, Hook, Placement, SpriteSheets};
use oteryn_session::world_map::WorldMapViewSnapshot;

impl World {
    pub(crate) fn from_live_map(map: &WorldMapViewSnapshot) -> Result<Self, BatchError> {
        let mut world = Self::builtin()?;
        let origin = map.header.origin;
        world.center = (origin.x, origin.y, origin.floor);
        world.first_visible_floor = map.header.first_visible_floor;
        // A missing appearance remains visibly present without inventing local geometry.
        for tile in map
            .tiles
            .iter()
            .filter(|tile| in_draw_window(map, tile.position))
        {
            world.insert_floor_tile(
                tile.position,
                MapTile {
                    under: vec![Draw {
                        cell: MARKER_CELL,
                        offset: [0, 0],
                    }],
                    ..MapTile::default()
                },
            );
        }
        if let Some(root) = std::env::var_os(ASSET_DIR_ENV)
            && let Ok(resolved) = Self::resolve_live_map(Path::new(&root), map)
        {
            return Ok(resolved);
        }
        Ok(world)
    }

    pub(crate) fn resolve_live_map(
        root: &Path,
        map: &WorldMapViewSnapshot,
    ) -> Result<Self, String> {
        let store = AssetStore::open_pinned(
            &root.join("content/assets/files"),
            &root.join(MANIFEST_PATH),
            MANIFEST_SHA256,
        )
        .map_err(|error| error.to_string())?;
        let effects = store.effects().clone();
        let catalog = Catalog::load(&store).map_err(|error| error.to_string())?;
        let index = AppearanceIndex::load(&store, &catalog).map_err(|error| error.to_string())?;
        let mut sheets = SpriteSheets::new(store, catalog);
        let origin = map.header.origin;
        let mut entries = Vec::new();
        #[cfg(test)]
        let mut native_resolved_items = Vec::new();
        let mut item_lights = Vec::new();
        let mut minimap_colors = BTreeMap::new();
        for tile in map
            .tiles
            .iter()
            .filter(|tile| in_draw_window(map, tile.position))
        {
            let mut color = (!tile.more && !tile.items.is_empty()).then_some(0);
            for (stack_index, item) in tile.items.iter().enumerate() {
                if let Some(appearance) = index.get(u32::from(item.appearance_id)) {
                    if let Some((level, color)) = appearance.light.filter(|(level, _)| *level > 0) {
                        item_lights.push((tile.position, level, color));
                    }
                    // A truncated stack can hide a coloured item above the ground.
                    if (!tile.more || stack_index != 0)
                        && let Some(value) = appearance.minimap_color.filter(|value| *value != 0)
                    {
                        color = Some(value);
                    }
                } else {
                    color = None;
                }
            }
            if let Some(color) = color {
                minimap_colors.insert(
                    (tile.position.floor, tile.position.x, tile.position.y),
                    color,
                );
            }
            let hook = tile
                .items
                .iter()
                .filter_map(|item| index.get(u32::from(item.appearance_id)))
                .fold(Hook::None, |hook, appearance| {
                    if hook == Hook::South || appearance.hook == Hook::None {
                        hook
                    } else {
                        appearance.hook
                    }
                });
            let mut items = Vec::new();
            let mut cells = 0;
            for item in &tile.items {
                let mut placement = Placement::at(
                    tile.position.x,
                    tile.position.y,
                    i32::from(tile.position.floor),
                );
                placement.count = item.count;
                placement.sub_type = item.sub_type;
                placement.hook = hook;
                if let Ok(frames) =
                    index.resolve_frames(&sheets, u32::from(item.appearance_id), placement)
                    && let Some(resolved) = frames.first()
                {
                    #[cfg(test)]
                    if !resolved.cells.is_empty()
                        && let oteryn_session::world_map::MapDefinition::Item(reference) =
                            item.definition
                    {
                        native_resolved_items.push((tile.position, reference.get()));
                    }
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
                    cells += frames
                        .iter()
                        .map(|frame| frame.cells.len())
                        .max()
                        .unwrap_or(0);
                    within_draw_cap(cells)?;
                    let durations = index
                        .get(u32::from(item.appearance_id))
                        .map(|appearance| appearance.phase_ms.as_slice())
                        .unwrap_or_default();
                    items.push((
                        layer,
                        resolved.elevation,
                        frames
                            .into_iter()
                            .enumerate()
                            .map(|(phase, frame)| {
                                (durations.get(phase).copied().unwrap_or(100), frame.cells)
                            })
                            .collect::<Vec<_>>(),
                    ));
                }
            }
            items.sort_by_key(|(layer, _, _)| *layer);
            entries.push((tile.position, items));
        }
        let mut player: [Vec<oteryn_client_assets::DrawCell>; 4] = Default::default();
        for (direction, cells) in (0..).zip(&mut player) {
            *cells = index
                .resolve_outfit(&sheets, PLAYER_LOOK_TYPE, direction)
                .map_err(|error| error.to_string())?
                .cells;
            within_draw_cap(cells.len())?;
        }
        let atlas = WorldAtlas::prepare(
            entries
                .iter()
                .flat_map(|(_, items)| {
                    items
                        .iter()
                        .flat_map(|(_, _, frames)| frames.iter().flat_map(|(_, cells)| cells))
                })
                .chain(player.iter().flatten()),
            |sprite, x, y| {
                sheets
                    .cell_rgba(sprite, x, y)
                    .map_err(|error| error.to_string())
            },
        )?;
        let slot = |cell: &oteryn_client_assets::DrawCell| atlas.slot(cell);
        let animations: Vec<_> = entries
            .iter()
            .map(|(position, items)| {
                use oteryn_client_simulation::world::{item_height, stack_lifts};
                let (lifts, elevation) = stack_lifts(
                    items
                        .iter()
                        .map(|(layer, height, _)| (*layer, item_height(*height))),
                );
                TileAnimation {
                    position: *position,
                    elevation,
                    items: items
                        .iter()
                        .zip(lifts)
                        .map(|((layer, _, frames), lift)| ItemAnimation {
                            over: *layer == 4,
                            frames: frames
                                .iter()
                                .map(|(duration, cells)| {
                                    (
                                        *duration,
                                        cells
                                            .iter()
                                            .filter_map(|cell| project_cell(cell, lift, slot))
                                            .collect(),
                                    )
                                })
                                .collect(),
                        })
                        .collect(),
                }
            })
            .collect();
        let tiles = animations
            .iter()
            .map(|animation| (animation.position, animation.sample(0)))
            .collect::<Vec<_>>();
        let player = player.map(|cells| {
            cells
                .iter()
                .filter_map(|cell| project_cell(cell, 0, slot))
                .collect()
        });
        let mut world = Self {
            effects: Default::default(),
            #[cfg(test)]
            native_resolved_items,
            item_lights,
            minimap_colors,
            center: (origin.x, origin.y, origin.floor),
            first_visible_floor: map.header.first_visible_floor,
            tiles: HashMap::new(),
            layers: BTreeMap::new(),
            player,
            atlas: std::sync::Arc::new(atlas.into_image()),
            animations: animations
                .into_iter()
                .filter(|tile| tile.items.iter().any(|item| item.frames.len() > 1))
                .collect(),
        }
        .with_effect_catalog(effects);
        for (position, tile) in tiles {
            world.insert_floor_tile(position, tile);
        }
        // Establish the shared clock before a caller captures its first sample time.
        // Initializing it during that sample can place the epoch after the supplied time,
        // leaving an exact full-cycle sample one millisecond short after truncation.
        if !world.animations.is_empty() {
            Self::animation_epoch(std::time::Instant::now());
        }
        Ok(world)
    }
}

// A shared monotonic epoch keeps terrain phases stable when server windows are replaced.
static ANIMATION_EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

#[derive(Debug, Clone)]
pub(super) struct TileAnimation {
    position: oteryn_session::ActorPosition,
    elevation: i32,
    items: Vec<ItemAnimation>,
}

#[derive(Debug, Clone)]
struct ItemAnimation {
    over: bool,
    frames: Vec<(u32, Vec<Draw>)>,
}

impl TileAnimation {
    fn sample(&self, elapsed_ms: u128) -> MapTile {
        let mut tile = MapTile {
            elevation: self.elevation,
            ..MapTile::default()
        };
        for item in &self.items {
            let period: u128 = item.frames.iter().map(|(ms, _)| u128::from(*ms)).sum();
            let mut phase_ms = elapsed_ms % period.max(1);
            for (duration, draws) in &item.frames {
                if phase_ms < u128::from(*duration) {
                    if item.over {
                        tile.over.extend_from_slice(draws);
                    } else {
                        tile.under.extend_from_slice(draws);
                    }
                    break;
                }
                phase_ms -= u128::from(*duration);
            }
        }
        if self.items.is_empty() {
            tile.under.push(Draw {
                cell: MARKER_CELL,
                offset: [0, 0],
            });
        }
        tile
    }
}

impl World {
    /// The first supplied sample fixes the shared epoch, without a later wall-clock read.
    pub(crate) fn animation_epoch(now: std::time::Instant) -> std::time::Instant {
        *ANIMATION_EPOCH.get_or_init(|| now)
    }

    /// Sample every item's own duration sequence, skipping elapsed frames after a stall.
    pub(crate) fn animated_at(&self, now: std::time::Instant) -> Option<Self> {
        if self.animations.is_empty() {
            return None;
        }
        let elapsed = now
            .saturating_duration_since(Self::animation_epoch(now))
            .as_millis();
        let mut next = None;
        for animation in self.animations.iter() {
            let position = animation.position;
            let Some((x, y)) = self.project_floor_tile(position) else {
                continue;
            };
            let tile = animation.sample(elapsed);
            if self.floor_tile(position.floor, x, y) != Some(&tile) {
                next.get_or_insert_with(|| self.clone())
                    .insert_floor_tile(position, tile);
            }
        }
        next
    }
}

/// Retain the bounded supplied geometry, including floors needed during a fade.
fn in_draw_window(map: &WorldMapViewSnapshot, position: oteryn_session::ActorPosition) -> bool {
    oteryn_session::world_map::floors_in_view(map.header.origin.floor)
        .is_some_and(|floors| floors.contains(&position.floor))
        && oteryn_session::world_map::in_window(map.header.origin, position)
}

impl World {
    fn project_floor_tile(&self, position: oteryn_session::ActorPosition) -> Option<(i32, i32)> {
        let shift = i32::from(position.floor) - i32::from(self.center.2);
        Some((
            position.x.checked_sub(shift)?,
            position.y.checked_sub(shift)?,
        ))
    }

    fn insert_floor_tile(&mut self, position: oteryn_session::ActorPosition, tile: MapTile) {
        let Some(projected) = self.project_floor_tile(position) else {
            return;
        };
        if position.floor == self.center.2 {
            self.tiles.insert(projected, tile);
        } else {
            self.layers
                .entry(position.floor)
                .or_default()
                .insert(projected, tile);
        }
    }

    /// Painter order: the deepest floor first, the highest visible floor last.
    pub(crate) fn draw_floors(&self) -> Vec<i16> {
        self.all_draw_floors()
            .into_iter()
            .filter(|floor| *floor <= self.first_visible_floor)
            .collect()
    }

    pub(crate) fn all_draw_floors(&self) -> Vec<i16> {
        let mut floors: Vec<_> = self.layers.keys().copied().chain([self.center.2]).collect();
        floors.sort_unstable();
        floors
    }

    pub(crate) fn floor_tile(&self, floor: i16, x: i32, y: i32) -> Option<&MapTile> {
        if floor == self.center.2 {
            self.tiles.get(&(x, y))
        } else {
            self.layers.get(&floor)?.get(&(x, y))
        }
    }
}

#[cfg(test)]
#[path = "world/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "world_light_tests.rs"]
mod light_tests;
