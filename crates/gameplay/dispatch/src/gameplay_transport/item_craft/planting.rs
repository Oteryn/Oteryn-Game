//! W19b seed USE stays on the ordinary craft / consume-transform path.
use super::*;
use crate::durability::consume_transform::ConsumeTransformRequest;
use oteryn_gameplay_items::event_planting::{self as rules, SeedTrigger};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_planting(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        let Some(native) = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        else {
            return Ok(None);
        };
        let permit = self.spell_lane_permit().await.ok_or(())?;
        let mut request = ConsumeTransformRequest {
            command: CommandRef::new(
                command.game_session_id,
                CommandId::new(command.command_id).map_err(|_| ())?,
            ),
            source: [0; 16],
            target: None,
            intent: oteryn_protocol_oteryn::world_object::encode_item_use_intent(intent)
                .map_err(|_| ())?,
        };
        let weights = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        let outcome = |disposition| {
            Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            })
        };
        {
            let states = self.states.spell_states.lock().await;
            if let Some(memo) = states.pending_item_effects.iter().find(|memo| {
                memo.actor == actor
                    && memo.request.command == request.command
                    && memo.request.intent == request.intent
            }) {
                request.source = memo.request.source;
                request.target = memo.request.target;
            }
        }
        // An exact historical use is answered before its consumed seed/pot handles resolve.
        if let Some(result) = self
            .root
            .consume_transform_detailed(
                &permit,
                self.character,
                self.holder,
                fence,
                request.clone(),
                native,
                &weights,
                true,
            )
            .await
            .map_err(|_| ())?
        {
            if result
                .recipe
                .as_deref()
                .is_some_and(|recipe| recipe.starts_with("oteryn:planting."))
            {
                return Ok(outcome(result.disposition));
            }
            return self
                .finish_novelty_item(actor, &request, result)
                .await
                .map(Some);
        }
        let Some(ItemKey::Instance(seed)) = source else {
            return Ok(None);
        };
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .map_err(|_| ())?;
        let entries = inventory
            .backpack
            .iter()
            .flat_map(|bag| bag.entries.iter().map(|entry| &entry.item))
            .chain(inventory.nested.iter().map(|(_, entry)| &entry.item));
        let Some(source) = entries.clone().find(|entry| entry.item_instance_id == seed) else {
            return Ok(None);
        };
        let source_key = &source.definition.production_key;
        if source_key != "oteryn:item.tibia.i13844" {
            return Ok(None);
        }
        let Some(ItemKey::Instance(pot)) = target else {
            return Ok(None);
        };
        request.source = pot;
        request.target = Some(seed);
        let Some(target) = entries.clone().find(|entry| entry.item_instance_id == pot) else {
            return self
                .use_ground_planting(actor, command, &request, &permit)
                .await;
        };
        let target_id = target
            .definition
            .production_key
            .strip_prefix("oteryn:item.tibia.i")
            .and_then(|id| id.parse().ok());
        let Some(_) = target_id
            .and_then(|id| rules::seed_action(rules::ICE_FLOWER_SEEDS, id, SeedTrigger::Use))
        else {
            return Ok(outcome(UseDisposition::NothingToUse));
        };
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(seed))
            || !self
                .states
                .player_trades
                .physical(command.game_session_id, Some(pot))
        {
            return Ok(outcome(UseDisposition::Rejected));
        }
        {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if runtime.owner_fence().is_err()
                || !runtime
                    .player_control_facts(actor, command.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
                || pinned_index(&runtime, self.active_generation).is_none()
                || states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, command.game_session_id)
            {
                return Ok(outcome(UseDisposition::Rejected));
            }
        }
        request.source = pot;
        request.target = Some(seed);
        self.root
            .consume_transform_detailed(
                &permit,
                self.character,
                self.holder,
                fence,
                request,
                native,
                &weights,
                false,
            )
            .await
            .map(|result| result.and_then(|result| outcome(result.disposition)))
            .map_err(|_| ())
    }
}

use crate::durability::spell_owner_commit::SpellLanePermit;

impl ComposedFreshAdmission<'_, '_, '_> {
    async fn use_ground_planting(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        request: &ConsumeTransformRequest,
        permit: &SpellLanePermit,
    ) -> Result<Option<UseOutcome>, ()> {
        use crate::durability::consume_transform::GroundActionContext;
        let outcome = |disposition| {
            Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            })
        };
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return Ok(None);
        };
        let Some(seed) = request.target else {
            return Ok(None);
        };
        let Some(source) = self
            .root
            .read_ground_action_source(self.character, fence, request.source)
            .await
            .map_err(|_| ())?
        else {
            return Ok(outcome(UseDisposition::StaleState));
        };
        if !matches!(
            source.definition().production_key.as_str(),
            "oteryn:item.tibia.i306" | "oteryn:item.tibia.i14029"
        ) {
            return Ok(outcome(UseDisposition::NothingToUse));
        }
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(seed))
            || !self
                .states
                .player_trades
                .physical(command.game_session_id, Some(request.source))
        {
            return Ok(outcome(UseDisposition::Rejected));
        }
        let Some(room) = self.rooms().gate_ground_item_action() else {
            return Ok(outcome(UseDisposition::NotPossible));
        };
        let context = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, command.game_session_id)
                || runtime.owner_fence().is_err()
                || !runtime
                    .player_control_facts(actor, command.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
                || pinned_index(&runtime, self.active_generation).is_none()
            {
                return Ok(outcome(UseDisposition::Rejected));
            }
            // Donor POFF remains captured in the receipt. Capability23 has no live
            // consumer; it must not reserve or enqueue an undrainable presentation.
            GroundActionContext::from_native_owner(
                room,
                &runtime,
                actor,
                command.game_session_id,
                source,
            )
            .map_err(|_| ())?
        };
        let receipt = self
            .root
            .consume_transform_on_ground(
                permit,
                self.character,
                self.holder,
                fence,
                request.clone(),
                native,
                &NativeItemWeights(native).with_wheel(self.premium_coordinator),
                &context,
            )
            .await
            .map_err(|_| ())?
            .ok_or(())?;
        if receipt.disposition == UseDisposition::Committed {
            self.states.loop_ground.invalidate();
        }
        Ok(outcome(receipt.disposition))
    }
}
