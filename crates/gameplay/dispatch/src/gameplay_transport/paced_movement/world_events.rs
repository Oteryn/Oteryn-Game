//! Source Chyllfroest bridge projection on the ordinary player movement owner.
use crate::durability::world_event::changes::timed::{EventKind, seasonal_open};
use crate::foundation::MovementLocalPosition;
use crate::map::boot::BundleMap;

/// Crystal9f5 movements_prohibit_entry.lua:12-16,21: AID50000;
/// April 1 through May 1 inclusive; closed visitors return to the south shore.
pub(super) fn chyllfroest_return(
    map: &BundleMap,
    position: MovementLocalPosition,
    unix_ms: Option<i64>,
) -> Option<MovementLocalPosition> {
    if unix_ms.is_some_and(|now| seasonal_open(EventKind::Chyllfroest, now)) {
        return None;
    }
    let tile = super::special_movement::tile_entries(map, position)?;
    tile.entries
        .iter()
        .any(|(key, _)| {
            map.facts
                .quest_callback_matches(*key, None, Some(50000), None)
        })
        .then_some(MovementLocalPosition {
            x: 32063,
            y: 31091,
            floor: -7,
        })
}
