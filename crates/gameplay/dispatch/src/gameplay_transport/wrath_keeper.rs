//! Exact ground corpse USE and fenced Quest reward admission.
use super::{
    ComposedFreshAdmission,
    connection::{UseCommand, UseOutcome},
    item_ref_admission::NativeItemWeights,
    item_view::ItemKey,
};
use crate::durability::{
    quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest},
    reward_claim_mint::{QuestRewardAdmission, quest_creature::QuestCreatureUseProof},
};
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use oteryn_gameplay_quests::quest::{reward_callbacks, wrath_keeper};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_wrath_keeper_corpse(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        let (Some(ItemKey::Instance(item)), Some(item_fence), Some(catalogue), Some(native)) = (
            source,
            command.item_fence,
            self.quest_catalogue.as_ref(),
            self.active_generation.and_then(|g| g.native_gameplay()),
        ) else {
            return Ok(None);
        };
        let Some(ground) = self
            .root
            .read_ground_action_source(self.character, item_fence, item)
            .await
            .map_err(|_| ())?
            .filter(|ground| ground.definition().production_key == wrath_keeper::CORPSE)
        else {
            return Ok(None);
        };
        let outcome = |disposition| {
            Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            })
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return Ok(outcome(UseDisposition::Rejected));
        };
        let Some(fence) = self
            .current_quest_fence(command.game_session_id)
            .await
            .map_err(|_| ())?
            .filter(|fence| fence.item_fence() == item_fence)
        else {
            return Ok(outcome(UseDisposition::Rejected));
        };
        let command_ref = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).map_err(|_| ())?,
        );
        let callback = reward_callbacks::load_for(catalogue)
            .map_err(|_| ())?
            .into_iter()
            .find(|callback| callback.key == wrath_keeper::TAIL_HANDOFF)
            .ok_or(())?;
        let mut sequence = self.revision_sequencer.acquire(fence.character_id).await;
        let runtime = self.runtime.lock().await;
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, None)
            || {
                let states = self.states.spell_states.lock().await;
                states.is_dead(actor)
                    || !states.spell_idle(&runtime, actor, command.game_session_id)
            }
        {
            return Ok(outcome(UseDisposition::Rejected));
        }
        let Some(proof) = QuestCreatureUseProof::read_corpse(
            &runtime,
            actor,
            fence,
            command_ref,
            &callback,
            intent,
            ground.quest_corpse_operand(),
            &permit,
        ) else {
            return Ok(outcome(UseDisposition::Rejected));
        };
        let weights = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        let result = sequence
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
                    carried_use: None,
                    creature_use: Some(&proof),
                    peer_use: None,
                },
                &self.premium_coordinator,
            )
            .await
            .map_err(|_| ())?;
        drop(runtime);
        drop(sequence);
        drop(permit);
        if matches!(
            result,
            QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)
        ) {
            self.refresh_quest_session(command.game_session_id, false)
                .await;
            return Ok(outcome(UseDisposition::Committed));
        }
        Ok(outcome(UseDisposition::NotPossible))
    }
}
