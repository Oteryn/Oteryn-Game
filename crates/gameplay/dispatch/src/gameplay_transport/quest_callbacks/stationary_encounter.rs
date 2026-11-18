//! Crystal 201189f8 movements_marziel_blood.lua:27-37. The stepper stays in place.
use super::*;
use crate::content::{CollisionClass, LogicalCell};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn prepare_stationary_encounter<'a>(
        &self,
        runtime: &mut ChannelRuntimeV1,
        map: &BundleMap,
        callback: &'a Callback,
    ) -> Result<Vec<(&'a str, i64, MovementLocalPosition)>, UseDisposition> {
        let rows = &callback.effects.stationary_encounter;
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let census = runtime
            .positioned_melee_census()
            .map_err(|_| UseDisposition::Rejected)?;
        let mut monsters = Vec::with_capacity(rows.len());
        for row in rows {
            let at = local(row.position);
            let cell = LogicalCell {
                x: at.x,
                y: at.y,
                z: i32::from(at.floor),
            };
            if self
                .movement_cells
                .index()
                .lookup(self.movement_cells.scope(), cell)
                != Ok(CollisionClass::Walkable)
                || map
                    .facts
                    .quest_callback_blocked(map.overlay.base(), tile(row.position), false)
                || census
                    .iter()
                    .any(|(_, position, _)| position.position() == at)
            {
                return Err(UseDisposition::Occupied);
            }
            let policy = runtime
                .companion_policy(&row.creature)
                .map_err(|_| UseDisposition::Rejected)?;
            monsters.push((row.creature.as_str(), policy.maximum_health, at));
        }
        runtime
            .preflight_quest_creature_spawn(&monsters)
            .map_err(|_| UseDisposition::Exhausted)?;
        Ok(monsters)
    }
}
