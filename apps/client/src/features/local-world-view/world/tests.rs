use super::*;
use oteryn_session::ActorPosition;
use oteryn_session::world_map::{MapDefinition, MapItem, MapOrigin, MapTile, MapViewHeader};
use std::num::NonZeroU32;

#[test]
fn review_live_map_skips_unrepresentable_projected_tiles() -> Result<(), BatchError> {
    let origin = ActorPosition {
        x: i32::MAX,
        y: i32::MAX,
        floor: 6,
    };
    for position in [
        ActorPosition {
            x: origin.x,
            y: origin.y - 1,
            floor: 7,
        },
        ActorPosition {
            x: origin.x - 1,
            y: origin.y,
            floor: 7,
        },
    ] {
        let snapshot = WorldMapViewSnapshot {
            header: MapViewHeader {
                origin,
                content_generation: [0; 32],
                bundle_digest: [0; 32],
                reset_epoch: 1,
                first_visible_floor: 6,
            },
            tiles: vec![MapTile {
                position,
                items: vec![MapItem {
                    definition: MapDefinition::Item(NonZeroU32::MIN),
                    count: 1,
                    sub_type: 0,
                    origin: MapOrigin::DisplayOnly,
                    appearance_id: 0,
                }],
                more: false,
                ground_speed: 100,
            }],
        };
        use oteryn_session::world_map::{decode_world_map_snapshot, encode_world_map_snapshot};
        let payload = encode_world_map_snapshot(&snapshot).map_err(|_| BatchError::InvalidView)?;
        let snapshot = decode_world_map_snapshot(&payload).map_err(|_| BatchError::InvalidView)?;
        let mut view = crate::play::PlayView::new(
            oteryn_renderer::TileCoord::new(origin.x - 32, origin.y - 32),
            origin.floor,
            &[],
        )?;
        view.set_live_map(&snapshot)?;
        let mut world = World::from_live_map(&snapshot)?;
        assert!(world.floor_tile(7, i32::MIN, i32::MIN).is_none());
        world.animations = std::sync::Arc::from([TileAnimation {
            position,
            elevation: 0,
            items: Vec::new(),
        }]);
        assert!(world.animated_at(std::time::Instant::now()).is_none());
    }
    Ok(())
}

#[test]
fn animation_durations_preserve_stack_order_and_transparent_phases() {
    let first = Draw {
        cell: 4,
        offset: [-8, -16],
    };
    let second = Draw {
        cell: 5,
        offset: [-8, -16],
    };
    let top = Draw {
        cell: 6,
        offset: [0, 0],
    };
    let animation = TileAnimation {
        position: ActorPosition {
            x: 100,
            y: 200,
            floor: 7,
        },
        elevation: 24,
        items: vec![
            ItemAnimation {
                over: false,
                frames: vec![(100, vec![first]), (300, vec![second])],
            },
            ItemAnimation {
                over: false,
                frames: vec![(100, vec![top])],
            },
            ItemAnimation {
                over: true,
                frames: vec![(100, vec![top]), (200, vec![])],
            },
        ],
    };
    let before = animation.sample(99);
    assert_eq!(before.under, [first, top]);
    assert_eq!(before.over, [top]);
    let boundary = animation.sample(100);
    assert_eq!(boundary.under, [second, top]);
    assert!(boundary.over.is_empty());
    assert_eq!(boundary.elevation, 24);
    assert_eq!(animation.sample(399).under, [second, top]);
    assert_eq!(animation.sample(400).under, [first, top]);
    assert_eq!(animation.sample(1200), animation.sample(0));
    assert_eq!(animation.sample(1_200_000_100), boundary);
}

#[test]
fn dense_visible_floors_keep_deepest_draws_within_one_batch() -> Result<(), BatchError> {
    use crate::scene::Scene;
    use oteryn_renderer::{MAX_BATCH_QUADS, TileCoord};
    use std::sync::Arc;
    let mut world = World::builtin()?;
    world.center = (100, 200, 8);
    for floor in 8..=10 {
        let shift = i32::from(8 - floor);
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
                                cell: if floor == 10 {
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
        floor: 7,
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
    assert_eq!(world.center(), (100, 200, 7));
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
        floor: 6,
    };
    map.tiles.push(upper);
    let layered = World::resolve_live_map(&root, &map)?;
    assert_eq!(layered.tile_count(), 2);
    let upper = layered
        .floor_tile(6, 100, 200)
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
