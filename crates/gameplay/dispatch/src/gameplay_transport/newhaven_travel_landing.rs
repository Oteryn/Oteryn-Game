//! Share the existing paid-travel landing facts with the new free departure.
use super::*;
use crate::map::view::MapFacts;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Free legacy defaults retain their existing behavior. The newly admitted
    /// continuation must prove its town and real landing in this current World.
    pub(super) async fn tutorial_departure_admissible(&self, route: NewhavenRoute) -> bool {
        let Some(town) = route.home_town() else {
            return true;
        };
        let Some(map) = self.bundle_map() else {
            return false;
        };
        if map.overlay.base().subareas.town(town).is_none() {
            return false;
        }
        let runtime = self.runtime.lock().await;
        let (x, y, floor) = route.destination();
        self.travel_landing_admissible(
            &runtime,
            crate::foundation::MovementLocalPosition { x, y, floor },
        )
    }

    pub(super) fn travel_landing_admissible(
        &self,
        runtime: &ChannelRuntimeV1,
        cell: crate::foundation::MovementLocalPosition,
    ) -> bool {
        super::super::respawn_cell_admissible(
            self.movement_cells().index(),
            self.movement_cells().scope(),
            self.world_id,
            runtime.content_pin().server_artifact_digest(),
            &std::collections::BTreeSet::new(),
            cell,
        ) && !self.bundle_map().is_some_and(|map| {
            crate::content::bundle_spell_tiles::tile_pos(crate::content::LogicalCell {
                x: cell.x,
                y: cell.y,
                z: i32::from(cell.floor),
            })
            .is_none_or(|tile| map.facts.house_tile(tile) || map.facts.doors().blocks(tile))
        })
    }
}
