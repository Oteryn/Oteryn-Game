//! Crystal 201189f actions_sacrophagus_use.lua:31-44.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn prepare_roll_failure<'a>(
        &self,
        runtime: &mut ChannelRuntimeV1,
        map: &BundleMap,
        position: TilePos,
        creature: &'a str,
    ) -> Result<Option<(&'a str, i64, MovementLocalPosition)>, UseDisposition> {
        let native = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
            .ok_or(UseDisposition::Rejected)?;
        native
            .install_companion_policies(runtime)
            .map_err(|_| UseDisposition::Rejected)?;
        let health = runtime
            .companion_policy(creature)
            .map_err(|_| UseDisposition::Rejected)?
            .maximum_health;
        let census = runtime
            .positioned_melee_census()
            .map_err(|_| UseDisposition::Rejected)?;
        let candidates = [
            (0, 0),
            (-1, 0),
            (1, 0),
            (0, -1),
            (0, 1),
            (-1, -1),
            (-1, 1),
            (1, -1),
            (1, 1),
        ];
        let landing = candidates
            .into_iter()
            .map(|(dx, dy)| MovementLocalPosition {
                x: i32::from(position.x) + dx,
                y: i32::from(position.y) + dy,
                floor: -i16::from(position.floor),
            })
            .find(|to| {
                self.movement_cells().index().lookup(
                    self.movement_cells().scope(),
                    LogicalCell {
                        x: to.x,
                        y: to.y,
                        z: i32::from(to.floor),
                    },
                ) == Ok(CollisionClass::Walkable)
                    && !census.iter().any(|(_, at, _)| at.position() == *to)
                    && native_position_open(map, *to)
            });
        Ok(landing.map(|to| (creature, health, to)))
    }
}

fn native_position_open(map: &BundleMap, to: MovementLocalPosition) -> bool {
    native(to).is_some_and(|to| {
        !map.facts.house_tile(to)
            && !map
                .facts
                .quest_callback_blocked(map.overlay.base(), to, false)
    })
}
