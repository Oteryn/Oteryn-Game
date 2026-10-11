use super::*;
use crate::map_view::{MapTarget, MapView};
use oteryn_session::{
    ActorPosition, EntityDetail, EntityKind, EntityRef, ItemHandle, StepDirection,
    map_view::{
        MapDefinition, MapItem, MapOrigin, MapTile, MapViewHeader, WorldMapViewDelta,
        WorldMapViewSnapshot, encode_world_map_delta, encode_world_map_snapshot,
    },
};
use std::num::{NonZeroU32, NonZeroU64};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn secondary_use_click_obeys_modal_and_focus_lifecycle() -> TestResult {
    use oteryn_input_actions::{
        ButtonState, Modifiers, MouseButton, NormalizedInputEvent, PointerCoordinate, PointerDelta,
        PointerMotion, PointerPosition,
    };
    let mut actions = MouseActions::new()?;
    let moved = NormalizedInputEvent::PointerMoved {
        position: PointerPosition::new(PointerCoordinate::new(20)?, PointerCoordinate::new(30)?),
        motion: PointerMotion::new(PointerDelta::new(0)?, PointerDelta::new(0)?),
    };
    let press = NormalizedInputEvent::MouseButton {
        button: MouseButton::SECONDARY,
        state: ButtonState::Pressed,
        modifiers: Modifiers::NONE,
    };
    let clicks = actions.route(&[moved, press.clone()]);
    assert_eq!(
        clicks,
        vec![ClickAction {
            x: 20.0,
            y: 30.0,
            use_item: true
        }]
    );
    actions.set_modal_active(true)?;
    assert!(actions.route(std::slice::from_ref(&press)).is_empty());
    actions.set_modal_active(false)?;
    actions.route(&[NormalizedInputEvent::FocusChanged { focused: false }]);
    assert!(actions.route(&[press]).is_empty());
    Ok(())
}
fn position(x: i32, y: i32, floor: i16) -> ActorPosition {
    ActorPosition { x, y, floor }
}
fn header(visible: i16) -> MapViewHeader {
    MapViewHeader {
        content_generation: [1; 32],
        bundle_digest: [2; 32],
        reset_epoch: 3,
        origin: position(100, 100, -7),
        first_visible_floor: visible,
    }
}
fn handle() -> ItemHandle {
    NonZeroU64::MIN
}
fn tile(floor: i16, origin: MapOrigin) -> MapTile {
    let shift = i32::from(floor + 7);
    MapTile {
        position: position(100 + shift, 100 + shift, floor),
        more: true,
        ground_speed: 137,
        items: vec![MapItem {
            definition: MapDefinition::Item(NonZeroU32::MIN),
            count: 1,
            sub_type: 0,
            appearance_id: u16::MAX,
            origin,
        }],
    }
}
fn map(visible: i16, tiles: Vec<MapTile>) -> Result<MapView, String> {
    let bytes = encode_world_map_snapshot(&WorldMapViewSnapshot {
        header: header(visible),
        tiles,
    })
    .map_err(|e| format!("{e:?}"))?;
    let mut map = MapView::new([1; 32]).map_err(|e| format!("{e:?}"))?;
    map.replace_snapshot(&bytes).map_err(|e| format!("{e:?}"))?;
    Ok(map)
}

#[test]
fn map_picks_only_top_received_origin_on_highest_visible_floor() -> TestResult {
    let base = MapOrigin::BaseOrdinal {
        ordinal: 7,
        object_revision: 23,
    };
    let mut lower = tile(-7, base);
    lower.items.push(MapItem {
        origin: MapOrigin::Handle(handle()),
        ..lower.items[0]
    });
    let point = TileCoord::new(100, 100);
    let hidden = map(-7, vec![lower.clone(), tile(-6, MapOrigin::DisplayOnly)])?;
    assert_eq!(
        map_pick(&hidden, point).ok_or("no pick")?.target,
        Some(MapTarget::Item(handle()))
    );
    let upper = map(0, vec![lower, tile(-6, MapOrigin::DisplayOnly)])?;
    let picked = map_pick(&upper, point).ok_or("no upper pick")?;
    assert_eq!(picked.position.floor, -6);
    assert!(picked.target.is_none());
    let base_map = map(-7, vec![tile(-7, base)])?;
    let Some(MapTarget::Base {
        placement,
        expected_revision,
    }) = map_pick(&base_map, point).and_then(|pick| pick.target)
    else {
        return Err("not base target".into());
    };
    assert_eq!(&placement[..32], &[2; 32]);
    assert_eq!(
        &placement[32..],
        &((100u64 << 32) | (100u64 << 16) | (7 << 8) | 7).to_be_bytes()
    );
    assert_eq!(expected_revision, 23);
    Ok(())
}

#[test]
fn departed_tile_removes_handle_and_hidden_upper_floor_cannot_capture_actor_click() -> TestResult {
    let mut map = map(
        -7,
        vec![
            tile(-7, MapOrigin::Handle(handle())),
            tile(-6, MapOrigin::DisplayOnly),
        ],
    )?;
    let actor = oteryn_session::WorldSpatialEntity {
        kind: EntityKind::Creature,
        entity: EntityRef {
            identity: [3; 16],
            generation: 1,
        },
        position: position(100, 100, -7),
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 9,
            health_percent: 100,
        },
    };
    let point = TileCoord::new(100, 100);
    assert_eq!(
        map_actor_target(&map, &[actor], point).and_then(|t| t.entity),
        Some(actor.entity)
    );
    assert!(map.contains_target(MapTarget::Item(handle())));
    let mut next = header(0);
    let roof = encode_world_map_delta(
        &header(-7),
        &WorldMapViewDelta {
            header: next,
            tiles: vec![],
            cleared: vec![],
        },
    )
    .map_err(|e| format!("{e:?}"))?;
    map.apply_delta(&roof).map_err(|e| format!("{e:?}"))?;
    assert!(map_actor_target(&map, &[actor], point).is_none());
    let previous = next;
    next.origin.x += 1;
    let clear = encode_world_map_delta(
        &previous,
        &WorldMapViewDelta {
            header: next,
            tiles: vec![],
            cleared: vec![position(100, 100, -7)],
        },
    )
    .map_err(|e| format!("{e:?}"))?;
    map.apply_delta(&clear).map_err(|e| format!("{e:?}"))?;
    assert!(!map.contains_target(MapTarget::Item(handle())));
    Ok(())
}
