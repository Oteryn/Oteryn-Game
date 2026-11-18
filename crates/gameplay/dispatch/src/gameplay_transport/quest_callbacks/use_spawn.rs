//! Immediate source spawn attempts after the existing Quest USE receipt commits.
//! Crystal 201189f8 data-global/scripts/lib/register_actions.lua:698-750.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn project_quest_use_spawns(
        &self,
        runtime: &mut ChannelRuntimeV1,
        map: &BundleMap,
        spawn: &world_callbacks::UseSpawn,
        cause: QuestCause,
    ) {
        let QuestCause::Use(command) = cause else {
            return;
        };
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return;
        };
        if native.install_companion_policies(runtime).is_err() || runtime.owner_fence().is_err() {
            return;
        }
        let Ok(census) = runtime.positioned_melee_census() else {
            return;
        };
        let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        let hash = Sha256::new()
            .chain_update(b"oteryn:quest-use-spawn:v1")
            .chain_update(command.game_session_id().as_bytes())
            .chain_update(command.command_id().get().to_be_bytes())
            .finalize();
        let mut occurrence = [0; 16];
        occurrence.copy_from_slice(&hash[..16]);
        let occurrence = DecisionOccurrenceId::from_bytes(occurrence);
        let Ok(maximum) = i64::try_from(spawn.creatures.len().saturating_sub(1)) else {
            return;
        };
        let cells = self.movement_cells();
        if cells.scope().generation_digest != runtime.content_pin().server_artifact_digest() {
            return;
        }
        for (ordinal, position) in spawn.positions.iter().enumerate() {
            let at = local(*position);
            let tile = tile(*position);
            // Crystal emits the cue for an attempt even when the cell is occupied.
            let _ = map.facts.emit_movement_effect(tile, spawn.effect);
            if cells.index().lookup(
                cells.scope(),
                LogicalCell {
                    x: at.x,
                    y: at.y,
                    z: i32::from(at.floor),
                },
            ) != Ok(CollisionClass::Walkable)
                || map
                    .facts
                    .quest_callback_blocked(map.overlay.base(), tile, false)
                || census
                    .iter()
                    .any(|(_, existing, _)| existing.position() == at)
            {
                continue;
            }
            let Ok(draw) = deterministic_decision_u64(
                &root,
                occurrence,
                "quest_use_spawn_creature",
                ordinal as u64,
            ) else {
                continue;
            };
            let Some(creature) = usize::try_from(crate::spell::uniform_draw(draw, 0, maximum))
                .ok()
                .and_then(|index| spawn.creatures.get(index))
            else {
                continue;
            };
            let Ok(policy) = runtime.companion_policy(creature) else {
                continue;
            };
            let _ = runtime.commit_quest_creature_spawn(&[(creature, policy.maximum_health, at)]);
        }
    }
}
