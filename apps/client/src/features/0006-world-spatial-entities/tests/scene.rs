use super::*;

#[test]
fn actor_motion_world_outfit_renders_mount_and_walking_then_standing_frames()
-> Result<(), BatchError> {
    use oteryn_session::actor_motion::ActorMotion;
    fn pixels(scene: &Scene) -> Vec<u8> {
        let atlas = scene.atlas();
        let px = atlas.cell_px() as usize;
        let columns = atlas.width() as usize / px;
        scene.actor_draws[0]
            .draws
            .iter()
            .flat_map(|draw| {
                let index = usize::from(draw.cell);
                (0..px).flat_map(move |y| {
                    let at = (((index / columns) * px + y) * atlas.width() as usize
                        + index % columns * px)
                        * 4;
                    atlas.rgba()[at..at + px * 4].iter().copied()
                })
            })
            .collect()
    }
    let tile = TileCoord::new(100, 200);
    let mut scene =
        Scene::centered_on(Arc::new(World::builtin()?), None, tile, StepDir::South, &[])?;
    let now = std::time::Instant::now();
    scene.actor_animation(now, None);
    let mut entry = VisibleAppearance {
        forge_stacks: None,
        light: None,
        entity: oteryn_session::WorldSpatialEntity {
            motion: Some(ActorMotion::default()),
            display_name: None,
            kind: oteryn_session::EntityKind::Player,
            entity: oteryn_session::EntityRef {
                identity: [1; 16],
                generation: 1,
            },
            position: oteryn_session::ActorPosition {
                x: 100,
                y: 200,
                floor: 0,
            },
            detail: EntityDetail::Actor {
                direction: oteryn_session::StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        },
        look: Some(oteryn_session::Look::Outfit {
            look_type: 128,
            colours: [0; 4],
            addons: 0,
        }),
    };
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    scene.set_presentations_from(std::slice::from_ref(&entry), 0, Some(root))?;
    let standing = pixels(&scene);
    assert!(
        scene.actor_draws[0]
            .draws
            .iter()
            .any(|draw| draw.cell > MARKER_CELL)
    );
    entry.entity.motion = Some(ActorMotion {
        mount_look_type: 139,
        ..ActorMotion::default()
    });
    scene.set_presentations_from(std::slice::from_ref(&entry), 0, None)?;
    let mounted = pixels(&scene);
    assert_ne!(mounted, standing);
    entry.entity.position.x += 1;
    entry.entity.motion = Some(ActorMotion {
        translation_revision: 1,
        step_duration_ms: 200,
        ..ActorMotion::default()
    });
    scene.set_presentations_from(std::slice::from_ref(&entry), 0, None)?;
    let first = pixels(&scene);
    scene.actor_animation(now + std::time::Duration::from_millis(100), None);
    scene.set_presentations_from(std::slice::from_ref(&entry), 0, None)?;
    assert_eq!(
        scene.actor_offset(entry.entity.entity),
        [-(scene.view.tile_px() as f32) / 2., 0.],
    );
    let walking = pixels(&scene);
    assert_ne!(walking, first);
    scene.actor_animation(now + std::time::Duration::from_millis(200), None);
    scene.set_presentations_from(&[entry], 0, None)?;
    assert_eq!(pixels(&scene), standing);
    Ok(())
}
#[test]
fn cl6_forge_marks_reach_native_name_glyphs_and_clear_with_visibility() -> Result<(), BatchError> {
    let tile = TileCoord::new(100, 200);
    let mut scene =
        Scene::centered_on(Arc::new(World::builtin()?), None, tile, StepDir::South, &[])?;
    let mut entry = VisibleAppearance {
        entity: oteryn_session::WorldSpatialEntity {
            motion: None,
            entity: oteryn_session::EntityRef {
                identity: [8; 16],
                generation: 1,
            },
            kind: oteryn_session::EntityKind::Creature,
            position: oteryn_session::ActorPosition {
                x: 101,
                y: 200,
                floor: 0,
            },
            display_name: Some(
                oteryn_session::EntityDisplayName::new("Rat")
                    .map_err(|_| BatchError::InvalidView)?,
            ),
            detail: EntityDetail::Actor {
                direction: oteryn_session::StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        },
        look: None,
        light: None,
        forge_stacks: None,
    };
    for (mark, expected) in [
        (None, "Rat"),
        (Some(1), "[Influenced 1] Rat"),
        (Some(5), "[Influenced 5] Rat"),
        (Some(15), "[Fiendish] Rat"),
        (None, "Rat"),
    ] {
        entry.forge_stacks = mark;
        scene.set_presentations_from(std::slice::from_ref(&entry), 0, None)?;
        let glyphs = scene.cache.name(expected, 100);
        assert_eq!(
            scene.names,
            vec![(TileCoord::new(101, 200), glyphs)],
            "{expected}"
        );
    }
    entry.forge_stacks = Some(15);
    entry.entity.display_name = None;
    scene.set_presentations_from(std::slice::from_ref(&entry), 0, None)?;
    assert_eq!(
        scene.names[0].1,
        scene.cache.name("[Fiendish] Creature", 100)
    );
    entry.entity.kind = oteryn_session::EntityKind::Player;
    scene.set_presentations_from(std::slice::from_ref(&entry), 0, None)?;
    assert!(
        scene.names.is_empty(),
        "forge marks describe creatures only"
    );
    entry.entity.kind = oteryn_session::EntityKind::Creature;
    entry.entity.position.floor = 1;
    scene.set_presentations_from(&[entry], 0, None)?;
    assert!(scene.names.is_empty());
    scene.set_presentations_from(&[], 0, None)?;
    assert!(scene.names.is_empty());
    Ok(())
}

#[test]
fn live_outfit_uses_server_look_and_empty_outfit_suppresses_pixels() -> Result<(), BatchError> {
    let tile = TileCoord::new(100, 200);
    let mut scene =
        Scene::centered_on(Arc::new(World::builtin()?), None, tile, StepDir::South, &[])?;
    let mut entry = VisibleAppearance {
        forge_stacks: None,
        light: None,
        entity: oteryn_session::WorldSpatialEntity {
            motion: None,
            display_name: Some(
                oteryn_session::EntityDisplayName::new("Citizen")
                    .map_err(|_| BatchError::InvalidView)?,
            ),
            kind: oteryn_session::EntityKind::Player,
            entity: oteryn_session::EntityRef {
                identity: [1; 16],
                generation: 1,
            },
            position: oteryn_session::ActorPosition {
                x: 100,
                y: 200,
                floor: 0,
            },
            detail: EntityDetail::Actor {
                direction: oteryn_session::StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        },
        look: Some(oteryn_session::Look::Outfit {
            look_type: 128,
            colours: [0; 4],
            addons: 0,
        }),
    };
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    scene.set_outfit_palette(&[[255, 0, 0], [0, 0, 255]]);
    scene.set_presentations_from(&[entry.clone()], 0, Some(root))?;
    assert!(
        scene.actor_draws[0]
            .draws
            .iter()
            .any(|draw| draw.cell > MARKER_CELL)
    );
    assert_eq!(scene.name_count(), 1);
    let base_cell = scene.actor_draws[0].draws[0].cell;
    entry.look = Some(oteryn_session::Look::Outfit {
        look_type: 128,
        colours: [1; 4],
        addons: 0,
    });
    scene.set_presentations_from(&[entry.clone()], 0, None)?;
    assert_ne!(scene.actor_draws[0].draws[0].cell, base_cell);
    let base_count = scene.actor_draws[0].draws.len();
    entry.look = Some(oteryn_session::Look::Outfit {
        look_type: 128,
        colours: [1; 4],
        addons: 3,
    });
    scene.set_presentations_from(&[entry.clone()], 0, None)?;
    assert!(scene.actor_draws[0].draws.len() > base_count);

    entry.look = Some(oteryn_session::Look::EmptyOutfit);
    scene.set_presentations_from(&[entry.clone()], 0, None)?;
    assert!(scene.actor_draws[0].draws.is_empty());
    entry.entity.position.floor = 1;
    scene.set_presentations_from(&[entry], 0, None)?;
    assert_eq!(scene.name_count(), 0);
    Ok(())
}
#[test]
fn other_actor_keeps_object_marker_and_local_player() -> Result<(), BatchError> {
    let tile = TileCoord::new(100, 200);
    let mut scene = Scene::centered_on(
        Arc::new(World::builtin()?),
        None,
        tile,
        StepDir::South,
        &[tile],
    )?;
    let entity = oteryn_session::WorldSpatialEntity {
        motion: None,
        display_name: None,
        kind: oteryn_session::EntityKind::Creature,
        entity: oteryn_session::EntityRef {
            identity: [8; 16],
            generation: 1,
        },
        position: oteryn_session::ActorPosition {
            x: 100,
            y: 200,
            floor: 0,
        },
        detail: EntityDetail::Actor {
            direction: oteryn_session::StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let mut appearance = VisibleAppearance {
        forge_stacks: None,
        light: None,
        entity,
        look: Some(oteryn_session::Look::EmptyOutfit),
    };
    scene.set_presentations_from(std::slice::from_ref(&appearance), 0, None)?;
    assert_eq!(
        scene.sprites().len(),
        2,
        "object marker and own actor must both survive"
    );
    appearance.look = None;
    scene.set_presentations_from(&[appearance], 0, None)?;
    assert_eq!(
        scene.sprites().len(),
        3,
        "object marker, other actor and local player must all survive"
    );
    Ok(())
}

#[test]
fn recenter_reuses_loaded_assets_outfits_glyphs_and_packed_atlas() -> Result<(), BatchError> {
    let world = Arc::new(World::builtin()?);
    let tile = TileCoord::new(100, 200);
    let mut scene = Scene::centered_on(world.clone(), None, tile, StepDir::South, &[])?;
    scene.set_own_identity([1; 16]);
    let entry = VisibleAppearance {
        forge_stacks: None,
        light: None,
        entity: oteryn_session::WorldSpatialEntity {
            motion: None,
            display_name: Some(
                oteryn_session::EntityDisplayName::new("Citizen")
                    .map_err(|_| BatchError::InvalidView)?,
            ),
            kind: oteryn_session::EntityKind::Player,
            entity: oteryn_session::EntityRef {
                identity: [1; 16],
                generation: 1,
            },
            position: oteryn_session::ActorPosition {
                x: 100,
                y: 200,
                floor: 0,
            },
            detail: EntityDetail::Actor {
                direction: oteryn_session::StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        },
        look: Some(oteryn_session::Look::Outfit {
            look_type: 128,
            colours: [0; 4],
            addons: 0,
        }),
    };
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    scene.set_presentations_from(std::slice::from_ref(&entry), 0, Some(root.clone()))?;
    assert_eq!(scene.cache.counts(), [1, 1, 1]);
    assert!(
        scene
            .own_draws
            .as_ref()
            .is_some_and(|draws| draws.iter().any(|draw| draw.cell > MARKER_CELL))
    );
    let atlas = scene
        .presentation_atlas
        .clone()
        .ok_or(BatchError::InvalidAtlas)?;
    scene.recenter_with(
        world,
        None,
        TileCoord::new(101, 200),
        StepDir::South,
        &[],
        &[],
    )?;
    scene.set_presentations_from(std::slice::from_ref(&entry), 0, Some(root.clone()))?;
    assert_eq!(
        scene.cache.counts(),
        [1, 1, 1],
        "rebuild must not reload, unpack or repack"
    );
    assert!(
        scene
            .presentation_atlas
            .as_ref()
            .is_some_and(|cached| Arc::ptr_eq(&atlas, cached))
    );
    assert!(
        scene.actor_draws.is_empty(),
        "the own identity is drawn only in the player slot"
    );
    assert_eq!(scene.names[0].0, TileCoord::new(101, 200));
    // A new map atlas needs new slots but uses the already loaded appearance index/sheets.
    scene.recenter_with(
        Arc::new(World::builtin()?),
        None,
        tile,
        StepDir::South,
        &[],
        &[],
    )?;
    scene.set_presentations_from(&[entry], 0, Some(root))?;
    assert_eq!(scene.cache.counts(), [1, 2, 2]);
    Ok(())
}

#[test]
fn play_view_rebuild_retains_the_same_cached_image() -> Result<(), BatchError> {
    let mut view = crate::play::PlayView::new(TileCoord::new(100, 200), 0, &[])?;
    view.set_own_identity([1; 16]);
    let entry = VisibleAppearance {
        forge_stacks: None,
        light: None,
        entity: oteryn_session::WorldSpatialEntity {
            motion: None,
            display_name: Some(
                oteryn_session::EntityDisplayName::new("Player")
                    .map_err(|_| BatchError::InvalidView)?,
            ),
            kind: oteryn_session::EntityKind::Player,
            entity: oteryn_session::EntityRef {
                identity: [1; 16],
                generation: 1,
            },
            position: oteryn_session::ActorPosition {
                x: 100,
                y: 200,
                floor: 0,
            },
            detail: EntityDetail::Actor {
                direction: oteryn_session::StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        },
        look: None,
    };
    view.set_presentations(vec![entry.clone()])?;
    let image = view
        .scene()
        .presentation_atlas
        .clone()
        .ok_or(BatchError::InvalidAtlas)?;
    let counts = view.scene().cache.counts();
    view.set_presentations(vec![entry])?;
    assert_eq!(view.scene().cache.counts(), counts);
    assert!(
        view.scene()
            .presentation_atlas
            .as_ref()
            .is_some_and(|cached| Arc::ptr_eq(&image, cached))
    );
    Ok(())
}

#[test]
fn nx8_influenced_and_fiendish_markers_follow_creature_presentation() -> Result<(), BatchError> {
    let tile = TileCoord::new(100, 200);
    let mut scene =
        Scene::centered_on(Arc::new(World::builtin()?), None, tile, StepDir::South, &[])?;
    let mut entry = VisibleAppearance {
        light: None,
        forge_stacks: Some(3),
        entity: oteryn_session::WorldSpatialEntity {
            motion: None,
            display_name: Some(
                oteryn_session::EntityDisplayName::new("Rat")
                    .map_err(|_| BatchError::InvalidView)?,
            ),
            kind: oteryn_session::EntityKind::Creature,
            entity: oteryn_session::EntityRef {
                identity: [9; 16],
                generation: 1,
            },
            position: oteryn_session::ActorPosition {
                x: 100,
                y: 200,
                floor: 0,
            },
            detail: EntityDetail::Actor {
                direction: oteryn_session::StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        },
        look: None,
    };
    scene.set_presentations_from(std::slice::from_ref(&entry), 0, None)?;
    assert_eq!(
        scene.names[0].1,
        scene.cache.name("[Influenced 3] Rat", 100)
    );
    entry.forge_stacks = Some(15);
    scene.set_presentations_from(std::slice::from_ref(&entry), 0, None)?;
    assert_eq!(scene.names[0].1, scene.cache.name("[Fiendish] Rat", 100));
    // Capability 104 is independently negotiable from capability 286 display names.
    let name = entry.entity.display_name.take();
    scene.set_presentations_from(std::slice::from_ref(&entry), 0, None)?;
    assert_eq!(
        scene.names[0].1,
        scene.cache.name("[Fiendish] Creature", 100)
    );
    entry.forge_stacks = None;
    scene.set_presentations_from(std::slice::from_ref(&entry), 0, None)?;
    assert_eq!(
        scene.name_count(),
        0,
        "ordinary nameless creatures gain no label"
    );
    entry.entity.display_name = name;
    entry.forge_stacks = None;
    scene.set_presentations_from(std::slice::from_ref(&entry), 0, None)?;
    assert_eq!(scene.names[0].1, scene.cache.name("Rat", 100));
    entry.forge_stacks = Some(15);
    entry.entity.kind = oteryn_session::EntityKind::Player;
    scene.set_presentations_from(&[entry], 0, None)?;
    assert_eq!(
        scene.names[0].1,
        scene.cache.name("Rat", 100),
        "forge labels apply only to creatures"
    );
    scene.set_presentations_from(&[], 0, None)?;
    assert_eq!(scene.name_count(), 0);
    Ok(())
}

#[cfg(bevy_shell)]
#[test]
fn item_icons_reuse_loaded_assets_and_packed_cells() -> Result<(), Box<dyn std::error::Error>> {
    let mut scene = Scene::centered_on(
        Arc::new(World::builtin()?),
        None,
        TileCoord::new(100, 100),
        StepDir::South,
        &[],
    )?;
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    scene.prime_visual_assets(root);
    let item = oteryn_session::ItemEntry {
        appearance_id: 3031,
        handle: 7.try_into()?,
        item_definition_ref: 17.try_into()?,
        count: 3.try_into()?,
        sub_type: 0,
    };
    let before = scene.cache.counts();
    scene.prepare_item_icons(std::iter::once(&item))?;
    assert!(scene.item_icon(&item).is_some());
    let prepared = scene.cache.counts();
    assert_eq!(prepared[0], before[0], "no asset reload");
    assert_eq!(prepared[1], before[1], "no base atlas unpack");
    scene.prepare_item_icons(std::iter::once(&item))?;
    assert_eq!(
        scene.cache.counts(),
        prepared,
        "no repack for the same item"
    );
    Ok(())
}

#[test]
fn srx3_soul_war_callback_outfits_decode_server_bindings_and_draw_native_sprites()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::appearance as wire;
    let tile = TileCoord::new(100, 200);
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut scene =
        Scene::centered_on(Arc::new(World::builtin()?), None, tile, StepDir::South, &[])?;
    let mut snapshot = wire::Snapshot::decode(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/protocol-oteryn/tests/fixtures/appearance_snapshot_v1.bin"
    )))
    .map_err(|error| format!("{error:?}"))?;
    for look_type in [242, 286] {
        snapshot.bindings[0].look = wire::Look::Outfit {
            look_type,
            colours: [0; 4],
            addons: 0,
        };
        let encoded = snapshot.encode().map_err(|error| format!("{error:?}"))?;
        let decoded = wire::Snapshot::decode(&encoded).map_err(|error| format!("{error:?}"))?;
        let binding = &decoded.bindings[0];
        let entry = VisibleAppearance {
            forge_stacks: None,
            light: None,
            entity: oteryn_session::WorldSpatialEntity {
                motion: None,
                display_name: None,
                kind: oteryn_session::EntityKind::Player,
                entity: binding.entity,
                position: oteryn_session::ActorPosition {
                    x: 100,
                    y: 200,
                    floor: 0,
                },
                detail: EntityDetail::Actor {
                    direction: oteryn_session::StepDirection::South,
                    appearance_ref: binding.appearance_ref,
                    health_percent: 100,
                },
            },
            look: Some(binding.look.clone()),
        };
        scene.set_presentations_from(&[entry], 0, Some(root.clone()))?;
        assert!(
            scene.actor_draws[0]
                .draws
                .iter()
                .any(|draw| draw.cell > MARKER_CELL),
            "source outfit {look_type} must use actual native sprite cells"
        );
    }
    Ok(())
}

#[path = "native_icons.rs"]
mod native_icons;

#[test]
fn srx3_native_object_pixels_use_explicit_resource_not_opaque_definition_reference()
-> Result<(), BatchError> {
    let tile = TileCoord::new(100, 200);
    let at = oteryn_session::ActorPosition {
        x: 100,
        y: 200,
        floor: 0,
    };
    let mut scene =
        Scene::centered_on(Arc::new(World::builtin()?), None, tile, StepDir::South, &[])?;
    let mut entry = VisibleAppearance {
        forge_stacks: None,
        light: None,
        look: None,
        entity: oteryn_session::WorldSpatialEntity {
            motion: None,
            display_name: None,
            kind: oteryn_session::EntityKind::Corpse,
            entity: oteryn_session::EntityRef {
                identity: [9; 16],
                generation: 0,
            },
            position: at,
            detail: EntityDetail::ObjectWithAppearance {
                item_definition_ref: 42,
                quantity: 1,
                item_handle: std::num::NonZeroU64::new(2),
                appearance_id: std::num::NonZeroU16::new(39529).ok_or(BatchError::InvalidView)?,
            },
        },
    };
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    scene.set_presentations_from(std::slice::from_ref(&entry), 0, Some(root.clone()))?;
    assert!(
        scene.native_item_rendered_at(at, 42),
        "actual pinned39529 cells must draw the opaque Item reference42"
    );
    assert!(
        scene.object_draws[0]
            .2
            .iter()
            .all(|draw| draw.cell > MARKER_CELL)
    );
    let origin = oteryn_session::ActorPosition { floor: -6, ..at };
    entry.entity.position = oteryn_session::ActorPosition {
        x: at.x - 1,
        y: at.y - 1,
        floor: -7,
    };
    let world = World::from_live_map(&oteryn_session::world_map::WorldMapViewSnapshot {
        header: oteryn_session::world_map::MapViewHeader {
            origin,
            first_visible_floor: -6,
            content_generation: [0; 32],
            bundle_digest: [0; 32],
            reset_epoch: 1,
        },
        tiles: vec![oteryn_session::world_map::MapTile {
            position: entry.entity.position,
            ground_speed: 100,
            more: false,
            items: vec![],
        }],
    })?;
    let mut layered = Scene::centered_on(
        Arc::new(world),
        Some(TileCoord::new(0, 0)),
        tile,
        StepDir::South,
        &[],
    )?;
    layered.set_presentations_from(std::slice::from_ref(&entry), -6, Some(root.clone()))?;
    let object = layered
        .object_draws
        .first()
        .ok_or(BatchError::InvalidView)?;
    let uv = layered.atlas().uv_rect(object.2[0].cell)?;
    assert_eq!(
        layered
            .sprites()
            .instances()
            .iter()
            .filter(|quad| quad.uv == uv)
            .count(),
        1
    );
    let (batch, _, ranges, _) = layered.world_sprites()?;
    let (range, _) = ranges
        .iter()
        .find(|(_, floor)| *floor == -7)
        .ok_or(BatchError::InvalidView)?;
    let draw = object.2[0];
    let scale = layered.view().tile_px() as f32 / oteryn_client_domain::world::CELL_PX as f32;
    let [x, y] = layered.view().tile_to_screen(tile);
    assert!(
        batch.instances()[range.clone()]
            .iter()
            .any(|quad| quad.uv == uv
                && quad.position
                    == [
                        x + draw.offset[0] as f32 * scale,
                        y + draw.offset[1] as f32 * scale
                    ])
    );

    #[cfg(bevy_shell)]
    {
        // The player remains below the roof as its terrain leaves the visible-floor cutoff.
        entry.entity.position = oteryn_session::ActorPosition {
            x: at.x + 1,
            y: at.y + 1,
            floor: -6,
        };
        let below = oteryn_session::ActorPosition { floor: -7, ..at };
        let mut map = oteryn_session::world_map::WorldMapViewSnapshot {
            header: oteryn_session::world_map::MapViewHeader {
                origin: below,
                first_visible_floor: -6,
                content_generation: [0; 32],
                bundle_digest: [0; 32],
                reset_epoch: 1,
            },
            tiles: vec![oteryn_session::world_map::MapTile {
                position: entry.entity.position,
                ground_speed: 100,
                more: false,
                items: vec![],
            }],
        };
        for cutoff in [-6, -7] {
            map.header.first_visible_floor = cutoff;
            let mut frame = Scene::centered_on(
                Arc::new(World::from_live_map(&map)?),
                Some(TileCoord::new(0, 0)),
                tile,
                StepDir::South,
                &[],
            )?;
            frame.set_presentations_from(std::slice::from_ref(&entry), -7, Some(root.clone()))?;
            let (range, _) = frame
                .floor_quad_ranges()
                .iter()
                .find(|(_, floor)| *floor == -6)
                .ok_or(BatchError::InvalidView)?;
            assert!(
                range.len() > 1,
                "corpse must be painted alongside the retained roof at cutoff {cutoff}"
            );
        }
    }
    entry.entity.position = at;
    entry.entity.detail = EntityDetail::Object {
        item_definition_ref: 39529,
        quantity: 1,
        item_handle: std::num::NonZeroU64::new(2),
    };
    scene.set_presentations_from(std::slice::from_ref(&entry), 0, None)?;
    assert!(
        scene.object_draws.is_empty(),
        "a definition ordinal must never be guessed as a native appearance"
    );
    assert!(!scene.native_item_rendered_at(at, 39529));
    Ok(())
}

#[test]
fn parity_a_health_updates_recolour_cached_name_glyphs() -> Result<(), BatchError> {
    let mut scene = Scene::centered_on(
        Arc::new(World::builtin()?),
        None,
        TileCoord::new(100, 200),
        StepDir::South,
        &[],
    )?;
    let mut entry = VisibleAppearance {
        entity: oteryn_session::WorldSpatialEntity {
            motion: None,
            entity: oteryn_session::EntityRef {
                identity: [8; 16],
                generation: 1,
            },
            kind: oteryn_session::EntityKind::Creature,
            position: oteryn_session::ActorPosition {
                x: 101,
                y: 200,
                floor: 0,
            },
            display_name: Some(
                oteryn_session::EntityDisplayName::new("Rat")
                    .map_err(|_| BatchError::InvalidView)?,
            ),
            detail: EntityDetail::Actor {
                direction: oteryn_session::StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        },
        look: None,
        light: None,
        forge_stacks: None,
    };
    scene.set_presentations_from(std::slice::from_ref(&entry), 0, None)?;
    let healthy = scene.names[0].1.clone();
    entry.entity.detail = EntityDetail::Actor {
        direction: oteryn_session::StepDirection::South,
        appearance_ref: 1,
        health_percent: 3,
    };
    scene.set_presentations_from(std::slice::from_ref(&entry), 0, None)?;
    assert_ne!(
        healthy, scene.names[0].1,
        "health must change the rendered name, not just its bar"
    );
    let critical = scene.names[0].1.clone();
    scene.set_presentations_from(std::slice::from_ref(&entry), 0, None)?;
    assert_eq!(
        critical, scene.names[0].1,
        "unchanged health reuses cached glyphs"
    );
    Ok(())
}

#[path = "tibia_font.rs"]
mod tibia_font;
