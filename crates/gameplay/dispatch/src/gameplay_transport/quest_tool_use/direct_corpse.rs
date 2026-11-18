//! Plain USE samples a current death-sourced Item before committing its handoff.
use super::*;
use crate::durability::{
    quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest},
    reward_claim_mint::{QuestRewardAdmission, quest_creature::QuestCreatureUseProof},
};
use oteryn_gameplay_quests::quest::reward_callbacks;

impl ComposedFreshAdmission<'_, '_, '_> {
    // Crystal 201189f8 actions_mission03_the_keeper.lua:17–21.
    pub(in crate::gameplay_transport) async fn use_direct_quest_corpse(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        if source.is_some_and(|key| !matches!(key, ItemKey::Entity(_))) {
            return Ok(None);
        }
        let (Some(item_fence), Some(catalogue)) = (command.item_fence, &self.quest_catalogue)
        else {
            return Ok(None);
        };
        let callbacks = reward_callbacks::load_for(catalogue).map_err(|_| ())?;
        if !callbacks
            .iter()
            .any(|callback| callback.is_direct_corpse_use())
        {
            return Ok(None);
        }
        let session = command.game_session_id;
        let command_ref =
            CommandRef::new(session, CommandId::new(command.command_id).map_err(|_| ())?);
        let outcome = |disposition| {
            Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            })
        };
        let Some(_permit) = self.spell_lane_permit().await else {
            return Ok(None);
        };
        let encoded =
            oteryn_protocol_oteryn::world_object::encode_item_use_intent(intent).map_err(|_| ())?;
        if let Some(matches) = self
            .root
            .reconcile_quest_carried_use(
                self.character,
                item_fence.character_id,
                command_ref,
                encoded,
            )
            .await
            .map_err(|_| ())?
        {
            return Ok(outcome(if matches {
                UseDisposition::Committed
            } else {
                UseDisposition::Rejected
            }));
        }
        let Some(ItemKey::Entity(identity)) = source else {
            return Ok(None);
        };
        let Some(fence) = self.current_quest_fence(session).await? else {
            return Ok(None);
        };
        if fence.item_fence() != item_fence || !self.states.player_trades.physical(session, None) {
            return Ok(outcome(UseDisposition::Rejected));
        }
        let mut revision = self.revision_sequencer.acquire(fence.character_id).await;
        let runtime = self.runtime.lock().await;
        let Some(body) = runtime.bound_corpse(identity) else {
            return Ok(None);
        };
        let mut selected = callbacks.iter().filter(|callback| {
            callback.is_direct_corpse_use()
                && callback.corpse_target.as_ref().is_some_and(|spec| {
                    body.item.production_key == format!("oteryn:item.tibia.i{}", spec.appearance)
                })
        });
        let Some(callback) = selected.next() else {
            return Ok(None);
        };
        if selected.next().is_some() {
            return Ok(outcome(UseDisposition::Rejected));
        }
        {
            let states = self.states.spell_states.lock().await;
            if states.is_dead(actor) || !states.spell_idle(&runtime, actor, session) {
                return Ok(outcome(UseDisposition::Rejected));
            }
        }
        let Some(proof) = QuestCreatureUseProof::read_direct_corpse(
            &runtime,
            actor,
            fence,
            command_ref,
            callback,
            intent,
            identity,
        ) else {
            return Ok(outcome(UseDisposition::Rejected));
        };
        let native = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
            .ok_or(())?;
        let weights = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        let result = revision
            .commit_quest_reward(
                self.root,
                self.character,
                self.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: callback.key.clone(),
                    cause: QuestCause::Command(command_ref),
                },
                std::sync::Arc::clone(catalogue),
                QuestRewardAdmission {
                    native,
                    weights: &weights,
                    fee_payment: None,
                    actor_conditions: None,
                    carried_use: None,
                    creature_use: Some(&proof),
                    peer_use: None,
                },
                &self.premium_coordinator,
            )
            .await
            .map_err(|_| ())?;
        drop(runtime);
        drop(revision);
        if matches!(
            result,
            QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)
        ) {
            self.refresh_quest_session(session, false).await;
            Ok(outcome(UseDisposition::Committed))
        } else {
            Ok(outcome(UseDisposition::Rejected))
        }
    }
}
