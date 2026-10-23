//! MINING-1: real USE adapter for the Reference pick's shiny-stone refining.
//! The original bounded intent binds the receipt; an unknown commit closes the
//! connection without sequencing a false terminal refusal.

use super::ComposedFreshAdmission;
use super::connection::{FreshAdmissionAuthority, UseCommand, UseOutcome};
use super::item_ref_admission::{NativeItemWeights, pinned_index};
use super::item_view::{ItemKey, SessionItemView};
use crate::durability::character_authority::SERVER_BUILD_ID;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_use::{ItemUseFlaskPlan, ItemUseKind, ItemUseRequest};
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use oteryn_gameplay_items::mining::{PICK, refined_item};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

pub(super) mod offer;

pub(super) async fn dispatch<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    command: UseCommand,
    intent: UseIntent,
    view: Option<&SessionItemView>,
) -> Option<UseOutcome> {
    let source = match intent.target {
        UseTarget::Item(handle) => view.and_then(|v| v.resolve(handle)),
        _ => None,
    };
    let target = match intent.use_with_map_item {
        Some(MapUseTarget::Item(handle)) => view.and_then(|v| v.resolve(handle)),
        _ => None,
    };
    authority
        .use_pick(actor, command, intent, source, target)
        .await
}

fn outcome(disposition: UseDisposition) -> UseOutcome {
    UseOutcome {
        disposition,
        ..UseOutcome::rejected()
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn use_pick_impl(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Option<UseOutcome> {
        match self.refine(actor, command, intent, source, target).await {
            Ok(value) => value.map(outcome),
            Err(disposition) => Some(outcome(disposition)),
        }
    }

    async fn refine(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Result<Option<UseDisposition>, UseDisposition> {
        let fence = command.item_fence.ok_or(UseDisposition::Rejected)?;
        let command_ref = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).map_err(|_| UseDisposition::Rejected)?,
        );
        let encoded = encode_item_use_intent(&intent).map_err(|_| UseDisposition::Rejected)?;
        let binding: [u8; 32] = Sha256::digest(&encoded).into();
        let _permit = self
            .spell_lane_permit()
            .await
            .ok_or(UseDisposition::Rejected)?;
        {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|f| f.control_loss.is_none())
                || states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, command.game_session_id)
            {
                return Err(UseDisposition::Rejected);
            }
        }
        match self
            .root
            .read_pick_refining_receipt(self.character, fence, command_ref, binding)
            .await
        {
            Ok(Some(_)) => return Ok(Some(UseDisposition::Committed)),
            Ok(None) => {}
            Err(crate::durability::item_use::ItemUseError::Unavailable(_)) => return Ok(None),
            Err(_) => return Err(UseDisposition::Rejected),
        }
        let native = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
            .ok_or(UseDisposition::NothingToUse)?;
        let index = {
            let runtime = self.runtime.lock().await;
            pinned_index(&runtime, self.active_generation).ok_or(UseDisposition::Rejected)?
        };
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .map_err(|_| UseDisposition::Rejected)?;
        let bag = inventory
            .backpack
            .as_ref()
            .ok_or(UseDisposition::StaleState)?;
        let tool = match intent.target {
            UseTarget::ItemByDefinition(wanted) => {
                let (key, revision) = index
                    .definition_at(wanted)
                    .ok_or(UseDisposition::NothingToUse)?;
                inventory
                    .first_unreserved(
                        &TypedDefinitionRef {
                            family: "Item".into(),
                            production_key: key.into(),
                            revision_ref: revision.into(),
                        },
                        &[],
                    )
                    .map_err(|_| UseDisposition::NothingToUse)?
            }
            UseTarget::Item(_) => match source {
                Some(ItemKey::Instance(id)) => id,
                _ => return Err(UseDisposition::StaleState),
            },
            _ => return Err(UseDisposition::NothingToUse),
        };
        let Some(ItemKey::Instance(target)) = target else {
            return Err(UseDisposition::NothingToUse);
        };
        let entries = bag
            .entries
            .iter()
            .chain(inventory.nested.iter().map(|(_, e)| e));
        let pick = entries
            .clone()
            .find(|e| e.item.item_instance_id == tool)
            .ok_or(UseDisposition::StaleState)?;
        let stone = entries
            .clone()
            .find(|e| e.item.item_instance_id == target)
            .ok_or(UseDisposition::StaleState)?;
        if pick.item.definition.production_key != PICK {
            return Err(UseDisposition::NothingToUse);
        }
        // Occurrence is the target instance and remaining quantity, rather than the
        // client-chosen command id: changing a command cannot reroll a refused attempt.
        let mut occurrence = [0; 16];
        let digest = Sha256::new()
            .chain_update(b"oteryn.pick.refine.v1")
            .chain_update(target)
            .chain_update(stone.item.quantity.to_be_bytes())
            .finalize();
        occurrence.copy_from_slice(&digest[..16]);
        let root = GameplayDecisionRoot::from_bytes(native.source_digest());
        let draw = deterministic_decision_u64(
            &root,
            DecisionOccurrenceId::from_bytes(occurrence),
            "pick.refine.draw",
            0,
        )
        .map_err(|_| UseDisposition::Rejected)?;
        let percentile = u8::try_from(crate::spell::uniform_draw(draw, 1, 100))
            .map_err(|_| UseDisposition::Rejected)?;
        let key = refined_item(
            &pick.item.definition.production_key,
            &stone.item.definition.production_key,
            percentile,
        )
        .ok_or(UseDisposition::NothingToUse)?;
        let revision = index.revision(key).ok_or(UseDisposition::NothingToUse)?;
        let output = TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: revision.into(),
        };
        let flask = inventory
            .plan_flask(stone, output, || crate::operator_files::uuid_v7().ok())
            .map_err(|_| UseDisposition::Rejected)?;
        if matches!(flask, ItemUseFlaskPlan::Missing | ItemUseFlaskPlan::None) {
            return Err(UseDisposition::Full);
        }
        let request = ItemUseRequest {
            kind: ItemUseKind::PickRefine {
                tool,
                wire_binding: binding,
                percentile,
            },
            command_id: command.command_id,
            content_revision: inventory.content_revision,
            ruleset_revision: inventory.ruleset_revision,
            sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
            transaction_id: crate::operator_files::uuid_v7()
                .map_err(|_| UseDisposition::Rejected)?,
            event_id: crate::operator_files::uuid_v7().map_err(|_| UseDisposition::Rejected)?,
            occurred_at_unix_ms: i64::try_from(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|_| UseDisposition::Rejected)?
                    .as_millis(),
            )
            .map_err(|_| UseDisposition::Rejected)?,
            server_build_id: SERVER_BUILD_ID.into(),
            source_item_instance_id: target,
            source_definition: stone.item.definition.clone(),
            source_quantity_before: stone.item.quantity,
            flask,
        };
        let content = NativeItemWeights(native);
        match self
            .root
            .commit_pick_refining(self.character, self.holder, fence, request, &content)
            .await
        {
            Ok(_) => {
                #[cfg(test)]
                if test_faults::take() {
                    return Ok(None);
                }
                Ok(Some(UseDisposition::Committed))
            }
            Err(crate::durability::item_use::ItemUseError::Unavailable(error)) => {
                #[cfg(test)]
                eprintln!("mining conversion unavailable: {error:?}");
                let _ = error;
                Ok(None)
            }
            Err(crate::durability::item_use::ItemUseError::StaleState) => {
                Err(UseDisposition::StaleState)
            }
            Err(crate::durability::item_use::ItemUseError::ConflictingCommand) => {
                match self
                    .root
                    .read_pick_refining_receipt(self.character, fence, command_ref, binding)
                    .await
                {
                    Ok(Some(_)) => Ok(Some(UseDisposition::Committed)),
                    Err(crate::durability::item_use::ItemUseError::Unavailable(_)) => Ok(None),
                    _ => Err(UseDisposition::Rejected),
                }
            }
            Err(error) => {
                #[cfg(test)]
                eprintln!("mining conversion refused: {error:?}");
                let _ = error;
                Err(UseDisposition::Rejected)
            }
        }
    }
}

#[cfg(test)]
pub(super) mod test_faults {
    use std::cell::Cell;
    thread_local! { static LOSE_ACK: Cell<bool> = const { Cell::new(false) }; }
    pub(in crate::gameplay_transport) fn lose_next_ack() {
        LOSE_ACK.set(true);
    }
    pub(super) fn take() -> bool {
        LOSE_ACK.replace(false)
    }
}
