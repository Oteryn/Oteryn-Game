//! S06 static machine targets and construction pieces through the audited item owner.
use super::{
    ComposedFreshAdmission,
    connection::{UseCommand, UseOutcome},
    item_view::ItemKey,
};
use crate::content::native_gameplay::mount_taming::rules;
use crate::durability::consume_transform::{ConsumeTransformRequest, MountKitMapTarget};
use crate::foundation::ExactActorRef;
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_protocol_oteryn::world_object::{MapUseTarget, encode_item_use_intent};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_mount_kit(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        let Some(native) = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        else {
            return Ok(None);
        };
        let Some(index) = native.item_index() else {
            return Ok(None);
        };
        if let UseTarget::ItemByDefinition(reference) = intent.target {
            let Some((key, _)) = index.definition_at(reference) else {
                return Ok(None);
            };
            if !rules()
                .iter()
                .any(|r| r.item_key == key && r.target_item_key.is_some())
            {
                return Ok(None);
            }
        }
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return Ok(None);
        };
        let mut request = ConsumeTransformRequest {
            command: CommandRef::new(
                command.game_session_id,
                CommandId::new(command.command_id).map_err(|_| ())?,
            ),
            source: [0; 16],
            target: None,
            intent: encode_item_use_intent(intent).map_err(|_| ())?,
        };
        let weights = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        let outcome = |disposition| UseOutcome {
            disposition,
            ..UseOutcome::rejected()
        };
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
            .map_err(|error| {
                let _ = error;
            })?
        {
            return Ok(Some(outcome(result.disposition)));
        }
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .map_err(|error| {
                let _ = error;
            })?;
        let source = match (intent.target.clone(), source) {
            (UseTarget::ItemByDefinition(reference), _) => {
                let (key, revision) = index.definition_at(reference).ok_or(())?;
                inventory
                    .first_unreserved(
                        &crate::durability::item_mint::TypedDefinitionRef {
                            family: "Item".into(),
                            production_key: key.into(),
                            revision_ref: revision.into(),
                        },
                        &[],
                    )
                    .ok()
            }
            (UseTarget::Item(_), Some(ItemKey::Instance(id))) => Some(id),
            _ => None,
        };
        let Some(source) = source else {
            return Ok(None);
        };
        let Some(item) = inventory
            .backpack
            .iter()
            .flat_map(|b| b.entries.iter().map(|e| &e.item))
            .chain(inventory.nested.iter().map(|(_, e)| &e.item))
            .find(|i| i.item_instance_id == source)
        else {
            return Ok(None);
        };
        let Some(rule) = rules()
            .iter()
            .find(|r| r.item_key == item.definition.production_key && r.target_item_key.is_some())
        else {
            return Ok(None);
        };
        if intent.use_with.is_some()
            || intent.use_with_position.is_some()
            || (rule.self_use && intent.use_with_map_item.is_some())
        {
            return Ok(Some(outcome(UseDisposition::NothingToUse)));
        }
        if !self
            .states
            .player_trades
            .physical_service(command.game_session_id)
        {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        request.source = source;
        request.target = match target {
            Some(ItemKey::Instance(id)) => Some(id),
            _ => None,
        };
        let map_target = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|f| f.control_loss.is_none())
                || states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, command.game_session_id)
            {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            if let Some(MapUseTarget::Object(target)) = &intent.use_with_map_item {
                let Some(map) = self.bundle_map() else {
                    return Ok(Some(outcome(UseDisposition::NothingToUse)));
                };
                let facts = map.facts.pinned();
                let source = super::world_map::MapViewSource {
                    overlay: &map.overlay,
                    facts: &facts,
                    content_generation: runtime.content_pin().client_artifact_digest(),
                    reset_epoch: self.reset_epoch,
                };
                let selected =
                    match super::world_map::resolve_map_target(&source, target, |_| None::<()>) {
                        Ok(selected) => selected,
                        Err(_) => return Ok(Some(outcome(UseDisposition::StaleState))),
                    };
                let tile = crate::map::view::compose_tile(&map.overlay, &facts, selected.pos)
                    .map_err(|_| ())?
                    .ok_or(())?;
                let Some(entry) = tile.entries.iter().find(|e| {
                    e.source.key() == crate::map::view::MapEntryKey::Base(selected.placement_key)
                }) else {
                    return Ok(Some(outcome(UseDisposition::NothingToUse)));
                };
                let key = format!("oteryn:item.tibia.i{}", entry.facts.appearance_id);
                if rule.target_item_key.as_deref() != Some(key.as_str())
                    || entry.facts.bound
                    || entry.facts.count != 1
                    || entry.facts.sub_type != 0
                {
                    return Ok(Some(outcome(UseDisposition::NothingToUse)));
                }
                let position = crate::foundation::MovementLocalPosition {
                    x: i32::from(selected.pos.x),
                    y: i32::from(selected.pos.y),
                    floor: -i16::from(selected.pos.floor),
                };
                match MountKitMapTarget::from_native_owner(
                    &runtime,
                    actor,
                    command.game_session_id,
                    position,
                    entry.facts.appearance_id,
                    target.clone(),
                    native.source_digest(),
                ) {
                    Ok(proof) => Some(proof),
                    Err(_) => return Ok(Some(outcome(UseDisposition::TooFar))),
                }
            } else {
                None
            }
        };
        if !rule.self_use && map_target.is_none() && request.target.is_none() {
            return Ok(Some(outcome(UseDisposition::NothingToUse)));
        }
        let result = self
            .root
            .consume_mount_kit(
                &permit,
                self.character,
                self.holder,
                fence,
                request,
                native,
                &weights,
                map_target.as_ref(),
            )
            .await
            .map_err(|_| ())?
            .ok_or(())?;
        if result.fresh
            && result.disposition == UseDisposition::Committed
            && let Some(feedback) = result
                .effects
                .get("cosmetic")
                .and_then(|e| e.get("feedback"))
                .and_then(serde_json::Value::as_str)
        {
            self.states
                .chat
                .push_quest_feedback(command.game_session_id, feedback);
        }
        Ok(Some(outcome(result.disposition)))
    }
}
