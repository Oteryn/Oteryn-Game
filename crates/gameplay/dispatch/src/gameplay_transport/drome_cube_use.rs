//! W09: the existing item-use owner handles Cube USE, including retained unknown commits.
//! Historical receipts precede live handles. Every new/retried write re-proves current
//! authority; a saved PREPARE request is data, never a replacement for the current fences.

use crate::durability::character_authority::SERVER_BUILD_ID;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_use::drome_cube::DromeCubeError;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_use::{
    ItemUseError, ItemUseFlaskPlan, ItemUseKind, ItemUseRequest,
};
use crate::foundation::{CommandId, CommandRef, ExactActorRef, GameSessionId};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{UseCommand, UseOutcome};
use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, pinned_index};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_gameplay_items::drome_cube::{DROME_CUBE_KEY, reward, reward_index};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};
use sha2::{Digest, Sha256};

#[path = "drome_cube_use/gate.rs"]
mod gate;
pub(super) use gate::PendingCubeUse;
#[path = "drome_cube_use/ports.rs"]
mod ports;
pub(in crate::gameplay_transport) use ports::{authority_methods, composed_methods};

fn outcome(disposition: UseDisposition) -> UseOutcome {
    UseOutcome {
        disposition,
        ..UseOutcome::rejected()
    }
}

fn refusal(error: &DromeCubeError) -> UseDisposition {
    match error {
        DromeCubeError::Capacity => UseDisposition::Full,
        DromeCubeError::ItemUse(ItemUseError::StaleState) => UseDisposition::StaleState,
        _ => UseDisposition::Rejected,
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// None delegates an ordinary item. Err means no terminal result may be acknowledged.
    pub(in crate::gameplay_transport) async fn use_drome_cube_impl(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: UseIntent,
        source: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        if intent.use_with.is_some()
            || intent.use_with_map_item.is_some()
            || intent.use_with_position.is_some()
            || !matches!(intent.target, UseTarget::Item(_) | UseTarget::ItemByDefinition(_))
        {
            return Ok(None);
        }
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        let command_ref = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).map_err(|_| ())?,
        );
        let binding: [u8; 32] =
            Sha256::digest(encode_item_use_intent(&intent).map_err(|_| ())?).into();
        let _permit = self.spell_lane_permit().await.ok_or(())?;
        match self
            .root
            .read_drome_cube_receipt(self.character, fence, command_ref, binding)
            .await
        {
            Ok(Some(receipt)) => {
                if self.finish_cube(actor, command, UseDisposition::Committed).await {
                    self.cube_notice(command.game_session_id, receipt.reward_index);
                }
                return Ok(Some(outcome(UseDisposition::Committed)));
            }
            Ok(None) => {}
            Err(ItemUseError::Unavailable(_)) => return Err(()),
            Err(_) => return Ok(Some(outcome(UseDisposition::Rejected))),
        }
        let pending = {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            states
                .item_uses
                .gate(&runtime, actor, command.game_session_id)
                .and_then(|gate| gate.cube_pending.clone())
        };
        let Some(native) = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        else {
            return if pending.is_some() { Err(()) } else { Ok(None) };
        };
        let content = NativeItemWeights(native);
        if let Some(pending) = pending.as_ref()
            && pending.request.command_id == command.command_id
        {
            if pending.binding() != Some(binding) {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            // Retry the original UUIDs/quantities/output, never re-PREPARE the next unit.
            // The writer still receives the independently current command fence and content.
            if !self.cube_actor_live(actor, command.game_session_id).await {
                return Err(());
            }
            return self
                .commit_cube_attempt(actor, command, pending.request.clone(), &content)
                .await
                .map(|result| Some(outcome(result)));
        }
        let index = {
            let runtime = self.runtime.lock().await;
            pinned_index(&runtime, self.active_generation).ok_or(())?
        };
        let requested = if let UseTarget::ItemByDefinition(wanted) = intent.target {
            let Some((key, revision)) = index.definition_at(wanted) else {
                return Ok(None);
            };
            if key != DROME_CUBE_KEY {
                return Ok(None);
            }
            Some(TypedDefinitionRef {
                family: "Item".into(),
                production_key: key.into(),
                revision_ref: revision.into(),
            })
        } else {
            None
        };
        let inventory = match self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
        {
            Ok(inventory) => inventory,
            Err(ItemUseError::Unavailable(_)) => return Err(()),
            Err(_) => return Ok(Some(outcome(UseDisposition::Rejected))),
        };
        let Some(bag) = inventory.backpack.as_ref() else {
            return Ok(None);
        };
        let carried = |id| {
            bag.entries
                .iter()
                .chain(inventory.nested.iter().map(|(_, entry)| entry))
                .find(|entry| entry.item.item_instance_id == id)
        };
        if requested.is_none()
            && !matches!(source, Some(ItemKey::Instance(id))
                if carried(id).is_some_and(|entry| entry.item.definition.production_key == DROME_CUBE_KEY))
        {
            return Ok(None);
        }
        if pending.is_some() {
            // One bounded retained Cube attempt per actor; do not overwrite an unknown one.
            return Ok(Some(outcome(UseDisposition::Exhausted)));
        }
        if !self.cube_actor_live(actor, command.game_session_id).await {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        let reserved = {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            let gate = states
                .item_uses
                .gate(&runtime, actor, command.game_session_id)
                .ok_or(())?;
            if let Err(disposition) = gate.admit(command.command_id, self.owner_ms()) {
                return Ok(Some(outcome(disposition)));
            }
            gate.unspendable.clone()
        };
        let source_id = match requested.as_ref() {
            Some(definition) => match inventory.first_unreserved(definition, &reserved) {
                Ok(id) => id,
                Err(_) => return Ok(Some(outcome(UseDisposition::NothingToUse))),
            },
            None => match source {
                Some(ItemKey::Instance(id)) => id,
                _ => return Ok(Some(outcome(UseDisposition::StaleState))),
            },
        };
        let Some(entry) = carried(source_id) else {
            return Ok(Some(outcome(UseDisposition::StaleState)));
        };
        if reserved.contains(&source_id)
            || !self
                .states
                .player_trades
                .physical(command.game_session_id, Some(source_id))
        {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        let draw = reward_index(source_id, entry.item.quantity).ok_or(())?;
        let selected = reward(draw).ok_or(())?;
        let Some(revision) = index.revision(selected.production_key()) else {
            return Ok(Some(outcome(UseDisposition::NothingToUse)));
        };
        let output = TypedDefinitionRef {
            family: "Item".into(),
            production_key: selected.production_key().into(),
            revision_ref: revision.into(),
        };
        let (Some(facts), Some(capacity)) = (
            content.loop_facts(&output),
            content
                .loop_facts(&bag.backpack.definition)
                .and_then(|facts| facts.container_capacity),
        ) else {
            return Ok(Some(outcome(UseDisposition::NothingToUse)));
        };
        let flask = match crate::durability::item_use::drome_cube::plan_output(
            &inventory,
            entry,
            facts,
            capacity,
            &reserved,
            || crate::operator_files::uuid_v7().ok(),
        ) {
            Ok(plan) => plan,
            Err(error) => return Ok(Some(outcome(refusal(&error)))),
        };
        if let ItemUseFlaskPlan::Stack { item_instance_id, .. } = flask
            && !self
                .states
                .player_trades
                .physical(command.game_session_id, Some(item_instance_id))
        {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        let request = ItemUseRequest {
            kind: ItemUseKind::DromeCube { wire_binding: binding },
            command_id: command.command_id,
            content_revision: inventory.content_revision.clone(),
            ruleset_revision: inventory.ruleset_revision.clone(),
            sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
            transaction_id: crate::operator_files::uuid_v7().map_err(|_| ())?,
            event_id: crate::operator_files::uuid_v7().map_err(|_| ())?,
            occurred_at_unix_ms: super::unix_ms()?,
            server_build_id: SERVER_BUILD_ID.into(),
            source_item_instance_id: source_id,
            source_definition: entry.item.definition.clone(),
            source_quantity_before: entry.item.quantity,
            flask,
        };
        {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            let gate = states
                .item_uses
                .gate(&runtime, actor, command.game_session_id)
                .ok_or(())?;
            if let Err(disposition) = gate.prepare_cube(request.clone(), self.owner_ms()) {
                return Ok(Some(outcome(disposition)));
            }
        }
        self.commit_cube_attempt(actor, command, request, &content)
            .await
            .map(|result| Some(outcome(result)))
    }

    async fn cube_actor_live(&self, actor: ExactActorRef, session: GameSessionId) -> bool {
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        runtime.owner_fence().is_ok()
            && runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
            && !states.is_dead(actor)
            && states.spell_idle(&runtime, actor, session)
            && pinned_index(&runtime, self.active_generation).is_some()
    }

    /// Called only while holding the existing spell/item lane. A retained request is
    /// revalidated by the current fenced writer, even after a connection replacement.
    async fn commit_cube_attempt(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        request: ItemUseRequest,
        content: &NativeItemWeights<'_>,
    ) -> Result<UseDisposition, ()> {
        let fence = command.item_fence.ok_or(())?;
        let ItemUseKind::DromeCube { wire_binding } = request.kind else {
            return Err(());
        };
        let index = reward_index(request.source_item_instance_id, request.source_quantity_before)
            .ok_or(())?;
        let command_ref = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).map_err(|_| ())?,
        );
        let written = self
            .root
            .commit_drome_cube(self.character, self.holder, fence, request, content)
            .await;
        #[cfg(test)]
        if matches!(&written, Ok(crate::durability::item_use::ItemUseOutcome::Written { .. }))
            && test_faults::take_lost_ack()
        {
            // The storage acknowledgement is lost: keep the PREPARE and both reservations.
            self.mark_cube_unknown(actor, command).await;
            return Err(());
        }
        let disposition = match written {
            Ok(_) => UseDisposition::Committed,
            Err(error) => {
                match self
                    .root
                    .read_drome_cube_receipt(self.character, fence, command_ref, wire_binding)
                    .await
                {
                    Ok(Some(_)) => UseDisposition::Committed,
                    Err(ItemUseError::Unavailable(_)) => {
                        self.mark_cube_unknown(actor, command).await;
                        return Err(());
                    }
                    _ if matches!(error, DromeCubeError::ItemUse(ItemUseError::Unavailable(_))) => {
                        // No visible row after an unknown COMMIT does not prove rollback.
                        self.mark_cube_unknown(actor, command).await;
                        return Err(());
                    }
                    _ => refusal(&error),
                }
            }
        };
        if self.finish_cube(actor, command, disposition).await
            && disposition == UseDisposition::Committed
        {
            self.cube_notice(command.game_session_id, index);
        }
        Ok(disposition)
    }

    async fn mark_cube_unknown(&self, actor: ExactActorRef, command: UseCommand) {
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if let Some(gate) = states.item_uses.gate(&runtime, actor, command.game_session_id) {
            gate.mark_cube_unknown(command.command_id);
        }
    }

    async fn finish_cube(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        disposition: UseDisposition,
    ) -> bool {
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        states
            .item_uses
            .gate(&runtime, actor, command.game_session_id)
            .is_some_and(|gate| gate.finish_cube(command.command_id, disposition))
    }

    fn cube_notice(&self, session: GameSessionId, index: u8) {
        use crate::content::ReferenceItemField::Known;
        let Some(kind) = reward(index) else { return };
        let Some(native) = self.active_generation.and_then(crate::content::ActiveGeneration::native_gameplay) else {
            return;
        };
        let Some(revision) = native.item_index().and_then(|index| index.revision(kind.production_key())) else {
            return;
        };
        if let Some(policy) = native.item_policy(kind.production_key(), revision)
            && let Known(presentation) = &policy.record().semantics.presentation
            && let Known(name) = &presentation.name
        {
            let text = format!("You have received a {name}!");
            if text.len() <= 255 {
                self.states.tutorial_sessions.notice(
                    session,
                    oteryn_protocol_oteryn::tutorial_notices::TutorialNotice::StatusText(text),
                );
            }
        }
    }
}

#[cfg(test)]
pub(in crate::gameplay_transport) mod test_faults {
    use std::cell::Cell;
    thread_local! { static LOSE_ACK: Cell<bool> = const { Cell::new(false) }; }
    pub(crate) fn lose_next_ack() { LOSE_ACK.set(true); }
    pub(super) fn take_lost_ack() -> bool { LOSE_ACK.replace(false) }
}
