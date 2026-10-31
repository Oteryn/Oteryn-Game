//! Crystal's fixed Soul War chest delivers the retained reward through the existing MINT owner.
use super::super::item_ref_admission::{NativeItemWeights, generation_item_facts, pinned_index};
use super::*;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::quest_state::soul_war::{
    SOUL_WAR_REWARD_PLACEMENT, SOUL_WAR_REWARD_REVISION,
};
use crate::durability::reward_claim_mint::{
    RewardClaimMintError, RewardClaimMintRequest, RewardClaimRefusal,
};

#[derive(Clone)]
pub(super) struct Pending {
    character: crate::domain::CharacterId,
    request: RewardClaimMintRequest,
}

fn is_source_chest(position: TilePos) -> bool {
    // Crystal201189f action-reward_soul_war.lua:4-28 registers this position.
    // BundleMap TilePos is native negative depth; carrier z is converted once for reach.
    position
        == (TilePos {
            x: 33620,
            y: 31400,
            floor: -10,
        })
}

fn disposition(error: &RewardClaimMintError) -> UseDisposition {
    match error {
        RewardClaimMintError::Refused(
            RewardClaimRefusal::MainBackpackFull
            | RewardClaimRefusal::CarryingCapacity
            | RewardClaimRefusal::NoMainBackpack,
        ) => UseDisposition::NoRoom,
        _ => UseDisposition::Rejected,
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Receipt reconciliation only. A disconnected character cannot authorize a new MINT.
    pub(super) async fn reconcile_soul_war_reward_receipts(&self, session: GameSessionId) -> bool {
        let pending = match self.states.quest_callbacks.soul_rewards.lock() {
            Ok(records) => records.get(&session).cloned(),
            Err(_) => return false,
        };
        let Some(pending) = pending else {
            return true;
        };
        let mut slot = self.revision_sequencer.acquire(pending.character).await;
        slot.observe_endgame_progression();
        let result = match self
            .root
            .reconcile_soul_war_reward(self.character, pending.character, &pending.request)
            .await
        {
            Ok(Some(_)) => UseDisposition::Committed,
            Ok(None) => UseDisposition::Rejected,
            Err(_) => return false,
        };
        if self
            .states
            .command_ingress
            .complete_quest_callback(pending.request.command, result)
            .is_none()
        {
            return false;
        }
        if result == UseDisposition::Committed {
            self.states.loop_ground.invalidate();
        }
        match self.states.quest_callbacks.soul_rewards.lock() {
            Ok(mut records) => {
                records.remove(&session);
                true
            }
            Err(_) => false,
        }
    }

    pub(super) async fn use_soul_war_reward(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &BundleMap,
        target: &WorldObjectTarget,
    ) -> Option<UseOutcome> {
        let generation = self.active_generation?;
        let source = super::super::world_map::MapViewSource {
            overlay: &map.overlay,
            facts: &map.facts,
            content_generation: generation.identity().client_artifact_digest(),
            reset_epoch: self.reset_epoch,
        };
        let resolved =
            super::super::world_map::resolve_map_target(&source, target, |_| None::<()>).ok()?;
        let (position, _) = crate::map::view::placement_of(resolved.placement_key)?;
        if !is_source_chest(position) {
            return None;
        }
        let result = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            let Some(_permit) = self.spell_lane_permit().await else {
                return UseDisposition::Rejected;
            };
            let Some(item_fence) = command.item_fence else {
                return UseDisposition::Rejected;
            };
            let mut slot = self
                .revision_sequencer
                .acquire(item_fence.character_id)
                .await;
            let Ok(Some(fence)) = self.current_quest_fence(command.game_session_id).await else {
                return UseDisposition::Rejected;
            };
            if item_fence != fence.item_fence() {
                return UseDisposition::Rejected;
            }
            let Ok(id) = CommandId::new(command.command_id) else {
                return UseDisposition::Rejected;
            };
            let Some(native) = generation.native_gameplay() else {
                return UseDisposition::Rejected;
            };
            let pending = match self
                .root
                .pending_soul_war_reward(self.character, fence.character_id)
                .await
            {
                Ok(Some(pending)) => pending,
                Ok(None) | Err(_) => return UseDisposition::Rejected,
            };
            let item = {
                let runtime = self.runtime.lock().await;
                let states = self.states.spell_states.lock().await;
                if super::super::hazard::qualified_source_frame(&runtime, self.movement_cells)
                    .is_none()
                    || !runtime
                        .player_control_facts(actor, command.game_session_id)
                        .is_ok_and(|facts| facts.control_loss.is_none())
                    || states.is_dead(actor)
                    || !states.spell_idle(&runtime, actor, command.game_session_id)
                    || !self
                        .states
                        .player_trades
                        .physical(command.game_session_id, None)
                    || !runtime
                        .bound_attacker_lease(actor, command.game_session_id)
                        .is_ok_and(|lease| {
                            lease.character_id().as_bytes() == fence.character_id.as_bytes()
                                && lease.generation() == fence.character_lease_generation
                        })
                {
                    return UseDisposition::Rejected;
                }
                let Ok(expected) = runtime.read_actor_position(actor) else {
                    return UseDisposition::Rejected;
                };
                let Some(at) = super::native(expected.position()) else {
                    return UseDisposition::Rejected;
                };
                if !super::super::map_door::reachable(at, resolved.pos) {
                    return UseDisposition::TooFar;
                }
                if !self
                    .states
                    .world_gate
                    .lock()
                    .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()))
                {
                    return UseDisposition::Exhausted;
                }
                let Some(index) = pinned_index(&runtime, self.active_generation) else {
                    return UseDisposition::Rejected;
                };
                let key = format!("oteryn:item.tibia.i{}", pending.item_source_id);
                let Some(revision) = index.revision(&key) else {
                    return UseDisposition::Rejected;
                };
                let Ok(item) = generation_item_facts(
                    native,
                    &TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: key,
                        revision_ref: revision.into(),
                    },
                ) else {
                    return UseDisposition::Rejected;
                };
                item
            };
            let bag = match self
                .root
                .read_character_backpack(self.character, fence.character_id)
                .await
            {
                Ok(Some(bag)) => bag,
                Ok(None) => return UseDisposition::NoRoom,
                Err(_) => return UseDisposition::Rejected,
            };
            let Ok(backpack) = generation_item_facts(native, &bag.backpack.definition) else {
                return UseDisposition::Rejected;
            };
            let request = RewardClaimMintRequest {
                command: CommandRef::new(command.game_session_id, id),
                claim: TypedDefinitionRef {
                    family: "RewardClaim".into(),
                    production_key: pending.claim_key,
                    revision_ref: SOUL_WAR_REWARD_REVISION.into(),
                },
                source_placement: SOUL_WAR_REWARD_PLACEMENT.into(),
                item,
                quantity: 1,
                backpack,
                content_revision: format!(
                    "sha256:{}",
                    native
                        .source_digest()
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>()
                ),
                ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION
                    .into(),
                sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
                achievement: None,
                quest_transition: None,
                key_number: None,
            };
            {
                let Ok(mut records) = self.states.quest_callbacks.soul_rewards.lock() else {
                    return UseDisposition::Rejected;
                };
                if records.contains_key(&command.game_session_id) {
                    return UseDisposition::Rejected;
                }
                records.insert(
                    command.game_session_id,
                    Pending {
                        character: fence.character_id,
                        request: request.clone(),
                    },
                );
            }
            // A timeout can cancel this future after SQL commits. Clear the
            // cursor before awaiting MINT so the next writer reads the root.
            slot.observe_endgame_progression();
            let outcome = self
                .root
                .claim_pending_soul_war_reward(
                    self.character,
                    self.holder,
                    fence,
                    request,
                    &NativeItemWeights(native),
                )
                .await;
            slot.observe_endgame_progression();
            if matches!(outcome, Err(RewardClaimMintError::Unavailable(_))) {
                // The input gate observes the retained cause and settles it by read-only recovery.
                return UseDisposition::Rejected;
            }
            if let Ok(mut records) = self.states.quest_callbacks.soul_rewards.lock() {
                records.remove(&command.game_session_id);
            }
            match outcome {
                Ok(_) => {
                    self.states.loop_ground.invalidate();
                    UseDisposition::Committed
                }
                Err(error) => disposition(&error),
            }
        })
        .await
        .unwrap_or(UseDisposition::Rejected);
        Some(UseOutcome {
            disposition: result,
            ..UseOutcome::rejected()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soul_war_reward_requires_exact_source_placement() {
        let chest = TilePos {
            x: 33620,
            y: 31400,
            floor: -10,
        };
        assert!(is_source_chest(chest));
        assert!(!is_source_chest(TilePos { floor: 10, ..chest }));
        assert!(!is_source_chest(TilePos { x: 33621, ..chest }));
    }
}
