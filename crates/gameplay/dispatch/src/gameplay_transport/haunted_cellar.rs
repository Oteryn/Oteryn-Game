//! Carried bucket USE on a native map egg.
use super::{
    ComposedFreshAdmission,
    connection::{UseCommand, UseOutcome},
    item_ref_admission::NativeItemWeights,
    item_view::ItemKey,
    world_map::{MapHandleTarget, MapViewSource, resolve_map_handle, resolve_map_target},
};
use crate::{
    durability::consume_transform::ConsumeTransformRequest,
    foundation::{CommandId, CommandRef, ExactActorRef},
    map::{facts::QuestCallbackProjection, view::MapFacts},
};
use oteryn_gameplay_items::{haunted_cellar as rules, reward_bag::reward_bag_bounded_draw};
use oteryn_protocol_oteryn::{
    world_map::MapDefinition,
    world_object::{MapUseTarget, UseDisposition, UseIntent, encode_item_use_intent},
};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_haunted_cellar_egg(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        let (Some(fence), Some(ItemKey::Instance(source)), Some(use_with)) = (
            command.item_fence,
            source,
            intent.use_with_map_item.as_ref(),
        ) else {
            return Ok(None);
        };
        let (Some(map), Some(active)) = (self.bundle_map(), self.active_generation) else {
            return Ok(None);
        };
        let Some(native) = active.native_gameplay() else {
            return Ok(None);
        };
        let Some(index) = native.item_index() else {
            return Ok(None);
        };
        let Some(egg) = index.definition_ref("Item", "oteryn:item.tibia.i29305", "definition-r1")
        else {
            return Ok(None);
        };
        let view = MapViewSource {
            overlay: &map.overlay,
            facts: &map.facts,
            content_generation: active.identity().client_artifact_digest(),
            reset_epoch: self.reset_epoch,
        };
        let (position, ordinal, key, revision) = match use_with {
            MapUseTarget::Object(object) => {
                let Ok(resolved) = resolve_map_target(&view, object, |_| None::<()>) else {
                    return Ok(None);
                };
                (
                    resolved.pos,
                    resolved.ordinal,
                    resolved.placement_key,
                    object.expected_revision,
                )
            }
            MapUseTarget::Item(_) => {
                let Some(target) = target else {
                    return Ok(None);
                };
                let Ok(MapHandleTarget::Base {
                    pos,
                    ordinal,
                    placement_key,
                }) = resolve_map_handle(&view, target)
                else {
                    return Ok(None);
                };
                (
                    pos,
                    ordinal,
                    placement_key,
                    map.facts.object_revision(placement_key),
                )
            }
        };
        let Some(base) = map
            .overlay
            .base()
            .tile(position.x, position.y, position.floor)
        else {
            return Ok(None);
        };
        let before = base
            .ids()
            .iter()
            .zip(base.depths())
            .filter(|(_, depth)| **depth == 0)
            .nth(usize::from(ordinal))
            .and_then(|(id, _)| map.facts.base_entry(position, ordinal, *id));
        let Some(before) = before.filter(|entry| entry.definition == MapDefinition::Item(egg))
        else {
            return Ok(None);
        };
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .map_err(|_| ())?;
        let bucket = inventory
            .backpack
            .iter()
            .flat_map(|bag| bag.entries.iter().map(|entry| &entry.item))
            .chain(inventory.nested.iter().map(|(_, entry)| &entry.item))
            .find(|item| item.item_instance_id == source)
            .and_then(|item| {
                item.definition
                    .production_key
                    .strip_prefix("oteryn:item.tibia.i")
            })
            .and_then(|id| id.parse().ok());
        if !bucket.is_some_and(rules::is_bucket) {
            return Ok(None);
        }
        let outcome = |disposition| {
            Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            })
        };
        let normal_egg_key = format!("oteryn:item.tibia.i{}", rules::NORMAL_EGG);
        let normal_egg_appearance = crate::map::view::appearance_id(
            &normal_egg_key,
            crate::map::palette_appearance(&normal_egg_key).map(u32::from),
        );
        let Some(after) = map
            .facts
            .quest_callback_replacement(key, normal_egg_appearance)
        else {
            return Ok(outcome(UseDisposition::NotPossible));
        };
        let permit = self.spell_lane_permit().await.ok_or(())?;
        let mut runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        if !runtime
            .player_control_facts(actor, command.game_session_id)
            .is_ok_and(|facts| facts.control_loss.is_none())
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || states.is_dead(actor)
            || map.overlay.runtime_scope() != runtime.binding().runtime_scope()
            || map.facts.house_tile(position)
            || !map.facts.quest_callback_capacity(key, revision, 8)
            || !self
                .states
                .player_trades
                .physical(command.game_session_id, Some(source))
        {
            return Ok(outcome(UseDisposition::Rejected));
        }
        let at = runtime
            .read_actor_position(actor)
            .map_err(|_| ())?
            .position();
        if !oteryn_gameplay_items::reach::within_reach(
            (at.x, at.y, i32::from(at.floor)),
            (
                i32::from(position.x),
                i32::from(position.y),
                -i32::from(position.floor),
            ),
        ) {
            return Ok(outcome(UseDisposition::TooFar));
        }
        let roll = reward_bag_bounded_draw(
            *command.game_session_id.as_bytes(),
            command.command_id,
            source,
            47,
            11,
        )
        .ok_or(())?;
        let request = ConsumeTransformRequest {
            command: CommandRef::new(
                command.game_session_id,
                CommandId::new(command.command_id).map_err(|_| ())?,
            ),
            source,
            target: None,
            intent: encode_item_use_intent(intent).map_err(|_| ())?,
        };
        let result = self
            .root
            .consume_transform_haunted_egg(
                &permit,
                self.character,
                self.holder,
                fence,
                request,
                native,
                &NativeItemWeights(native).with_wheel(self.premium_coordinator),
                roll as u32,
            )
            .await;
        let Ok(Some(receipt)) = result else {
            runtime.retire_owner_cycle();
            return Err(());
        };
        if receipt.disposition != UseDisposition::Committed {
            return Ok(outcome(receipt.disposition));
        }
        if receipt.recipe.as_deref() != Some("oteryn:haunted-cellar-egg.v1") {
            return Ok(outcome(UseDisposition::Rejected));
        }
        if !receipt.fresh {
            return Ok(outcome(UseDisposition::Committed));
        }
        let delay = receipt.effects["restore_after_ms"].as_u64().ok_or(())?;
        let projection = QuestCallbackProjection::TimedTransform {
            due: self
                .owner_now()
                .get()
                .checked_add(delay * 1_000)
                .ok_or(())?,
            restore: before,
        };
        if !map
            .facts
            .apply_quest_callback(key, revision, after, &[], &[], false, Some(&projection))
        {
            runtime.retire_owner_cycle();
            return Err(());
        }
        states.owner_wake.notify_one();
        if let Some(bucket) = receipt.effects["filled_bucket"].as_u64() {
            let feedback = if bucket == u64::from(rules::FULL_BUCKET) {
                "The ectoplasm was already seeping into the cocoon. You manage to fill the bucket."
            } else {
                "There is not enough ectoplasm left to fill it in the bucket."
            };
            self.states
                .chat
                .push_quest_feedback(command.game_session_id, feedback);
        }
        Ok(outcome(UseDisposition::Committed))
    }
}
