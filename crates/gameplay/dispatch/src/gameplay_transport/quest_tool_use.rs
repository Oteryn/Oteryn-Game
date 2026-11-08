//! Selected quest tools retain a live target through the fenced value commit.
use super::{
    ComposedFreshAdmission,
    connection::{UseCommand, UseOutcome},
    item_ref_admission::NativeItemWeights,
    item_view::ItemKey,
};
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent};
use std::sync::atomic::{AtomicBool, Ordering};

// A possibly committed capture cannot leave the target available to another
// action. Cancellation and an unknown acknowledgement retire this owner turn.
struct CreatureTurn<'a> {
    runtime: tokio::sync::MutexGuard<'a, ChannelRuntimeV1>,
    uncertain: AtomicBool,
}
impl Drop for CreatureTurn<'_> {
    fn drop(&mut self) {
        if self.uncertain.load(Ordering::Relaxed) {
            self.runtime.retire_owner_cycle();
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Ordinary carried quest tools borrow the same fenced quest/value owner as
    /// NPC exchanges. The selected instance and ingress remain in its input proof.
    pub(super) async fn use_carried_quest_item(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        use crate::durability::quest_state::{
            QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
        };
        use crate::durability::reward_claim_mint::{QuestCarriedUse, QuestRewardAdmission};
        use oteryn_gameplay_quests::quest::reward_callbacks;
        let Some(item_fence) = command.item_fence else {
            return Ok(None);
        };
        if !matches!(
            item_fence.runtime_scope,
            crate::foundation::RuntimeScopeRefV1::Channel { .. }
        ) {
            return Ok(None);
        }
        let command_ref = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).map_err(|_| ())?,
        );
        let encoded =
            oteryn_protocol_oteryn::world_object::encode_item_use_intent(intent).map_err(|_| ())?;
        let outcome = |disposition| UseOutcome {
            disposition,
            ..UseOutcome::rejected()
        };
        if let Some(matches) = self
            .root
            .reconcile_quest_carried_use(
                self.character,
                item_fence.character_id,
                command_ref,
                encoded.clone(),
            )
            .await
            .map_err(|_| ())?
        {
            return Ok(Some(outcome(if matches {
                UseDisposition::Committed
            } else {
                UseDisposition::Rejected
            })));
        }
        let Some(ItemKey::Instance(source)) = source else {
            return Ok(None);
        };
        let Ok(inventory) = self
            .root
            .read_item_use_inventory(self.character, self.holder, item_fence)
            .await
        else {
            return Ok(None);
        };
        let Some(item) = inventory
            .backpack
            .iter()
            .flat_map(|bag| bag.entries.iter().map(|entry| &entry.item))
            .chain(inventory.nested.iter().map(|(_, entry)| &entry.item))
            .find(|item| item.item_instance_id == source)
        else {
            return Ok(None);
        };
        let Some(catalogue) = self.quest_catalogue.as_deref() else {
            return Ok(None);
        };
        let callbacks = reward_callbacks::load_for(catalogue).map_err(|_| ())?;
        let bound = callbacks
            .iter()
            .filter(|c| c.carried_item.as_deref() == Some(&item.definition.production_key))
            // Selected two-input recipes retain their own M20 admission owner.
            .filter(|c| c.carried_target.is_none())
            .collect::<Vec<_>>();
        if bound.is_empty() {
            return Ok(None);
        }
        if intent.use_with_map_item.is_some()
            || intent.use_with_position.is_some()
            || !self
                .states
                .player_trades
                .physical(command.game_session_id, Some(source))
        {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if states.is_dead(actor) || !states.spell_idle(&runtime, actor, command.game_session_id)
            {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
        }
        let Some(catalogue) = self.quest_catalogue.as_ref() else {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        };
        let callback = {
            let sessions = self.states.quest_sessions.lock().map_err(|_| ())?;
            let tracks = sessions
                .get(&command.game_session_id)
                .and_then(|s| s.copy.as_ref())
                .ok_or(())?
                .tracks();
            let eligible =
                |storage: &[oteryn_gameplay_quests::quest::world_callbacks::Storage]| {
                    storage.iter().all(|s| {
                        tracks
                            .get(&s.track)
                            .copied()
                            .or_else(|| catalogue.track(&s.track).map(|t| t.initial))
                            .is_some_and(|v| (s.minimum..=s.maximum).contains(&v))
                    })
                };
            let mut selected = bound.into_iter().filter(|c| {
                ((c.peer_requires.is_some() || c.creature_target.is_some())
                    == intent.use_with.is_some())
                    && eligible(&c.storage)
                    && c.requires.iter().all(|r| r.holds(catalogue, tracks))
                    && c.state_ops
                        .iter()
                        .all(|op| op.advisory_holds(catalogue, tracks))
                    && c.participants.iter().all(|p| {
                        eligible(&p.storage)
                            && p.state_ops
                                .iter()
                                .all(|op| op.advisory_holds(catalogue, tracks))
                    })
            });
            let Some(callback) = selected.next() else {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            };
            if selected.next().is_some() {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            callback.clone()
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        };
        // A queued duplicate may have probed before the first capture committed.
        // Reconcile again after serialization, while its former target can be absent.
        if callback.capture_creature
            && let Some(matches) = self
                .root
                .reconcile_quest_carried_use(
                    self.character,
                    item_fence.character_id,
                    command_ref,
                    encoded.clone(),
                )
                .await
                .map_err(|_| ())?
        {
            return Ok(Some(outcome(if matches {
                UseDisposition::Committed
            } else {
                UseDisposition::Rejected
            })));
        }
        let fence = self
            .current_quest_fence(command.game_session_id)
            .await
            .map_err(|_| ())?
            .ok_or(())?;
        if fence.item_fence() != item_fence {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        let peer = if callback.peer_requires.is_some() {
            match self.quest_peer_use(actor, command, intent).await {
                Ok(peer) => peer,
                Err(()) => return Ok(Some(outcome(UseDisposition::Rejected))),
            }
        } else {
            None
        };
        // Revision serialization precedes the runtime turn, matching the other
        // physical action owners. Keep the live target current through COMMIT.
        let mut revision = self.revision_sequencer.acquire(fence.character_id).await;
        let mut creature_runtime = if callback.creature_target.is_some() {
            Some(CreatureTurn {
                runtime: self.runtime.lock().await,
                uncertain: AtomicBool::new(false),
            })
        } else {
            None
        };
        let creature = if callback.creature_target.is_some() {
            let runtime = &*creature_runtime.as_ref().ok_or(())?.runtime;
            let proof = crate::durability::reward_claim_mint::quest_creature::QuestCreatureUseProof::read_current(
                runtime, actor, fence, command_ref, &callback, intent);
            let Some(proof) = proof else {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            };
            Some(proof)
        } else {
            None
        };
        let native = self
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or(())?;
        let weights = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        let use_ = QuestCarriedUse {
            recipe_target: None,
            source,
            intent: encoded,
        };
        let capture_target = creature
            .as_ref()
            .filter(|_| callback.capture_creature)
            .map(|proof| proof.target_actor());
        let capture_key = callback.creature_target.clone();
        if callback.capture_creature
            && let Some(turn) = &creature_runtime
        {
            turn.uncertain.store(true, Ordering::Relaxed);
        }
        let result = revision
            .commit_quest_reward(
                self.root,
                self.character,
                self.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: callback.key,
                    cause: QuestCause::Command(command_ref),
                },
                std::sync::Arc::clone(catalogue),
                QuestRewardAdmission {
                    native,
                    weights: &weights,
                    fee_payment: None,
                    actor_conditions: None,
                    carried_use: Some(&use_),
                    creature_use: creature.as_ref(),
                    peer_use: peer.as_ref(),
                },
                &self.premium_coordinator,
            )
            .await
            .map_err(|_| ())?;
        if let Some(turn) = &mut creature_runtime {
            if matches!(result, QuestTransitionOutcome::Committed(_))
                && let Some(target) = capture_target
            {
                turn.runtime
                    .remove_mount_taming_subject(target, capture_key.as_deref().ok_or(())?)
                    .map_err(|_| ())?;
            }
            turn.uncertain.store(false, Ordering::Relaxed);
        }
        drop(creature_runtime);
        drop(revision);
        drop(permit);
        let disposition = if matches!(
            result,
            QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)
        ) {
            self.refresh_quest_session(command.game_session_id, false)
                .await;
            UseDisposition::Committed
        } else {
            UseDisposition::Rejected
        };
        Ok(Some(outcome(disposition)))
    }
}
