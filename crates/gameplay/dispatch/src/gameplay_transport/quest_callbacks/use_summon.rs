//! Crystal 201189f8 data-global/scripts/quests/others/actions_holy_water.lua:65-80.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use crate::movement::source_placement::{SourcePlacementCell, SourcePlacementPlan};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn prepare_callback_summon<'a>(
        &self,
        runtime: &mut ChannelRuntimeV1,
        map: &BundleMap,
        callback: &'a Callback,
        position: TilePos,
        cause: QuestCause,
    ) -> Result<Option<(&'a str, i64, MovementLocalPosition)>, UseDisposition> {
        let Some(summon) = &callback.effects.use_summon else {
            return Ok(None);
        };
        let at = local(Position {
            x: position.x,
            y: position.y,
            floor: position.floor,
        });
        let QuestCause::Use(command) = cause else {
            return Err(UseDisposition::Rejected);
        };
        let digest = Sha256::new()
            .chain_update(b"oteryn:quest-use-summon-placement:v1")
            .chain_update(command.game_session_id().as_bytes())
            .chain_update(command.command_id().get().to_be_bytes())
            .chain_update(callback.key.as_bytes())
            .finalize();
        let mut occurrence = [0; 16];
        occurrence.copy_from_slice(&digest[..16]);
        let occurrence = DecisionOccurrenceId::from_bytes(occurrence);
        let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        let mut ordinal = 0;
        // Game.createMonster uses the normal shuffled eight neighbors when the
        // requested cell cannot admit it. Crystal src/map/map.cpp:234-313.
        let plan = SourcePlacementPlan::prepare(at, false, false, |bound| {
            let draw =
                deterministic_decision_u64(&root, occurrence, "quest.summon.placement", ordinal);
            ordinal += 1;
            draw.ok()
                .and_then(|draw| {
                    u32::try_from(crate::spell::uniform_draw(draw, 0, i64::from(bound - 1))).ok()
                })
                .unwrap_or(bound)
        })
        .map_err(|_| UseDisposition::Rejected)?;
        let census = runtime
            .positioned_melee_census()
            .map_err(|_| UseDisposition::Rejected)?;
        let Some(at) = plan
            .select(
                |at| {
                    let position = native(at)?;
                    let cell = LogicalCell {
                        x: at.x,
                        y: at.y,
                        z: i32::from(at.floor),
                    };
                    let exists = map
                        .overlay
                        .base()
                        .tile(position.x, position.y, position.floor)
                        .is_some();
                    if !exists {
                        return Some(SourcePlacementCell {
                            exists,
                            query_add: false,
                            protection_zone: false,
                            teleport: false,
                        });
                    }
                    let flags = self
                        .movement_cells
                        .spell_tiles()
                        .lookup(self.movement_cells.scope(), cell)
                        .ok()?
                        .flags();
                    let teleport = crate::map::floor::is_teleport(self.movement_cells(), cell);
                    let query_add = !flags.protection_zone
                        && !flags.floor_change
                        && !teleport
                        && self
                            .movement_cells
                            .index()
                            .lookup(self.movement_cells.scope(), cell)
                            == Ok(CollisionClass::Walkable)
                        && !map
                            .facts
                            .quest_callback_blocked(map.overlay.base(), position, false)
                        && !census
                            .iter()
                            .any(|(_, snapshot, _)| snapshot.position() == at);
                    Some(SourcePlacementCell {
                        exists,
                        query_add,
                        protection_zone: flags.protection_zone,
                        teleport,
                    })
                },
                |_, _| Some(true),
            )
            .map_err(|_| UseDisposition::Rejected)?
        else {
            // Both source actions ignore a failed native creation and still settle their quest state.
            return Ok(None);
        };
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
