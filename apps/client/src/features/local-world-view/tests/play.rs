use super::*;

#[test]
fn the_map_is_drawn_only_on_the_join_floor() -> Result<(), String> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let world = Arc::new(World::load(&root)?);
    let mut view = view().map_err(|e| e.to_string())?;
    view.apply(&moved_to(crate::map::START.0, crate::map::START.1, 7))
        .map_err(|e| e.to_string())?;
    view.set_world(world).map_err(|e| e.to_string())?;
    let on_floor = view.scene().sprites().len();
    view.apply(&moved_to(100, 200, -1))
        .map_err(|e| e.to_string())?;
    let off_floor = view.scene().sprites().len();
    view.apply(&moved_to(crate::map::START.0, crate::map::START.1, 7))
        .map_err(|e| e.to_string())?;
    assert!(off_floor < on_floor / 10, "{off_floor} of {on_floor}");
    assert_eq!(view.scene().sprites().len(), on_floor);
    Ok(())
}

#[test]
fn pushed_domain_1_and_2_deltas_move_the_view() -> Result<(), BatchError> {
    let mut view = view()?;
    let mut pushed = Pushed::default();
    pushed.set_marker(entry(key(101, 200, 0, 0)));
    pushed.own = Some(ActorPosition {
        x: 101,
        y: 200,
        floor: 0,
    });
    view.apply_pushed(&pushed)?;
    assert_eq!(view.own(), TileCoord::new(101, 200));
    assert_eq!(view.marker_count(), 2);
    Ok(())
}

#[test]
fn live_map_push_reaches_the_rendered_scene() -> Result<(), BatchError> {
    let mut view = view()?;
    let before = view.scene().sprites().len();
    let map = live_snapshot(100);
    let mut pushed = Pushed::default();
    coalesce_pushed(&mut pushed, vec![map_event(map.header, map.tiles, vec![])]);
    view.apply_pushed(&pushed)?;
    assert!(view.scene().sprites().len() > before);
    Ok(())
}

pub(super) fn live_snapshot(x: i32) -> oteryn_session::world_map::WorldMapViewSnapshot {
    use oteryn_session::world_map::{MapTile, MapViewHeader, WorldMapViewSnapshot};
    let position = ActorPosition {
        x,
        y: 200,
        floor: 0,
    };
    WorldMapViewSnapshot {
        header: MapViewHeader {
            content_generation: [0; 32],
            bundle_digest: [0; 32],
            reset_epoch: 1,
            origin: position,
            first_visible_floor: 0,
        },
        tiles: vec![MapTile {
            position,
            items: vec![oteryn_session::world_map::MapItem {
                definition: oteryn_session::world_map::MapDefinition::Item(NonZeroU32::MIN),
                count: 1,
                sub_type: 0,
                origin: oteryn_session::world_map::MapOrigin::DisplayOnly,
                appearance_id: 0,
            }],
            more: false,
            ground_speed: 0,
        }],
    }
}

pub(super) fn map_event(
    header: oteryn_session::world_map::MapViewHeader,
    tiles: Vec<oteryn_session::world_map::MapTile>,
    cleared: Vec<ActorPosition>,
) -> SessionEvent {
    SessionEvent::WorldMap(AppliedDelta {
        server_sequence: 1,
        base_revision: 1,
        new_revision: 2,
        value: oteryn_session::world_map::WorldMapViewDelta {
            header,
            tiles,
            cleared,
        },
    })
}

#[test]
fn live_snapshot_replaces_local_geometry_and_a_new_join_replaces_old_tiles()
-> Result<(), BatchError> {
    let mut view = view()?;
    let map = live_snapshot(100);
    view.set_live_map(&map)?;
    assert_eq!(view.local.world().tile_count(), 1);
    assert!(view.local.world().tile(100, 200).is_some());
    view.set_world(Arc::new(World::builtin()?))?;
    assert_eq!(view.local.world().tile_count(), 1);
    view.set_live_map(&live_snapshot(101))?;
    assert!(view.local.world().tile(100, 200).is_none());
    assert!(view.local.world().tile(101, 200).is_some());
    Ok(())
}

#[test]
fn map_mailbox_retains_unchanged_tiles_and_coalesces_replacement_and_clear()
-> Result<(), BatchError> {
    let map = live_snapshot(100);
    let (link, _commands, outbox) = play_channel();
    mailbox::lock(&outbox.0).map = Some(Box::new(map.clone()));
    let mut added = map.tiles[0].clone();
    added.position.x += 1;
    outbox.push(vec![map_event(map.header, vec![added.clone()], vec![])]);
    let Some(PlayEvent::Pushed(first)) = link.poll() else {
        return Err(BatchError::InvalidView);
    };
    assert_eq!(first.map.as_ref().map(|map| map.tiles.len()), Some(2));
    added.ground_speed = 150;
    outbox.push(vec![map_event(map.header, vec![added.clone()], vec![])]);
    outbox.push(vec![map_event(
        map.header,
        vec![],
        vec![map.tiles[0].position],
    )]);
    let Some(PlayEvent::Pushed(latest)) = link.poll() else {
        return Err(BatchError::InvalidView);
    };
    assert_eq!(
        latest.map.as_ref().map(|map| &map.tiles),
        Some(&vec![added])
    );
    let mut view = view()?;
    view.set_live_map(&map)?;
    view.apply_pushed(&latest)?;
    assert!(view.local.world().tile(100, 200).is_none());
    assert!(view.local.world().tile(101, 200).is_some());
    assert!(link.poll().is_none());
    Ok(())
}

#[test]
fn slow_shell_map_reduction_evicts_departed_tiles_even_after_returning() -> Result<(), BatchError> {
    let map = live_snapshot(100);
    let (link, _commands, outbox) = play_channel();
    mailbox::lock(&outbox.0).map = Some(Box::new(map.clone()));
    for x in 101..=130 {
        let next = live_snapshot(x);
        outbox.push(vec![map_event(next.header, next.tiles, vec![])]);
    }
    for x in (100..130).rev() {
        outbox.push(vec![map_event(live_snapshot(x).header, vec![], vec![])]);
    }
    let Some(PlayEvent::Pushed(latest)) = link.poll() else {
        return Err(BatchError::InvalidView);
    };
    let map = latest.map.ok_or(BatchError::InvalidView)?;
    assert!(
        map.tiles.is_empty(),
        "departed tiles must not reappear on return"
    );
    Ok(())
}

#[test]
fn pushed_maps_stay_ordered_around_step_outcomes() -> Result<(), BatchError> {
    let map = live_snapshot(100);
    let (link, _commands, outbox) = play_channel();
    mailbox::lock(&outbox.0).map = Some(Box::new(map.clone()));
    outbox.push(vec![map_event(
        map.header,
        vec![],
        vec![map.tiles[0].position],
    )]);
    outbox.stepped(moved_to(101, 200, 0));
    outbox.push(vec![map_event(map.header, map.tiles.clone(), vec![])]);
    let Some(PlayEvent::Pushed(before)) = link.poll() else {
        return Err(BatchError::InvalidView);
    };
    assert!(before.map.is_some_and(|map| map.tiles.is_empty()));
    assert!(matches!(link.poll(), Some(PlayEvent::Stepped(_))));
    let Some(PlayEvent::Pushed(after)) = link.poll() else {
        return Err(BatchError::InvalidView);
    };
    assert_eq!(after.map, Some(Box::new(map)));
    Ok(())
}

#[test]
fn floor_replacement_follows_origin_and_draws_visible_floors() -> Result<(), BatchError> {
    let mut view = view()?;
    hide_minimap(&mut view)?;
    let mut map = live_snapshot(100);
    map.header.origin.floor = -8;
    map.header.first_visible_floor = -8;
    map.header.reset_epoch = 2;
    let base = map.tiles[0].clone();
    map.tiles = [-10, -9, -8, -6]
        .map(|floor| {
            let mut tile = base.clone();
            let shift = i32::from(floor - map.header.origin.floor);
            tile.position = ActorPosition {
                x: 100 + shift,
                y: 200 + shift,
                floor,
            };
            tile
        })
        .to_vec();
    view.set_live_map(&map)?;
    assert_eq!(view.map_center(), (100, 200, -8));
    // -6 is hidden by the server's first-visible-floor boundary.
    assert_eq!(view.local.world().tile_count(), 3);
    assert_eq!(view.local.world().draw_floors(), vec![-10, -9, -8]);
    for floor in [-10, -9, -8] {
        assert!(view.local.world().floor_tile(floor, 100, 200).is_some());
    }
    assert_eq!(view.scene().sprites().len(), 4); // Three map glyphs and the own actor.
    Ok(())
}

#[test]
fn floor_transition_cancels_the_previous_floor_click_walk() -> Result<(), BatchError> {
    let mut view = view()?;
    view.queue(Input::TargetWalk(ActorPosition {
        x: 110,
        y: 200,
        floor: 0,
    }));
    let mut map = live_snapshot(100);
    map.header.origin.floor = -8;
    map.header.first_visible_floor = -8;
    map.tiles.clear();
    view.set_live_map(&map)?;
    assert!(view.inputs.is_empty());
    Ok(())
}

#[test]
fn surface_and_deep_windows_draw_every_server_visible_floor() -> Result<(), BatchError> {
    use oteryn_session::world_map::floors_in_view;
    for (floor, first, count) in [
        (0, 0, 8),
        (-7, 0, 8),
        (-8, -8, 3),
        (-10, -8, 5),
        (-15, -13, 3),
    ] {
        let mut map = live_snapshot(100);
        map.header.origin.floor = floor;
        map.header.first_visible_floor = first;
        let base = map.tiles[0].clone();
        map.tiles = floors_in_view(floor)
            .ok_or(BatchError::InvalidView)?
            .map(|layer| {
                let shift = i32::from(layer - floor);
                let mut tile = base.clone();
                tile.position = ActorPosition {
                    x: 100 + shift,
                    y: 200 + shift,
                    floor: layer,
                };
                tile
            })
            .collect();
        let mut view = PlayView::new(TileCoord::new(100, 200), floor, &[])?;
        hide_minimap(&mut view)?;
        view.set_live_map(&map)?;
        assert_eq!(view.local.world().tile_count(), count);
        assert_eq!(view.scene().sprites().len(), count + 1);
    }
    Ok(())
}

#[test]
fn replacement_snapshot_discards_overlapping_old_tiles_before_later_deltas()
-> Result<(), BatchError> {
    let mut initial = live_snapshot(100);
    let mut old = initial.tiles[0].clone();
    old.position.x = 101;
    initial.tiles.push(old);
    let (link, _commands, outbox) = play_channel();
    mailbox::lock(&outbox.0).map = Some(Box::new(initial));
    let mut replacement = live_snapshot(100);
    replacement.header.reset_epoch = 2;
    outbox.push(vec![SessionEvent::WorldMapSnapshot(AppliedDelta {
        server_sequence: 2,
        base_revision: 1,
        new_revision: 2,
        value: replacement.clone(),
    })]);
    let mut added = replacement.tiles[0].clone();
    added.position.x = 102;
    outbox.push(vec![map_event(
        replacement.header,
        vec![added.clone()],
        vec![],
    )]);
    replacement.tiles.push(added);
    let Some(PlayEvent::Pushed(pushed)) = link.poll() else {
        return Err(BatchError::InvalidView);
    };
    assert_eq!(pushed.map, Some(Box::new(replacement)));
    let mut view = view()?;
    view.apply_pushed(&pushed)?;
    assert_eq!(view.local.world().tile_count(), 2);
    assert!(view.local.world().tile(101, 200).is_none());
    Ok(())
}

#[test]
fn failed_live_map_install_preserves_position_world_scene_and_walk() -> Result<(), BatchError> {
    let mut view = view()?;
    view.set_live_map(&live_snapshot(100))?;
    view.queue(Input::TargetWalk(ActorPosition {
        x: 110,
        y: 200,
        floor: 0,
    }));
    let world = view.local.world();
    let sprites = view.scene().sprites().instances().to_vec();
    let mut invalid = live_snapshot(i32::MAX);
    invalid.header.origin.floor = -8;
    invalid.tiles.clear();
    assert!(view.set_live_map(&invalid).is_err());
    assert_eq!(view.map_center(), (100, 200, 0));
    assert!(Arc::ptr_eq(&world, &view.local.world()));
    assert_eq!(view.scene().sprites().instances(), sprites);
    assert_eq!(
        view.inputs.back(),
        Some(&Input::TargetWalk(ActorPosition { x: 110, y: 200, floor: 0 }))
    );
    Ok(())
}

#[test]
fn floor_replacement_preserves_ground_item_targets_without_extra_glyphs()
-> Result<(), BatchError> {
    use oteryn_session::world_map::MapOrigin;
    let mut view = view()?;
    hide_minimap(&mut view)?;
    let mut map = live_snapshot(101);
    map.header.origin.floor = -8;
    map.header.first_visible_floor = -8;
    map.tiles[0].position.floor = -8;
    map.tiles[0].position.x += 1;
    map.tiles[0].items[0].origin = MapOrigin::BaseOrdinal {
        ordinal: 3,
        object_revision: 9,
    };
    map.tiles[0].items[0].count = 7;
    view.apply_pushed(&Pushed {
        map: Some(Box::new(map)),
        ..Pushed::default()
    })?;
    assert_eq!(view.map_center(), (101, 200, -8));
    // The map placeholder and player are drawn; item targets add no marker glyph.
    assert_eq!(view.scene().sprites().len(), 2);
    view.click(TileCoord::new(102, 200))?;
    let selected = view.scene().target().and_then(|target| target.item);
    assert_eq!(selected.map(|item| item.floor()), Some(-8));
    assert_eq!(selected.map(|item| item.quantity()), Some(7));
    assert!(!view.inputs.iter().any(|input| matches!(input, Input::TargetWalk(_))));
    Ok(())
}

// Floor sprite assertions isolate world rendering from the package-5 minimap overlay.
fn hide_minimap(view: &mut PlayView) -> Result<(), BatchError> {
    let mut preferences = view.preferences().clone();
    preferences.visible_panels.retain(|panel| panel != "minimap");
    view.set_preferences(preferences).map_err(|_| BatchError::InvalidView)
}
