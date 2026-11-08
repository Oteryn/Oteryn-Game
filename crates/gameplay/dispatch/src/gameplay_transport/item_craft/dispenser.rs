//! Flask-on-dispenser is an ordinary USE_WITH, composed with the item action owner.
use super::*;
use crate::durability::consume_transform::ConsumeTransformRequest;

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
        if let Some(receipt) = self
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
            return Ok(
                if receipt.recipe.as_deref() == Some("oteryn:dispenser.fill") {
                    outcome(receipt.disposition)
                } else {
                    None
                },
            );
        }
        let (Some(ItemKey::Instance(flask)), Some(ItemKey::Instance(dispenser))) = (source, target)
        else {
            return Ok(None);
        };
        if flask == dispenser {
            return Ok(None);
        }
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
        let Some(source) = entries.clone().find(|item| item.item_instance_id == flask) else {
            return Ok(None);
        };
        let target_definition;
        let ground_source;
        if let Some(target) = entries
            .clone()
            .find(|item| item.item_instance_id == dispenser)
        {
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
        let id = |key: &str| key.strip_prefix("oteryn:item.tibia.i")?.parse::<u32>().ok();
        if id(&target_definition.production_key)
            .zip(id(&source.definition.production_key))
            .and_then(|(keg, flask)| {
                oteryn_gameplay_items::remaining_consumables::keg_fill(keg, flask, 1, 1)
            })
            .is_none()
        {
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
            if runtime.owner_fence().is_err()
                || states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, command.game_session_id)
                || !runtime
                    .player_control_facts(actor, command.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
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
        receipt
            .map(|receipt| {
                receipt.and_then(|receipt| {
                    if receipt.fresh
                        && receipt.disposition == UseDisposition::Committed
                        && let Some(charges) = receipt.effects["dispenser"]["after"].as_u64()
                    {
                        self.states.loop_ground.invalidate();
                        self.states.chat.push_quest_feedback(
                            command.game_session_id,
                            &format!("Remaining {charges} charges."),
                        );
                    }
                    outcome(receipt.disposition)
                })
            })
            .map_err(|_| ())
    }
}
