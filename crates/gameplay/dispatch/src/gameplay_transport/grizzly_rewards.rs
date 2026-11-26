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

pub(super) enum GrizzlyRewardDelivery {
    NoChange,
    NoRoom,
    RefreshInventory,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Called after an accepted Grizzly utterance, with its spell lane released. Full
    /// bags/capacity retain the durable entitlement, retried by a later utterance even
    /// after relog. Refresh inventory only if the mint may have changed custody.
    pub(super) async fn deliver_grizzly_reward(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
    ) -> GrizzlyRewardDelivery {
        let Some(expected) = command.item_fence else {
            return GrizzlyRewardDelivery::NoChange;
        };
        let pending = match self
            .root
            .pending_grizzly_reward(self.character, expected.character_id)
            .await
        {
            Ok(Some(pending)) => pending,
            Ok(None) => return GrizzlyRewardDelivery::NoChange,
            Err(_) => return GrizzlyRewardDelivery::NoChange,
        };
        let Some(_permit) = self.spell_lane_permit().await else {
            return GrizzlyRewardDelivery::NoChange;
        };
        let Ok(Some(fence)) = self.current_quest_fence(command.game_session_id).await else {
            return GrizzlyRewardDelivery::NoChange;
        };
        if fence.item_fence() != expected {
            return GrizzlyRewardDelivery::NoChange;
        }
        let Some(native) = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
        else {
            return GrizzlyRewardDelivery::NoChange;
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
                return GrizzlyRewardDelivery::NoChange;
            }
            let Some(index) = pinned_index(&runtime, self.active_generation) else {
                return GrizzlyRewardDelivery::NoChange;
            };
            let key = format!("oteryn:item.tibia.i{}", pending.item_source_id);
            let Some(revision) = index.revision(&key) else {
                return GrizzlyRewardDelivery::NoChange;
            };
            let Ok(item) = generation_item_facts(
                native,
                &TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: key,
                    revision_ref: revision.into(),
                },
            ) else {
                return GrizzlyRewardDelivery::NoChange;
            };
            item
        };
        let bag = match self
            .root
            .read_character_backpack(self.character, fence.character_id)
            .await
        {
            Ok(Some(bag)) => bag,
            Ok(None) => return GrizzlyRewardDelivery::NoRoom,
            Err(_) => return GrizzlyRewardDelivery::NoChange,
        };
        let Ok(backpack) = generation_item_facts(native, &bag.backpack.definition) else {
            return GrizzlyRewardDelivery::NoChange;
        };
        let Ok(id) = CommandId::new(command.command_id) else {
            return GrizzlyRewardDelivery::NoChange;
        };
        let request = RewardClaimMintRequest {
            backpack_outputs: false,
            additional_items: Vec::new(),
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
            root_fluid_subtype: None,
            root_charges_remaining: None,
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
                request,
                &NativeItemWeights(native),
            )
            .await;
        match outcome {
            Ok(_) => {
                self.states.loop_ground.invalidate();
                GrizzlyRewardDelivery::RefreshInventory
            }
            Err(RewardClaimMintError::Unavailable(
                crate::durability::DurabilityError::RootUnavailable,
            )) => GrizzlyRewardDelivery::NoChange,
            Err(RewardClaimMintError::Unavailable(_)) => {
                // An ambiguous mint can have changed custody. Read the authoritative inventory.
                self.states.loop_ground.invalidate();
                GrizzlyRewardDelivery::RefreshInventory
            }
            Err(RewardClaimMintError::Refused(
                RewardClaimRefusal::MainBackpackFull
                | RewardClaimRefusal::CarryingCapacity
                | RewardClaimRefusal::NoMainBackpack,
            )) => GrizzlyRewardDelivery::NoRoom,
            Err(_) => GrizzlyRewardDelivery::NoChange,
        }
    }
}
