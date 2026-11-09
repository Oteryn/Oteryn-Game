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
        // A missing appearance remains visibly present without inventing local geometry.
        for tile in map.tiles.iter().filter(|tile| visible(map, tile.position)) {
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

    fn resolve_live_map(root: &Path, map: &WorldMapViewSnapshot) -> Result<Self, String> {
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
        for tile in map.tiles.iter().filter(|tile| visible(map, tile.position)) {
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
                    cells += resolved.cells.len();
                    within_draw_cap(cells)?;
                    items.push((layer, resolved.elevation, resolved.cells));
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
                .flat_map(|(_, items)| items.iter().flat_map(|(_, _, cells)| cells))
                .chain(player.iter().flatten()),
            |sprite, x, y| {
                sheets
                    .cell_rgba(sprite, x, y)
                    .map_err(|error| error.to_string())
            },
        )?;
        let slot = |cell: &oteryn_client_assets::DrawCell| atlas.slot(cell);
        let tiles = entries
            .iter()
            .map(|(position, items)| {
                let mut tile = project_tile(items, slot);
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
            effects: Default::default(),
            #[cfg(test)]
            native_resolved_items,
            center: (origin.x, origin.y, origin.floor),
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

/// Native floors increase upwards. The server's boundary hides everything above it.
fn visible(map: &WorldMapViewSnapshot, position: oteryn_session::ActorPosition) -> bool {
    position.floor <= map.header.first_visible_floor
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

    /// Painter order: the deepest floor first, the highest visible floor last.
    pub(crate) fn draw_floors(&self) -> Vec<i16> {
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
