use super::feature_local_world_view::{live_snapshot, map_event};
use super::*;

#[test]
fn live_ground_item_identity_reaches_click_and_is_removed_with_tile() -> Result<(), BatchError> {
    use oteryn_session::world_map::{MapDefinition, MapOrigin};
    let mut view = view()?;
    let mut map = live_snapshot(101);
    map.tiles[0].items[0].origin = MapOrigin::BaseOrdinal {
        ordinal: 3,
        object_revision: 9,
    };
    map.tiles[0].items[0].count = 7;
    let mut pushed = Pushed::default();
    coalesce_pushed(
        &mut pushed,
        vec![map_event(map.header, map.tiles.clone(), vec![])],
    );
    view.apply_pushed(&pushed)?;
    view.click(TileCoord::new(101, 200))?;
    let selected = view.scene().target().and_then(|target| target.item);
    assert_eq!(selected.map(|item| item.definition()), Some(1));
    assert_eq!(selected.map(|item| item.quantity()), Some(7));
    assert!(
        !view
            .inputs
            .iter()
            .any(|input| matches!(input, Input::TargetWalk(_)))
    );
    let mut cleared = Pushed::default();
    coalesce_pushed(
        &mut cleared,
        vec![map_event(map.header, vec![], vec![map.tiles[0].position])],
    );
    view.apply_pushed(&cleared)?;
    assert_eq!(view.scene().target(), None);
    view.click(TileCoord::new(101, 200))?;
    assert_eq!(
        view.inputs.back(),
        Some(&Input::TargetWalk(oteryn_session::ActorPosition {
            x: 101,
            y: 200,
            floor: 0
        }))
    );
    // Terrain has its own reference family and must never become an item target.
    map.tiles[0].items[0].definition = MapDefinition::Terrain(NonZeroU32::MIN);
    view.set_live_map(&map)?;
    view.click(TileCoord::new(101, 200))?;
    assert_eq!(view.scene().target(), None);
    Ok(())
}

#[test]
fn ground_item_projection_keeps_stack_origin_and_floor() {
    use crate::play::feature_0004_item_view_move::state::map_targets;
    use oteryn_session::world_map::{MapDefinition, MapOrigin};
    use std::num::NonZeroU64;
    let mut map = live_snapshot(101);
    let mut top = map.tiles[0].items[0];
    top.count = 12;
    top.sub_type = 4;
    top.origin = MapOrigin::Handle(NonZeroU64::MIN);
    map.tiles[0].items.push(top);
    let targets = map_targets(&map);
    let picked = crate::input::pick_target(TileCoord::new(101, 200), &targets);
    assert_eq!(
        picked.and_then(|target| target.item),
        Some(GroundItem::Map {
            header: map.header,
            position: map.tiles[0].position,
            item: top,
        })
    );
    map.tiles[0].items[0].definition = MapDefinition::Terrain(NonZeroU32::MIN);
    assert_eq!(map_targets(&map).len(), 1);
    map.tiles[0].position.floor = -1;
    assert!(map_targets(&map).is_empty());
}

#[test]
fn ground_entity_projection_preserves_handle_definition_and_quantity() {
    use crate::play::feature_0004_item_view_move::state::entity_item;
    use oteryn_session::{EntityDetail, EntityKind, EntityRef, WorldSpatialEntity};
    use std::num::NonZeroU64;
    let entity = WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::GroundItem,
        entity: EntityRef {
            identity: [7; 16],
            generation: 0,
        },
        position: ActorPosition {
            x: 101,
            y: 200,
            floor: 0,
        },
        detail: EntityDetail::Object {
            item_definition_ref: 42,
            quantity: 8,
            item_handle: Some(NonZeroU64::MIN),
        },
    };
    assert_eq!(
        entity_item(&entity),
        Some(GroundItem::Entity {
            reference: entity.entity,
            position: entity.position,
            definition: 42,
            quantity: 8,
            handle: Some(NonZeroU64::MIN),
        })
    );
}

#[test]
fn drag_projection_requires_live_handle_and_follows_map_removals() -> Result<(), BatchError> {
    use oteryn_session::world_map::MapOrigin;
    let mut view = view()?;
    let mut map = live_snapshot(101);
    map.tiles[0].items[0].count = 12;
    map.tiles[0].items[0].origin = MapOrigin::Handle(std::num::NonZeroU64::MIN);
    view.set_live_map(&map)?;
    let tile = TileCoord::new(101, 200);
    let source = view.item_drag_source(tile);
    assert_eq!(source.map(|s| s.handle), Some(std::num::NonZeroU64::MIN));
    assert_eq!(source.map(|s| s.count), Some(12));
    view.floor = 1;
    assert_eq!(view.item_drag_source(tile), None);
    view.floor = 0;
    map.tiles.clear();
    view.set_live_map(&map)?;
    assert_eq!(view.item_drag_source(tile), None);
    Ok(())
}
