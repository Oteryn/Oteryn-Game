//! Crystal 201189f8 data-global/scripts/quests/others/actions_holy_water.lua:65-80.
use super::*;
use crate::content::{CollisionClass, LogicalCell};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn prepare_callback_summon<'a>(
        &self,
        runtime: &mut ChannelRuntimeV1,
        map: &BundleMap,
        callback: &'a Callback,
        position: TilePos,
    ) -> Result<Option<(&'a str, i64, MovementLocalPosition)>, UseDisposition> {
        let Some(summon) = &callback.effects.use_summon else {
            return Ok(None);
        };
        let at = local(Position {
            x: position.x,
            y: position.y,
            floor: position.floor,
        });
        if self.movement_cells.index().lookup(
            self.movement_cells.scope(),
            LogicalCell {
                x: at.x,
                y: at.y,
                z: i32::from(at.floor),
            },
        ) != Ok(CollisionClass::Walkable)
            || map
                .facts
                .quest_callback_blocked(map.overlay.base(), position, false)
            || runtime
                .positioned_melee_census()
                .map_err(|_| UseDisposition::Rejected)?
                .iter()
                .any(|(_, snapshot, _)| snapshot.position() == at)
        {
            return Err(UseDisposition::Occupied);
        }
        let policy = runtime
            .companion_policy(&summon.creature)
            .map_err(|_| UseDisposition::Rejected)?;
        let monster = (summon.creature.as_str(), policy.maximum_health, at);
        runtime
            .preflight_quest_creature_spawn(&[monster])
            .map_err(|_| UseDisposition::Occupied)?;
        Ok(Some(monster))
    }
}
