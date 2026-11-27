//! Flask-on-dispenser is an ordinary USE_WITH, composed with the item action owner.
use super::*;
use crate::durability::consume_transform::{ConsumeTransformRequest, ConsumeTransformResult};
use crate::durability::item_loop_move::LoopGround;
use crate::durability::map_item_mint::{MAP_DISPENSER_KEY, MapItemMintError, MapItemMintRequest};
use crate::foundation::{ChannelRuntimeV1, GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::world_map::{
    MapHandleTarget, MapViewSource, resolve_map_handle, resolve_map_target,
};
use oteryn_gameplay_items::item_ref_admission::generation_ground_item_facts;

#[cfg(test)]
tokio::task_local! {
    pub(in crate::gameplay_transport) static DISPENSER_ACK_BARRIER: std::sync::Arc<tokio::sync::Notify>;
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn use_dispenser(
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
        let Some(native) = self.active_generation.and_then(|g| g.native_gameplay()) else {
            return Ok(None);
        };
        let permit = self.spell_lane_permit().await.ok_or(())?;
        let weights = NativeItemWeights(native).with_wheel(self.premium_coordinator);
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
        let outcome = |disposition| {
            Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            })
        };
        if matches!(
            fence.runtime_scope,
            crate::foundation::RuntimeScopeRefV1::Instance { .. }
        ) {
            use crate::durability::consume_transform::HouseActionContext;
            let context = {
                let runtime = self.runtime.lock().await;
                if pinned_index(&runtime, self.active_generation).is_none() {
                    return Ok(outcome(UseDisposition::StaleState));
                }
                HouseActionContext::from_native_owner(
                    &runtime,
                    actor,
                    command.game_session_id,
                    fence,
                )
                .map_err(|_| ())?
            };
            if let Some(receipt) = self
                .root
                .consume_dispenser_in_house(
                    &permit,
                    self.character,
                    self.holder,
                    &context,
                    request.clone(),
                    native,
                    &weights,
                    true,
                )
                .await
                .map_err(|_| ())?
            {
                return Ok(
                    if receipt.recipe.as_deref() == Some("oteryn:dispenser.fill") {
                        outcome(receipt.disposition)
                    } else {
                        None
                    },
                );
            }
            let (Some(ItemKey::Instance(flask)), Some(ItemKey::Instance(dispenser))) =
                (source, target)
            else {
                return Ok(None);
            };
            if flask == dispenser {
                return Ok(None);
            }
            if !self
                .states
                .player_trades
                .physical(command.game_session_id, Some(flask))
                || !self
                    .states
                    .player_trades
                    .physical(command.game_session_id, Some(dispenser))
            {
                return Ok(outcome(UseDisposition::Rejected));
            }
            request.source = flask;
            request.target = Some(dispenser);
            let Some(receipt) = self
                .root
                .consume_dispenser_in_house(
                    &permit,
                    self.character,
                    self.holder,
                    &context,
                    request,
                    native,
                    &weights,
                    false,
                )
                .await
                .map_err(|_| ())?
            else {
                return Ok(None);
            };
            if receipt.disposition == UseDisposition::NothingToUse {
                return Ok(None);
            }
            if receipt.fresh && receipt.disposition == UseDisposition::Committed {
                self.states.loop_ground.invalidate();
                if let Some(charges) = receipt.effects["dispenser"]["after"].as_u64() {
                    self.states.chat.push_quest_feedback(
                        command.game_session_id,
                        &format!("Remaining {charges} charges."),
                    );
                }
            }
            return Ok(outcome(receipt.disposition));
        }
        let budget = super::super::AdmissionRetryBudget::for_command(self.root);
        if let Some(receipt) = budget
            .retry_use_holder(
                "dispenser_receipt",
                async || {
                    self.root
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
                },
                super::super::root_holder_busy,
            )
            .await
            .map_err(|_| ())?
        {
            return if receipt.recipe.as_deref() == Some("oteryn:dispenser.fill") {
                self.finish_dispenser(command.game_session_id, Some(receipt))
                    .await
            } else {
                Ok(None)
            };
        }
        let Some(ItemKey::Instance(flask)) = source else {
            return Ok(None);
        };
        let (instance, map_mint) = match target {
            Some(ItemKey::Instance(dispenser)) if flask != dispenser => (Some(dispenser), None),
            Some(ItemKey::Instance(_)) => return Ok(None),
            _ => {
                let runtime = self.runtime.lock().await;
                let Some(mint) = self.map_cask_request(&runtime, intent, target, request.command)
                else {
                    return Ok(None);
                };
                (None, Some(mint))
            }
        };
        let inventory = budget
            .retry_use_holder(
                "dispenser_inventory",
                async || {
                    self.root
                        .read_item_use_inventory(self.character, self.holder, fence)
                        .await
                },
                super::super::item_use_holder_busy,
            )
            .await
            .map_err(|_| ())?;
        let mut entries = inventory
            .backpack
            .iter()
            .flat_map(|bag| bag.entries.iter().map(|entry| &entry.item))
            .chain(inventory.nested.iter().map(|(_, entry)| &entry.item));
        let Some(source) = entries.clone().find(|item| item.item_instance_id == flask) else {
            return Ok(None);
        };
        if let Some((mint, position)) = map_mint {
            if !matching_flask(&mint.item.definition, &source.definition) {
                return Ok(None);
            }
            {
                let runtime = self.runtime.lock().await;
                let states = self.states.spell_states.lock().await;
                if !dispenser_actor_current(&runtime, &states, actor, command)
                    || runtime.binding().runtime_scope() != fence.runtime_scope
                    || runtime.binding().scope_generation() != fence.scope_ownership_generation
                    || !self
                        .states
                        .player_trades
                        .physical(command.game_session_id, Some(flask))
                {
                    return Ok(outcome(UseDisposition::Rejected));
                }
                let Some((positioned, at)) = runtime
                    .positioned_player_for_session(command.game_session_id)
                    .map_err(|_| ())?
                else {
                    return Ok(outcome(UseDisposition::Rejected));
                };
                if positioned != actor {
                    return Ok(outcome(UseDisposition::Rejected));
                }
                let at = at.position();
                if at.floor != position.floor
                    || at.x.abs_diff(position.x) > 1
                    || at.y.abs_diff(position.y) > 1
                {
                    return Ok(outcome(UseDisposition::TooFar));
                }
            }
            request.source = flask;
            let _ground_change =
                super::super::loop_move::GroundChange(Some(&self.states.loop_ground));
            let receipt = match self
                .root
                .consume_dispenser_from_map(
                    &permit,
                    self.character,
                    self.holder,
                    fence,
                    request,
                    mint,
                    native,
                    &weights,
                )
                .await
            {
                Ok(receipt) => receipt,
                Err(MapItemMintError::Refused(_)) => {
                    return Ok(outcome(UseDisposition::StaleState));
                }
                Err(MapItemMintError::Unavailable(_) | MapItemMintError::CapacityExceeded) => {
                    return Err(());
                }
                Err(_) => return Ok(outcome(UseDisposition::Rejected)),
            };
            return self
                .finish_dispenser(command.game_session_id, receipt)
                .await;
        }
        let dispenser = instance.ok_or(())?;
        let target_definition;
        let ground_source;
        if let Some(target) = entries.find(|item| item.item_instance_id == dispenser) {
            target_definition = target.definition.clone();
            ground_source = None;
        } else {
            let Some(target) = self
                .root
                .read_ground_action_source(self.character, fence, dispenser)
                .await
                .map_err(|_| ())?
            else {
                return Ok(None);
            };
            target_definition = target.definition().clone();
            ground_source = Some(target);
        }
        if !matching_flask(&target_definition, &source.definition) {
            return Ok(None);
        }
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(flask))
            || !self
                .states
                .player_trades
                .physical(command.game_session_id, Some(dispenser))
        {
            return Ok(outcome(UseDisposition::Rejected));
        }
        let ground_context = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if !dispenser_actor_current(&runtime, &states, actor, command)
                || pinned_index(&runtime, self.active_generation).is_none()
            {
                return Ok(outcome(UseDisposition::Rejected));
            }
            if let Some(target) = ground_source {
                let Some(room) = self.rooms().gate_ground_item_action() else {
                    return Ok(outcome(UseDisposition::NotPossible));
                };
                Some(
                    crate::durability::consume_transform::GroundActionContext::from_native_owner(
                        room,
                        &runtime,
                        actor,
                        command.game_session_id,
                        target,
                    )
                    .map_err(|_| ())?,
                )
            } else {
                None
            }
        };
        request.source = flask;
        request.target = Some(dispenser);
        let _ground_change = super::super::loop_move::GroundChange(
            ground_context.as_ref().map(|_| &self.states.loop_ground),
        );
        let receipt = if let Some(context) = &ground_context {
            self.root
                .consume_dispenser_on_ground(
                    &permit,
                    self.character,
                    self.holder,
                    fence,
                    request,
                    native,
                    &weights,
                    context,
                )
                .await
        } else {
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
        };
        self.finish_dispenser(command.game_session_id, receipt.map_err(|_| ())?)
            .await
    }

    async fn finish_dispenser(
        &self,
        session: GameSessionId,
        receipt: Option<ConsumeTransformResult>,
    ) -> Result<Option<UseOutcome>, ()> {
        let Some(receipt) = receipt else {
            return Ok(None);
        };
        if receipt.disposition == UseDisposition::Committed {
            let dispenser = &receipt.effects["dispenser"];
            let remaining = dispenser["after"].as_u64();
            #[cfg(test)]
            if receipt.fresh
                && DISPENSER_ACK_BARRIER
                    .try_with(|ready| ready.notify_one())
                    .is_ok()
            {
                std::future::pending::<()>().await;
            }
            if remaining == Some(0) && dispenser["source_kind"].as_i64() == Some(3) {
                self.states.loop_ground.invalidate();
                super::super::loop_move::reconcile_cask_collision(self)
                    .await
                    .ok_or(())?;
            }
            if receipt.fresh
                && let Some(charges) = remaining
            {
                self.states.loop_ground.invalidate();
                self.states
                    .chat
                    .push_quest_feedback(session, &format!("Remaining {charges} charges."));
            }
        }
        Ok(Some(UseOutcome {
            disposition: receipt.disposition,
            ..UseOutcome::rejected()
        }))
    }

    fn map_cask_request(
        &self,
        runtime: &ChannelRuntimeV1,
        intent: &UseIntent,
        target: Option<ItemKey>,
        command: CommandRef,
    ) -> Option<(MapItemMintRequest, MovementLocalPosition)> {
        let map = self.bundle_map()?;
        let native = self.active_generation?.native_gameplay()?;
        let index = pinned_index(runtime, self.active_generation)?;
        let base = map.overlay.base();
        let facts = map.facts.pinned();
        let source = MapViewSource {
            overlay: &map.overlay,
            facts: &facts,
            content_generation: runtime.content_pin().client_artifact_digest(),
            reset_epoch: self.reset_epoch,
        };
        let (pos, placement_key) = match (&intent.use_with_map_item, target) {
            (Some(MapUseTarget::Object(target)), None) => {
                let selected = resolve_map_target(&source, target, |_| None::<()>).ok()?;
                (selected.pos, selected.placement_key)
            }
            (Some(MapUseTarget::Item(_)), Some(key @ ItemKey::MapBase { .. })) => {
                let MapHandleTarget::Base {
                    pos, placement_key, ..
                } = resolve_map_handle(&source, key).ok()?
                else {
                    return None;
                };
                (pos, placement_key)
            }
            _ => return None,
        };
        if facts.object_revision(placement_key) != 0
            || map.facts.terrain_tool_action(placement_key) != Some(None)
            || !map.facts.captured_map_children(placement_key)?.is_empty()
        {
            return None;
        }
        let tile = view::compose_tile(&map.overlay, &facts, pos).ok()??;
        let entry = tile
            .entries
            .iter()
            .find(|entry| entry.source.key() == MapEntryKey::Base(placement_key))?;
        let MapDefinition::Item(reference) = entry.facts.definition else {
            return None;
        };
        let definition = crate::gameplay_transport::item_look::typed_definition(index, reference)?;
        if definition.production_key != MAP_DISPENSER_KEY
            || entry.facts.bound
            || entry.facts.count != 1
            || entry.facts.pickupable
            || entry.facts.sub_type != 0
        {
            return None;
        }
        // The map-mint owner carries the exact definition, so qualify only a cask
        // that uses the admitted definition's starting charge count.
        use crate::content::ReferenceItemField::Known;
        let policy = native.item_policy(&definition.production_key, &definition.revision_ref)?;
        let Known(charges) = &policy.record().semantics.charges else {
            return None;
        };
        let Known(initial) = charges.count else {
            return None;
        };
        if initial == 0 || initial > u32::from(u16::MAX) {
            return None;
        }
        let position = MovementLocalPosition {
            x: i32::from(pos.x),
            y: i32::from(pos.y),
            floor: -i16::from(pos.floor),
        };
        let room = self.rooms().gate_ground_item_action()?;
        LoopGround::from_native_owner(
            room,
            runtime,
            position,
            &crate::map::overlay::map_revision(base),
        )
        .ok()?;
        let mint = MapItemMintRequest {
            command,
            base_bundle_digest: base.digest(),
            placement_key,
            reset_epoch: self.reset_epoch,
            item: generation_ground_item_facts(native, &definition).ok()?,
            quantity: 1,
            key_number: None,
            flat_crate: None,
            content_revision: crate::durability::map_item_mint_audit::map_revision_of(
                &native.source_digest(),
            ),
            ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
            sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
        };
        Some((mint, position))
    }
}

fn dispenser_actor_current(
    runtime: &ChannelRuntimeV1,
    states: &crate::gameplay_transport::actor_spell::ChannelSpellStates,
    actor: ExactActorRef,
    command: UseCommand,
) -> bool {
    runtime.owner_fence().is_ok()
        && !states.is_dead(actor)
        && states.spell_idle(runtime, actor, command.game_session_id)
        && runtime
            .player_control_facts(actor, command.game_session_id)
            .is_ok_and(|facts| facts.control_loss.is_none())
}

fn matching_flask(dispenser: &TypedDefinitionRef, flask: &TypedDefinitionRef) -> bool {
    let id = |key: &str| key.strip_prefix("oteryn:item.tibia.i")?.parse::<u32>().ok();
    id(&dispenser.production_key)
        .zip(id(&flask.production_key))
        .and_then(|(keg, flask)| {
            oteryn_gameplay_items::remaining_consumables::keg_fill(keg, flask, 1, 1)
        })
        .is_some()
}
