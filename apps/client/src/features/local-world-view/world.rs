//! Appearance projection of server tiles. Local assets supply pixels, never placements.
use super::*;
use oteryn_client_assets::world::{MANIFEST_PATH, MANIFEST_SHA256};
use oteryn_client_assets::{AppearanceIndex, AssetStore, Catalog, Hook, Placement, SpriteSheets};
use oteryn_session::world_map::WorldMapViewSnapshot;

struct RawItem {
    layer: u8,
    elevation: u32,
    frames: Vec<Vec<oteryn_client_assets::DrawCell>>,
    phase_ms: Vec<u32>,
}
#[derive(Debug, Clone)]
struct ItemAnimation {
    over: bool,
    frames: Vec<Vec<Draw>>,
    phase_ms: Vec<u32>,
}
#[derive(Debug, Clone)]
pub(super) struct TileAnimation {
    key: (i16, i32, i32),
    elevation: i32,
    items: Vec<ItemAnimation>,
}

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
        for tile in map
            .tiles
            .iter()
            .filter(|tile| in_draw_window(map, tile.position))
        {
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
                if let Ok(resolved) =
                    index.resolve(&sheets, u32::from(item.appearance_id), placement)
                {
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
                    let appearance = index
                        .get(u32::from(item.appearance_id))
                        .ok_or("appearance disappeared")?;
                    let mut frames = vec![resolved.cells];
                    for phase in 1..appearance.phases {
                        frames.push(
                            index
                                .resolve_phase(
                                    &sheets,
                                    u32::from(item.appearance_id),
                                    placement,
                                    phase,
                                )
                                .map_err(|error| error.to_string())?
                                .cells,
                        );
                    }
                    cells += frames.iter().map(Vec::len).max().unwrap_or(0);
                    within_draw_cap(cells)?;
                    items.push(RawItem {
                        layer,
                        elevation: resolved.elevation,
                        frames,
                        phase_ms: appearance.phase_ms.clone(),
                    });
                }
            }
            items.sort_by_key(|item| item.layer);
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
                .flat_map(|(_, items)| items.iter().flat_map(|item| item.frames.iter().flatten()))
                .chain(player.iter().flatten()),
            |sprite, x, y| {
                sheets
                    .cell_rgba(sprite, x, y)
                    .map_err(|error| error.to_string())
            },
        )?;
        let slot = |cell: &oteryn_client_assets::DrawCell| atlas.slot(cell);
        let mut animations = Vec::new();
        let tiles = entries
            .iter()
            .map(|(position, items)| {
                let first: Vec<_> = items
                    .iter()
                    .map(|item| (item.layer, item.elevation, item.frames[0].clone()))
                    .collect();
                let mut tile = project_tile(&first, slot);
                if items.iter().any(|item| item.frames.len() > 1) {
                    let (lifts, _) =
                        oteryn_client_simulation::world::stack_lifts(items.iter().map(|item| {
                            (
                                item.layer,
                                oteryn_client_simulation::world::item_height(item.elevation),
                            )
                        }));
                    let shift = i32::from(position.floor - origin.floor);
                    animations.push(TileAnimation {
                        key: (position.floor, position.x - shift, position.y - shift),
                        elevation: tile.elevation,
                        items: items
                            .iter()
                            .zip(lifts)
                            .map(|(item, lift)| ItemAnimation {
                                over: item.layer == 4,
                                frames: item
                                    .frames
                                    .iter()
                                    .map(|cells| {
                                        cells
                                            .iter()
                                            .filter_map(|cell| project_cell(cell, lift, slot))
                                            .collect()
                                    })
                                    .collect(),
                                phase_ms: item.phase_ms.clone(),
                            })
                            .collect(),
                    });
                }
                if tile.under.is_empty() && tile.over.is_empty() {
                    tile.under.push(Draw {
                        cell: MARKER_CELL,
                        offset: [0, 0],
                    });
                }
                (*position, tile)
            })
            .collect::<Vec<_>>();
        let player = player.map(|cells| {
            cells
                .iter()
                .filter_map(|cell| project_cell(cell, 0, slot))
                .collect()
        });
        let mut world = Self {
            animations,
            effects: Default::default(),
            center: (origin.x, origin.y, origin.floor),
            first_visible_floor: map.header.first_visible_floor,
            tiles: HashMap::new(),
            layers: BTreeMap::new(),
            player,
            atlas: atlas.into_image(),
        }
        .with_effect_catalog(effects);
        for (position, tile) in tiles {
            world.insert_floor_tile(position, tile);
        }
        Ok(world)
    }
}

/// Retain the bounded supplied geometry, including floors needed during a fade.
fn in_draw_window(map: &WorldMapViewSnapshot, position: oteryn_session::ActorPosition) -> bool {
    oteryn_session::world_map::floors_in_view(map.header.origin.floor)
        .is_some_and(|floors| floors.contains(&position.floor))
        && oteryn_session::world_map::in_window(map.header.origin, position)
}

impl World {
    fn insert_floor_tile(&mut self, position: oteryn_session::ActorPosition, tile: MapTile) {
        let shift = i32::from(position.floor - self.center.2);
        let projected = (position.x - shift, position.y - shift);
        if position.floor == self.center.2 {
            self.tiles.insert(projected, tile);
        } else {
            self.layers
                .entry(position.floor)
                .or_default()
                .insert(projected, tile);
        }
    }

    pub(crate) fn animation_epoch() -> std::time::Instant {
        static EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        *EPOCH.get_or_init(std::time::Instant::now)
    }

    pub(crate) fn animation_phases(&self, elapsed_ms: u64) -> Vec<usize> {
        self.animations
            .iter()
            .flat_map(|tile| &tile.items)
            .map(|item| {
                let total: u64 = (0..item.frames.len())
                    .map(|phase| u64::from(item.phase_ms.get(phase).copied().unwrap_or(100).max(1)))
                    .sum();
                let mut age = elapsed_ms % total;
                for phase in 0..item.frames.len() {
                    let duration =
                        u64::from(item.phase_ms.get(phase).copied().unwrap_or(100).max(1));
                    if age < duration {
                        return phase;
                    }
                    age -= duration;
                }
                0
            })
            .collect()
    }

    pub(crate) fn animation_tiles(&self, phases: &[usize]) -> HashMap<(i16, i32, i32), MapTile> {
        let mut phases = phases.iter();
        self.animations
            .iter()
            .map(|animation| {
                let mut tile = MapTile {
                    elevation: animation.elevation,
                    ..Default::default()
                };
                for item in &animation.items {
                    let phase = phases.next().copied().unwrap_or(0);
                    let draws = if item.over {
                        &mut tile.over
                    } else {
                        &mut tile.under
                    };
                    draws.extend(item.frames[phase].iter().copied());
                }
                (animation.key, tile)
            })
            .collect()
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
mod tests {
    use super::*;
    use oteryn_session::ActorPosition;
    use oteryn_session::world_map::{MapDefinition, MapItem, MapOrigin, MapTile, MapViewHeader};
    use std::num::NonZeroU32;

    #[test]
    fn dense_visible_floors_keep_deepest_draws_within_one_batch() -> Result<(), BatchError> {
        use crate::scene::Scene;
        use oteryn_renderer::{MAX_BATCH_QUADS, TileCoord};
        use std::sync::Arc;
        let mut world = World::builtin()?;
        world.center = (100, 200, -8);
        for floor in -10..=-8 {
            let shift = i32::from(floor + 8);
            for y in 195..=205 {
                for x in 93..=107 {
                    world.insert_floor_tile(
                        ActorPosition {
                            x: x + shift,
                            y: y + shift,
                            floor,
                        },
                        crate::map::MapTile {
                            under: vec![
                                Draw {
                                    cell: if floor == -10 {
                                        MARKER_CELL
                                    } else {
                                        TARGET_CELL
                                    },
                                    offset: [0, 0]
                                };
                                MAX_TILE_DRAWS
                            ],
                            ..crate::map::MapTile::default()
                        },
                    );
                }
            }
        }
        let mut scene = Scene::centered_on(
            Arc::new(world),
            Some(TileCoord::new(0, 0)),
            TileCoord::new(100, 200),
            StepDir::South,
            &[],
        )?;
        assert_eq!(scene.sprites().len(), MAX_BATCH_QUADS);
        let deep_uv = scene.atlas().uv_rect(MARKER_CELL)?;
        assert!(
            scene.sprites().instances()[..165 * MAX_TILE_DRAWS]
                .iter()
                .all(|quad| quad.uv == deep_uv)
        );
        scene.select_tile(TileCoord::new(100, 200))?;
        assert_eq!(scene.sprites().len(), MAX_BATCH_QUADS);
        assert_eq!(
            scene.sprites().instances().last().map(|quad| quad.uv),
            Some(scene.atlas().uv_rect(TARGET_CELL)?)
        );
        Ok(())
    }

    #[test]
    fn live_appearance_uses_native_floor_and_server_placements_only() -> Result<(), String> {
        let position = ActorPosition {
            x: 100,
            y: 200,
            floor: -7,
        };
        let mut map = WorldMapViewSnapshot {
            header: MapViewHeader {
                content_generation: [0; 32],
                bundle_digest: [0; 32],
                reset_epoch: 1,
                origin: position,
                first_visible_floor: 0,
            },
            tiles: vec![MapTile {
                position,
                items: vec![MapItem {
                    definition: MapDefinition::Item(NonZeroU32::MIN),
                    count: 1,
                    sub_type: 0,
                    origin: MapOrigin::BaseOrdinal {
                        ordinal: 0,
                        object_revision: 0,
                    },
                    appearance_id: 4526,
                }],
                more: false,
                ground_speed: 100,
            }],
        };
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let world = World::resolve_live_map(&root, &map)?;
        assert_eq!(world.center(), (100, 200, -7));
        assert_eq!(world.tile_count(), 1);
        let tile = world.tile(100, 200).ok_or("live tile missing")?;
        assert!(
            tile.under
                .iter()
                .any(|draw| draw.cell >= BUILTIN_CELLS as u16)
        );
        assert!(world.tile(START.0, START.1).is_none());
        let mut upper = map.tiles[0].clone();
        upper.position = ActorPosition {
            x: 101,
            y: 201,
            floor: -6,
        };
        map.tiles.push(upper);
        let layered = World::resolve_live_map(&root, &map)?;
        assert_eq!(layered.tile_count(), 2);
        let upper = layered
            .floor_tile(-6, 100, 200)
            .ok_or("upper floor missing")?;
        assert!(
            upper
                .under
                .iter()
                .any(|draw| draw.cell >= BUILTIN_CELLS as u16)
        );
        map.tiles.clear();
        let empty = World::resolve_live_map(&root, &map)?;
        assert_eq!(empty.tile_count(), 0);
        Ok(())
    }
}
