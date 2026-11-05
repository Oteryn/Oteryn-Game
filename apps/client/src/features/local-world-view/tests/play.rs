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
