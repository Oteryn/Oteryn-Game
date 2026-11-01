//! W19F production USE routes for rust remover and lottery tickets.
use super::connection::{UseCommand, UseOutcome};
use super::item_ref_admission::NativeItemWeights;
use super::{ComposedFreshAdmission, item_view::ItemKey};
use crate::durability::consume_transform::ConsumeTransformRequest;
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent};

const RUST_REMOVER: &str = "oteryn:item.tibia.i9016";
const LOTTERY_TICKET: &str = "oteryn:item.tibia.i5957";

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_consume_transform(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Option<UseOutcome> {
        let native = self.active_generation?.native_gameplay()?;
        let fence = command.item_fence?;
        let command_id = CommandId::new(command.command_id).ok()?;
        let permit = self.spell_lane_permit().await?;
        let mut request = ConsumeTransformRequest {
            command: CommandRef::new(command.game_session_id, command_id),
            source: [0; 16],
            target: None,
            intent: oteryn_protocol_oteryn::world_object::encode_item_use_intent(&intent).ok()?,
        };
        let outcome = |disposition| UseOutcome {
            disposition,
            ..UseOutcome::rejected()
        };
        let content = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        match self
            .root
            .consume_transform(
                &permit,
                self.character,
                self.holder,
                fence,
                request.clone(),
                native,
                &content,
                true,
            )
            .await
        {
            Ok(Some(result)) => return Some(outcome(result)),
            Ok(None) => {}
            Err(_) => return None,
        }
        // A receipt outlives both input instances and their session view handles.
        let (ItemKey::Instance(source_id), ItemKey::Instance(target_id)) = (source?, target?)
        else {
            return None;
        };
        let backpack = self
            .root
            .read_character_backpack(self.character, fence.character_id)
            .await
            .ok()??;
        if !backpack.entries.iter().any(|entry| {
            entry.item.item_instance_id == source_id
                && entry.item.definition.production_key == RUST_REMOVER
        }) {
            return None;
        }
        request.source = target_id;
        request.target = Some(source_id);
        {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if states.is_dead(actor) || !states.spell_idle(&runtime, actor, command.game_session_id)
            {
                return Some(outcome(UseDisposition::Rejected));
            }
        }
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(source_id))
            || !self
                .states
                .player_trades
                .physical(command.game_session_id, Some(target_id))
        {
            return Some(outcome(UseDisposition::Rejected));
        }
        self.root
            .consume_transform(
                &permit,
                self.character,
                self.holder,
                fence,
                request,
                native,
                &content,
                false,
            )
            .await
            .ok()
            .flatten()
            .map(outcome)
    }

    pub(super) async fn use_lottery_ticket(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        item: ItemKey,
    ) -> Option<Option<UseOutcome>> {
        let ItemKey::Instance(id) = item else {
            return None;
        };
        let native = self.active_generation?.native_gameplay()?;
        let fence = command.item_fence?;
        let backpack = self
            .root
            .read_character_backpack(self.character, fence.character_id)
            .await
            .ok()??;
        let carried = backpack
            .entries
            .iter()
            .find(|entry| entry.item.item_instance_id == id);
        if carried.is_some_and(|entry| entry.item.definition.production_key != LOTTERY_TICKET) {
            return None;
        }
        let outcome = |disposition| {
            Some(Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            }))
        };
        let command_id = CommandId::new(command.command_id).ok()?;
        let permit = self.spell_lane_permit().await?;
        let request = ConsumeTransformRequest {
            command: CommandRef::new(command.game_session_id, command_id),
            source: id,
            target: None,
            intent: [b"oteryn:lottery-ticket-use:v1".as_slice(), id.as_slice()].concat(),
        };
        let content = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        match self
            .root
            .consume_transform(
                &permit,
                self.character,
                self.holder,
                fence,
                request.clone(),
                native,
                &content,
                true,
            )
            .await
        {
            Ok(Some(result)) => return outcome(result),
            Ok(None) if carried.is_none() => return None,
            Ok(None) => {}
            Err(_) => return Some(None),
        }
        {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if states.is_dead(actor) || !states.spell_idle(&runtime, actor, command.game_session_id)
            {
                return outcome(UseDisposition::Rejected);
            }
        }
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(id))
        {
            return outcome(UseDisposition::Rejected);
        }
        match self
            .root
            .consume_transform(
                &permit,
                self.character,
                self.holder,
                fence,
                request,
                native,
                &content,
                false,
            )
            .await
        {
            Ok(Some(result)) => outcome(result),
            _ => Some(None),
        }
    }
}
