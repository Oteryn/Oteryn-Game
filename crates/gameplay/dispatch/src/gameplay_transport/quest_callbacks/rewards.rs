//! Deliver an earned shrine item through the existing once-claim MINT owner.
use super::super::item_ref_admission::{NativeItemWeights, generation_item_facts, pinned_index};
use super::*;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::reward_claim_mint::{
    RewardClaimMintError, RewardClaimMintRequest, RewardClaimRefusal,
};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn deliver_monk_reward(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        callback: &Callback,
        fence: CurrentCharacterGameplayFence,
    ) -> Option<UseDisposition> {
        callback.monk_reward.as_ref()?.item_source_id?;
        let pending = match self
            .root
            .pending_monk_reward(self.character, fence.character_id, &callback.key)
            .await
        {
            Ok(Some(pending)) => pending,
            Ok(None) => return None,
            Err(_) => return Some(UseDisposition::Rejected),
        };
        let native = self.active_generation?.native_gameplay()?;
        let item = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
                || states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, command.game_session_id)
                || !self
                    .states
                    .player_trades
                    .physical(command.game_session_id, None)
            {
                return Some(UseDisposition::Rejected);
            }
            let index = pinned_index(&runtime, self.active_generation)?;
            let key = format!("oteryn:item.tibia.i{}", pending.item_source_id);
            let revision = index.revision(&key)?;
            generation_item_facts(
                native,
                &TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: key,
                    revision_ref: revision.into(),
                },
            )
            .ok()?
        };
        let bag = match self
            .root
            .read_character_backpack(self.character, fence.character_id)
            .await
        {
            Ok(Some(bag)) => bag,
            Ok(None) => return Some(UseDisposition::NoRoom),
            Err(_) => return Some(UseDisposition::Rejected),
        };
        let backpack = generation_item_facts(native, &bag.backpack.definition).ok()?;
        let id = CommandId::new(command.command_id).ok()?;
        let request = RewardClaimMintRequest {
            command: CommandRef::new(command.game_session_id, id),
            claim: TypedDefinitionRef {
                family: "RewardClaim".into(),
                production_key: pending.claim_key,
                revision_ref: "monk-pilgrimage-r1".into(),
            },
            source_placement: callback.key.clone(),
            item,
            quantity: 1,
            backpack,
            // The entitlement retains the earning generation as evidence. Delivery
            // resolves the exact stable Item identity in the current pinned generation,
            // so a later deployment cannot strand a previously full backpack.
            content_revision: format!(
                "sha256:{}",
                native
                    .source_digest()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            ),
            ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
            sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
            achievement: None,
            quest_transition: None,
            key_number: None,
        };
        Some(
            match self
                .root
                .claim_pending_monk_reward(
                    self.character,
                    self.holder,
                    fence.item_fence(),
                    &callback.key,
                    request,
                    &NativeItemWeights(native).with_wheel(self.premium_coordinator),
                )
                .await
            {
                Ok(_) => {
                    self.states.loop_ground.invalidate();
                    UseDisposition::Committed
                }
                Err(RewardClaimMintError::Refused(
                    RewardClaimRefusal::MainBackpackFull
                    | RewardClaimRefusal::CarryingCapacity
                    | RewardClaimRefusal::NoMainBackpack,
                )) => UseDisposition::NoRoom,
                Err(_) => UseDisposition::Rejected,
            },
        )
    }
}
