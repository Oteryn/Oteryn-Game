//! Deferred native-hit credit uses the same revision and session owners as native kills.
pub(super) mod queue;
use super::ComposedFreshAdmission;
use crate::durability::{
    character_progression::CharacterProgressionError, quest_state::QuestTransitionOutcome,
};
use std::sync::Arc;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Crystal raging_mage_tower/creaturescripts_raging_mage_1.lua:20–25.
    pub(super) async fn eject_mage_tower_spectators(
        &self,
        epoch: u64,
        fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
        death: crate::foundation::CreatureDeathOccurrenceKey,
        center: crate::foundation::MovementLocalPosition,
    ) -> Result<(), crate::durability::DurabilityError> {
        use crate::durability::DurabilityError;
        use crate::foundation::MovementLocalPosition;

        let queue = self.states.attack.lock().await.kills().clone();
        if queue.mage_tower_room_qualified(death) {
            return Ok(());
        }
        let in_room = |at: MovementLocalPosition| {
            at.floor == center.floor
                && at.x.abs_diff(center.x) <= 10
                && at.y.abs_diff(center.y) <= 10
        };
        let (spectators, raging) = {
            let runtime = self.runtime.lock().await;
            let positioned = runtime
                .positioned_actor_census()
                .map_err(|_| DurabilityError::Unavailable)?;
            let mut spectators = Vec::new();
            let mut raging = None;
            for (actor, position, session) in positioned {
                if !in_room(position.position()) {
                    continue;
                }
                if let Some(session) = session {
                    let lease = runtime
                        .bound_attacker_lease(actor, session)
                        .map_err(|_| DurabilityError::Unavailable)?;
                    spectators.push((actor, session, lease));
                } else if runtime
                    .matches_live_creature_identity(actor, b"oteryn:creature.raging_mage")
                    && runtime
                        .world_spawn_cohort(actor)
                        .is_some_and(|(key, _, _)| key == "the_mage_s_tower")
                {
                    raging = Some(actor);
                }
            }
            (spectators, raging)
        };
        if spectators.is_empty() {
            queue.mark_mage_tower_room_qualified(death);
            return Ok(());
        }
        let ids = spectators
            .iter()
            .map(|(_, _, lease)| lease.character_id().as_bytes().to_vec())
            .collect();
        let Some(unworthy) = self
            .root
            .mage_tower_unworthy(self.character, self.holder, fence, epoch, ids, raging)
            .await?
        else {
            return Ok(());
        };
        if !unworthy.is_empty() {
            let map = self.bundle_map().ok_or(DurabilityError::Unavailable)?;
            let closed = crate::map::floor::BundleDoors(map.facts.doors());
            let mut runtime = self.runtime.lock().await;
            let mut spell_states = self
                .states
                .spell_states
                .try_lock()
                .map_err(|_| DurabilityError::Unavailable)?;
            runtime
                .owner_fence()
                .map_err(|_| DurabilityError::Unavailable)?;
            let exit = MovementLocalPosition {
                x: 33142,
                y: 31529,
                floor: 3,
            };
            for (actor, session, lease) in spectators {
                if !unworthy
                    .iter()
                    .any(|id| id.as_slice() == lease.character_id().as_bytes())
                    || runtime.bound_attacker_lease(actor, session) != Ok(lease)
                {
                    continue;
                }
                let position = runtime
                    .read_actor_position(actor)
                    .map_err(|_| DurabilityError::Unavailable)?;
                if in_room(position.position()) {
                    let proof = crate::map::floor::prepare_quest_encounter_ejection(
                        &runtime,
                        self.movement_cells(),
                        actor,
                        session,
                        position,
                        exit,
                        &closed,
                    )
                    .map_err(|_| DurabilityError::Unavailable)?;
                    let moved = runtime
                        .commit_source_step(proof)
                        .map_err(|_| DurabilityError::Unavailable)?;
                    let before = position.position();
                    let after = moved.position();
                    let now = self.owner_now().get();
                    spell_states.stage_special_movement_step(
                        &runtime,
                        map,
                        actor,
                        session,
                        now,
                        Some(before),
                        after,
                        None,
                    );
                    spell_states.stage_underwater_step(
                        &runtime,
                        actor,
                        session,
                        now,
                        super::paced_movement::underwater_drown::underwater(map, before),
                        super::paced_movement::underwater_drown::underwater(map, after),
                    );
                }
            }
        }
        queue.mark_mage_tower_room_qualified(death);
        Ok(())
    }

    async fn drain_mage_tower_hits(&self) -> bool {
        let queue = self.states.attack.lock().await.kills().clone();
        for hit in queue.pending_mage_tower_hits() {
            let Ok(current) = self.current_quest_fence(hit.session).await else {
                return false;
            };
            if let Some(fence) = current {
                let live = {
                    let runtime = self.runtime.lock().await;
                    runtime.owner_fence().is_ok()
                        && runtime.bound_attacker_lease(hit.actor, hit.session) == Ok(hit.lease)
                };
                if live
                    && fence.character_id.as_bytes() == hit.lease.character_id().as_bytes()
                    && fence.character_lease_generation == hit.lease.generation()
                    && self
                        .root
                        .consume_mage_tower_kills(
                            self.character,
                            self.holder,
                            fence,
                            hit.epoch,
                            hit.target,
                        )
                        .await
                        .is_err()
                {
                    return false;
                }
            }
            queue.acknowledge_mage_tower_hit(hit);
        }
        true
    }

    pub(super) async fn drain_native_quest_hits(&self) -> bool {
        if !self.drain_mage_tower_hits().await {
            return false;
        }
        let pending = {
            let attack = self.states.attack.lock().await;
            let Ok(cache) = attack.kills().1.lock() else {
                return false;
            };
            cache.hits.pending()
        };
        for pending in pending {
            let Some(catalogue) = &self.quest_catalogue else {
                return false;
            };
            let fence = match self.current_quest_fence(pending.session).await {
                Ok(Some(fence)) => Some(fence),
                Ok(None) => None,
                Err(()) => return false,
            };
            let mut accepted = false;
            if let Some(fence) = fence {
                let live = {
                    let runtime = self.runtime.lock().await;
                    runtime.owner_fence().is_ok()
                        && runtime.bound_attacker_lease(pending.actor, pending.session)
                            == Ok(pending.lease)
                };
                if live
                    && pending.current_for(catalogue)
                    && fence.character_id.as_bytes() == pending.lease.character_id().as_bytes()
                    && fence.character_lease_generation == pending.lease.generation()
                {
                    let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
                    let outcome = slot
                        .commit_native_quest_hit(
                            self.root,
                            self.character,
                            self.holder,
                            fence,
                            &pending.hit,
                            Arc::clone(catalogue),
                        )
                        .await;
                    drop(slot);
                    match outcome {
                        Ok(
                            QuestTransitionOutcome::Committed(_)
                            | QuestTransitionOutcome::AlreadyCommitted(_),
                        ) => accepted = true,
                        Ok(
                            QuestTransitionOutcome::Refused(_)
                            | QuestTransitionOutcome::ObligationClosed,
                        ) => {}
                        Err(
                            CharacterProgressionError::Unavailable(_)
                            | CharacterProgressionError::CharacterRevisionMismatch,
                        ) => {
                            self.schedule_quest_refresh(pending.session);
                            return false;
                        }
                        Err(error) => {
                            super::operator_event(&format!("quest_hit_refused reason={error:?}"))
                        }
                    }
                }
            }
            {
                let attack = self.states.attack.lock().await;
                let Ok(mut cache) = attack.kills().1.lock() else {
                    return false;
                };
                cache.hits.acknowledge(&pending);
            }
            if accepted {
                self.refresh_quest_session(pending.session, false).await;
            } else {
                self.schedule_quest_refresh(pending.session);
            }
        }
        let attack = self.states.attack.lock().await;
        let Ok(cache) = attack.kills().1.lock() else {
            return false;
        };
        !cache.hits.has_pending()
    }
}
