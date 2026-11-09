//! M20: exact carried item-on-item recipes borrow the fenced Quest/value owner.
use super::ComposedFreshAdmission;
use super::connection::{UseCommand, UseOutcome};
use super::item_ref_admission::NativeItemWeights;
use super::item_view::ItemKey;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::{QuestCarriedUse, QuestRewardAdmission};
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use oteryn_gameplay_quests::quest::reward_callbacks;
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseDisposition, UseIntent};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_carried_quest_recipe(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        if !matches!(intent.use_with_map_item, Some(MapUseTarget::Item(_))) {
            return Ok(None);
        }
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
        // Replays are resolved before consulting handles for the consumed operands.
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
        let (Some(ItemKey::Instance(source)), Some(ItemKey::Instance(target))) = (source, target)
        else {
            return Ok(None);
        };
        if source == target {
            return Ok(None);
        }
        let Some(catalogue) = self.quest_catalogue.as_ref() else {
            return Ok(None);
        };
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, item_fence)
            .await
            .map_err(|_| ())?;
        let find = |id| {
            inventory
                .backpack
                .iter()
                .flat_map(|b| b.entries.iter().map(|e| &e.item))
                .chain(inventory.nested.iter().map(|(_, e)| &e.item))
                .find(|i| i.item_instance_id == id)
        };
        let (Some(used), Some(target_item)) = (find(source), find(target)) else {
            return Ok(None);
        };
        let callbacks = reward_callbacks::load_for(catalogue).map_err(|_| ())?;
        let bound = callbacks
            .iter()
            .filter(|c| {
                c.carried_item.as_deref() == Some(&used.definition.production_key)
                    && c.carried_target.as_deref() == Some(&target_item.definition.production_key)
            })
            .collect::<Vec<_>>();
        if bound.is_empty() {
            return Ok(None);
        }
        let physical = || {
            self.states
                .player_trades
                .physical(command.game_session_id, Some(source))
                && self
                    .states
                    .player_trades
                    .physical(command.game_session_id, Some(target))
        };
        if intent.use_with.is_some() || intent.use_with_position.is_some() || !physical() {
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
        if fence.item_fence() != item_fence || !physical() {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        let native = self
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or(())?;
        let weights = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        let use_ = QuestCarriedUse {
            source,
            recipe_target: Some(target),
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
                    actor_conditions: None,
                    carried_use: Some(&use_),
                    creature_use: None,
                    peer_use: None,
                },
                &self.premium_coordinator,
            )
            .await
            .map_err(|_| ())?;
        drop(permit);
        let committed = matches!(
            result,
            QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)
        );
        if committed {
            self.refresh_quest_session(command.game_session_id, false)
                .await;
        }
        Ok(Some(outcome(if committed {
            UseDisposition::Committed
        } else {
            UseDisposition::Rejected
        })))
    }
}
