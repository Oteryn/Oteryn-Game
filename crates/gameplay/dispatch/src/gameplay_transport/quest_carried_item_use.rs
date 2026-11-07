//! Exact selected carried quest USE reuses the fenced quest/value owner (QENGINE2 97cb9db79).
use super::ComposedFreshAdmission;
use super::connection::{UseCommand, UseOutcome};
use super::item_ref_admission::NativeItemWeights;
use super::item_view::ItemKey;
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent};

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
        let callbacks = reward_callbacks::load().map_err(|_| ())?;
        let bound = callbacks
            .iter()
            .filter(|c| c.carried_item.as_deref() == Some(&item.definition.production_key))
            .collect::<Vec<_>>();
        if bound.is_empty() {
            return Ok(None);
        }
        if intent.use_with.is_some()
            || intent.use_with_map_item.is_some()
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
                eligible(&c.storage)
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
        let fence = self
            .current_quest_fence(command.game_session_id)
            .await
            .map_err(|_| ())?
            .ok_or(())?;
        if fence.item_fence() != item_fence {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        let native = self
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or(())?;
        let weights = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        let use_ = QuestCarriedUse {
            source,
            intent: encoded,
        };
        let result = self
            .revision_sequencer
            .acquire(fence.character_id)
            .await
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
                    carried_use: Some(&use_),
                },
            )
            .await
            .map_err(|_| ())?;
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
