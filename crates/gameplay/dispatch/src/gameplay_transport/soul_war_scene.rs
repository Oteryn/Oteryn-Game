//! Read current, independently owned source Bundle stacks without inventing map cells.
use crate::content::QualifiedNativeEntryRoom;
use crate::foundation::MovementLocalPosition;
use crate::map::boot::BundleCollisionIndexExt;
use crate::map::{
    boot::BundleMap,
    overlay::TilePos,
    view::{ComposedTile, EntrySource},
};

pub(super) fn source_map(room: &QualifiedNativeEntryRoom) -> Option<&BundleMap> {
    room.pinned_activity_source_room()?;
    match room.movement_cells().index() {
        crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) => {
            Some(index.map())
        }
        _ => None,
    }
}
pub(super) fn wire_position(at: MovementLocalPosition) -> Option<TilePos> {
    if !(0..=15).contains(&at.floor) {
        return None;
    }
    Some(TilePos {
        x: at.x.try_into().ok()?,
        y: at.y.try_into().ok()?,
        floor: at.floor.checked_neg()?.try_into().ok()?,
    })
}
pub(super) fn current_tile(
    room: &QualifiedNativeEntryRoom,
    at: MovementLocalPosition,
) -> Option<ComposedTile> {
    let map = source_map(room)?;
    crate::map::view::compose_tile(&map.overlay, &map.facts, wire_position(at)?).ok()?
}
#[derive(Debug, Clone, Copy)]
pub(super) struct SourceGround {
    pub position: TilePos,
    pub key: u64,
    pub appearance_id: u16,
}
pub(super) fn current_ground(
    room: &QualifiedNativeEntryRoom,
    at: MovementLocalPosition,
) -> Option<SourceGround> {
    let tile = current_tile(room, at)?;
    let entry = tile.entries.first()?;
    if entry.facts.terrain_kind != Some(oteryn_world_bundle::bundle::TerrainKind::Ground) {
        return None;
    }
    let EntrySource::Base {
        ordinal: 0,
        placement_key,
        ..
    } = entry.source
    else {
        return None;
    };
    Some(SourceGround {
        position: tile.pos,
        key: placement_key,
        appearance_id: entry.facts.appearance_id,
    })
}
pub(super) fn emit_effect(
    room: &QualifiedNativeEntryRoom,
    at: MovementLocalPosition,
    effect: u16,
) -> bool {
    let Some(map) = source_map(room) else {
        return false;
    };
    let Some(at) = wire_position(at) else {
        return false;
    };
    map.facts.emit_movement_effect(at, effect)
}

/// Exact immutable donor appearance top order, read once by a source callback owner.
pub(super) fn source_top_orders(
    room: &QualifiedNativeEntryRoom,
) -> Result<std::collections::BTreeMap<u16, u8>, ()> {
    let kind = room.pinned_activity_source_room().ok_or(())?;
    let value: serde_json::Value = serde_json::from_slice(kind.bytes()).map_err(|_| ())?;
    let mut orders = std::collections::BTreeMap::new();
    for policy in value["item_policies"].as_array().ok_or(())? {
        let id = u16::try_from(policy["server_item_id"].as_u64().ok_or(())?).map_err(|_| ())?;
        let order = u8::try_from(policy["source_top_order"].as_u64().ok_or(())?).map_err(|_| ())?;
        if order > 3 || orders.insert(id, order).is_some() {
            return Err(());
        }
    }
    Ok(orders)
}
