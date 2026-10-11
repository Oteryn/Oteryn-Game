use super::*;
use oteryn_session::map_view::{
    MapDefinition, MapItem, WorldMapViewDelta, encode_world_map_delta, encode_world_map_snapshot,
};
use std::num::{NonZeroU32, NonZeroU64};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn header() -> MapViewHeader {
    MapViewHeader {
        content_generation: [1; 32],
        bundle_digest: [2; 32],
        reset_epoch: 3,
        origin: position(100, 100),
        first_visible_floor: 0,
    }
}
fn position(x: i32, y: i32) -> ActorPosition {
    ActorPosition { x, y, floor: -7 }
}
fn item(origin: MapOrigin) -> MapItem {
    MapItem {
        definition: MapDefinition::Item(NonZeroU32::MIN),
        count: 1,
        sub_type: 0,
        origin,
        appearance_id: 0,
    }
}
fn tile(x: i32, y: i32, origin: MapOrigin) -> MapTile {
    MapTile {
        position: position(x, y),
        items: vec![item(origin)],
        more: false,
        ground_speed: 150,
    }
}
fn snapshot(view: &mut MapView, tiles: Vec<MapTile>) -> TestResult {
    let payload = encode_world_map_snapshot(&WorldMapViewSnapshot {
        header: header(),
        tiles,
    })
    .map_err(|error| format!("encode {error:?}"))?;
    view.replace_snapshot(&payload)
        .map_err(|error| format!("snapshot {error:?}"))?;
    Ok(())
}

#[test]
fn origin_move_drops_old_tiles_and_handles_before_replacements() -> TestResult {
    let mut view = MapView::new([1; 32]).map_err(|e| format!("{e:?}"))?;
    snapshot(
        &mut view,
        vec![
            tile(92, 100, MapOrigin::Handle(NonZeroU64::MIN)),
            tile(100, 100, MapOrigin::DisplayOnly),
        ],
    )?;
    let mut next = header();
    next.origin.x += 1;
    let changed = tile(110, 100, MapOrigin::DisplayOnly);
    let payload = encode_world_map_delta(
        &header(),
        &WorldMapViewDelta {
            header: next,
            tiles: vec![changed.clone()],
            cleared: vec![position(100, 100)],
        },
    )
    .map_err(|e| format!("{e:?}"))?;
    view.apply_delta(&payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(view.header(), Some(next));
    assert!(view.target(position(92, 100), 0).is_none());
    assert!(view.tile(position(100, 100)).is_none());
    assert_eq!(view.visible_tiles().collect::<Vec<_>>(), vec![&changed]);
    Ok(())
}

#[test]
fn targets_follow_origin_not_definition_or_appearance() -> TestResult {
    let mut view = MapView::new([1; 32]).map_err(|e| format!("{e:?}"))?;
    let mut entry = tile(
        100,
        100,
        MapOrigin::BaseOrdinal {
            ordinal: 3,
            object_revision: 27,
        },
    );
    entry.items.push(item(MapOrigin::Handle(NonZeroU64::MIN)));
    entry.items.push(item(MapOrigin::DisplayOnly));
    snapshot(&mut view, vec![entry])?;
    let mut expected = [2; 40];
    let key = 100_u64 << 32 | 100_u64 << 16 | 7_u64 << 8 | 3;
    expected[32..].copy_from_slice(&key.to_be_bytes());
    assert_eq!(
        view.target(position(100, 100), 0),
        Some(MapTarget::Base {
            placement: expected,
            expected_revision: 27,
        })
    );
    assert_eq!(
        view.target(position(100, 100), 1),
        Some(MapTarget::Item(NonZeroU64::MIN))
    );
    assert!(view.target(position(100, 100), 2).is_none());
    assert!(view.target(position(100, 100), 3).is_none());
    assert!(view.target(position(99, 100), 0).is_none());
    Ok(())
}

#[test]
fn binding_mismatch_invalidates_every_draw_and_target_until_snapshot() -> TestResult {
    for field in 0..3 {
        let mut view = MapView::new([1; 32]).map_err(|e| format!("{e:?}"))?;
        snapshot(
            &mut view,
            vec![tile(100, 100, MapOrigin::Handle(NonZeroU64::MIN))],
        )?;
        let mut other = header();
        match field {
            0 => other.content_generation[0] ^= 1,
            1 => other.bundle_digest[0] ^= 1,
            _ => other.reset_epoch += 1,
        }
        let payload = encode_world_map_delta(
            &other,
            &WorldMapViewDelta {
                header: other,
                tiles: vec![tile(100, 100, MapOrigin::DisplayOnly)],
                cleared: vec![],
            },
        )
        .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            view.apply_delta(&payload),
            Err(MapViewError::Wire(WorldMapWireError::BindingMismatch))
        );
        assert!(view.header().is_none());
        assert_eq!(view.visible_tiles().count(), 0);
        assert!(view.target(position(100, 100), 0).is_none());
        assert_eq!(
            view.apply_delta(&payload),
            Err(MapViewError::AwaitingSnapshot)
        );
        snapshot(&mut view, vec![])?;
        assert!(view.header().is_some());
    }
    Ok(())
}

#[test]
fn snapshot_replaces_reset_floor_and_targets_and_checks_admitted_generation() -> TestResult {
    let mut view = MapView::new([1; 32]).map_err(|e| format!("{e:?}"))?;
    snapshot(
        &mut view,
        vec![tile(100, 100, MapOrigin::Handle(NonZeroU64::MIN))],
    )?;
    let mut next = header();
    next.reset_epoch += 1;
    next.origin.floor = -10;
    next.first_visible_floor = -8;
    let payload = encode_world_map_snapshot(&WorldMapViewSnapshot {
        header: next,
        tiles: vec![],
    })
    .map_err(|e| format!("{e:?}"))?;
    view.replace_snapshot(&payload)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(view.header(), Some(next));
    assert!(view.target(position(100, 100), 0).is_none());
    next.content_generation = [9; 32];
    let payload = encode_world_map_snapshot(&WorldMapViewSnapshot {
        header: next,
        tiles: vec![],
    })
    .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        view.replace_snapshot(&payload),
        Err(MapViewError::ContentGeneration)
    );
    assert!(view.header().is_none());
    Ok(())
}

#[test]
fn header_only_roof_change_retains_tiles_but_hides_their_targets() -> TestResult {
    let mut view = MapView::new([1; 32]).map_err(|e| format!("{e:?}"))?;
    let mut above = tile(101, 101, MapOrigin::Handle(NonZeroU64::MIN));
    above.position.floor = -6;
    snapshot(&mut view, vec![above.clone()])?;
    assert_eq!(view.visible_tiles().count(), 1);
    let mut next = header();
    next.first_visible_floor = -7;
    let payload = encode_world_map_delta(
        &header(),
        &WorldMapViewDelta {
            header: next,
            tiles: vec![],
            cleared: vec![],
        },
    )
    .map_err(|e| format!("{e:?}"))?;
    view.apply_delta(&payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(view.visible_tiles().count(), 0);
    assert_eq!(view.tile(above.position), Some(&above));
    assert!(view.target(above.position, 0).is_none());
    assert_eq!(view.ground_speed(above.position), Some(150));
    Ok(())
}

#[test]
fn handle_budget_applies_to_accumulated_state_not_only_one_delta() -> TestResult {
    let mut view = MapView::new([1; 32]).map_err(|e| format!("{e:?}"))?;
    let mut tiles = Vec::new();
    let mut handle = 1;
    for y in 94..102 {
        for x in 92..108 {
            let mut tile = tile(x, y, MapOrigin::DisplayOnly);
            tile.items = (0..8)
                .map(|_| {
                    let item = item(MapOrigin::Handle(
                        NonZeroU64::new(handle).unwrap_or(NonZeroU64::MIN),
                    ));
                    handle += 1;
                    item
                })
                .collect();
            tiles.push(tile);
        }
    }
    assert_eq!(handle - 1, MAX_MAP_VIEW_HANDLES as u64);
    snapshot(&mut view, tiles)?;
    let payload = encode_world_map_delta(
        &header(),
        &WorldMapViewDelta {
            header: header(),
            tiles: vec![tile(
                108,
                102,
                MapOrigin::Handle(NonZeroU64::new(handle).ok_or("handle")?),
            )],
            cleared: vec![],
        },
    )
    .map_err(|e| format!("{e:?}"))?;
    assert_eq!(view.apply_delta(&payload), Err(MapViewError::Limit));
    assert!(view.header().is_none());
    assert_eq!(view.visible_tiles().count(), 0);
    Ok(())
}

#[test]
fn replacement_keeps_only_received_stack_and_no_invented_ground_speed() -> TestResult {
    let mut view = MapView::new([1; 32]).map_err(|e| format!("{e:?}"))?;
    snapshot(
        &mut view,
        vec![tile(100, 100, MapOrigin::Handle(NonZeroU64::MIN))],
    )?;
    let mut replacement = tile(100, 100, MapOrigin::DisplayOnly);
    replacement.more = true;
    replacement.ground_speed = 0;
    let payload = encode_world_map_delta(
        &header(),
        &WorldMapViewDelta {
            header: header(),
            tiles: vec![replacement.clone()],
            cleared: vec![],
        },
    )
    .map_err(|e| format!("{e:?}"))?;
    view.apply_delta(&payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(view.tile(position(100, 100)), Some(&replacement));
    assert!(view.target(position(100, 100), 0).is_none());
    assert!(view.target(position(100, 100), 1).is_none());
    assert!(view.ground_speed(position(100, 100)).is_none());
    assert!(view.ground_speed(position(99, 100)).is_none());
    Ok(())
}

#[test]
fn malformed_payload_drops_previous_state_and_map_capability_remains_unadvertised() -> TestResult {
    assert!(!oteryn_session::CLIENT_SUPPORTED_CAPABILITIES.contains(&18));
    assert_eq!(
        MapView::new([0; 32]).err(),
        Some(MapViewError::ContentGeneration)
    );
    let mut view = MapView::new([1; 32]).map_err(|e| format!("{e:?}"))?;
    snapshot(&mut view, vec![tile(100, 100, MapOrigin::DisplayOnly)])?;
    assert_eq!(
        view.replace_snapshot(&[0xff]),
        Err(MapViewError::Wire(WorldMapWireError::Malformed))
    );
    assert!(view.header().is_none());
    assert_eq!(view.visible_tiles().count(), 0);
    Ok(())
}
