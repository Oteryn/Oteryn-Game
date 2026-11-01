//! Delivery of Grizzly's earned holy icon uses the existing item MINT owner.
//! An actual NPC CommandRef supplies the cause; relog never invents a client command.

use super::ComposedFreshAdmission;
use super::connection::UseCommand;
use super::item_ref_admission::{NativeItemWeights, generation_item_facts, pinned_index};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::reward_claim_mint::{
    RewardClaimMintError, RewardClaimMintRequest, RewardClaimRefusal,
};
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use oteryn_protocol_oteryn::world_object::UseDisposition;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Called after an accepted Grizzly utterance, with its spell lane released. Full
    /// bags/capacity retain the durable entitlement, retried by a later utterance even
    /// after relog. `Committed` requests the ordinary inventory refresh.
    pub(super) async fn deliver_grizzly_reward(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
    ) -> Option<UseDisposition> {
        let Some(expected) = command.item_fence else {
            return Some(UseDisposition::Rejected);
        };
        let pending = match self
            .root
            .pending_grizzly_reward(self.character, expected.character_id)
            .await
        {
            Ok(Some(pending)) => pending,
            Ok(None) => return None,
            Err(_) => return Some(UseDisposition::Rejected),
        };
        let Some(_permit) = self.spell_lane_permit().await else {
            return Some(UseDisposition::Rejected);
        };
        let Ok(Some(fence)) = self.current_quest_fence(command.game_session_id).await else {
            return Some(UseDisposition::Rejected);
        };
        if fence.item_fence() != expected {
            return Some(UseDisposition::Rejected);
        }
        let Some(native) = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
        else {
            return Some(UseDisposition::Rejected);
        };
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
                    .physical_service(command.game_session_id)
                || !runtime
                    .bound_attacker_lease(actor, command.game_session_id)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    })
            {
                return Some(UseDisposition::Rejected);
            }
            let Some(index) = pinned_index(&runtime, self.active_generation) else {
                return Some(UseDisposition::Rejected);
            };
            let key = format!("oteryn:item.tibia.i{}", pending.item_source_id);
            let Some(revision) = index.revision(&key) else {
                return Some(UseDisposition::Rejected);
            };
            let Ok(item) = generation_item_facts(
                native,
                &TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: key,
                    revision_ref: revision.into(),
                },
            ) else {
                return Some(UseDisposition::Rejected);
            };
            item
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
        let Ok(backpack) = generation_item_facts(native, &bag.backpack.definition) else {
            return Some(UseDisposition::Rejected);
        };
        let Ok(id) = CommandId::new(command.command_id) else {
            return Some(UseDisposition::Rejected);
        };
        let request = RewardClaimMintRequest {
            command: CommandRef::new(command.game_session_id, id),
            claim: TypedDefinitionRef {
                family: "RewardClaim".into(),
                production_key: pending.claim_key,
                revision_ref: "grizzly-adams-r1".into(),
            },
            source_placement: pending.transition_key,
            item,
            quantity: 1,
            contents: Vec::new(),
            root_text: None,
            backpack,
            // Keep the earning pin in the entitlement. Resolve the durable Item
            // identity in the current generation so deployments cannot strand it.
            content_revision: format!(
                "sha256:{}",
                native
                    .source_digest()
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            ),
            ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
            sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
            achievement: None,
            quest_transition: None,
            key_number: None,
        };
        let outcome = self
            .root
            .claim_pending_grizzly_reward(
                self.character,
                self.holder,
                fence.item_fence(),
                request.clone(),
                &NativeItemWeights(native),
            )
            .await;
        Some(match outcome {
            Ok(_) => {
                self.states.loop_ground.invalidate();
                UseDisposition::Committed
            }
            Err(RewardClaimMintError::Unavailable(_)) => {
                // The transaction may already have minted the Item. Invalidating a
                // projection grants nothing; the next inventory read remains durable.
                self.states.loop_ground.invalidate();
                if self
                    .root
                    .reconcile_grizzly_reward(self.character, fence.character_id, request)
                    .await
                    .is_ok_and(|receipt| receipt.is_some())
                {
                    self.states.loop_ground.invalidate();
                    UseDisposition::Committed
                } else {
                    UseDisposition::Rejected
                }
            }
            Err(RewardClaimMintError::Refused(
                RewardClaimRefusal::MainBackpackFull
                | RewardClaimRefusal::CarryingCapacity
                | RewardClaimRefusal::NoMainBackpack,
            )) => UseDisposition::NoRoom,
            Err(_) => UseDisposition::Rejected,
        })
    }
}
