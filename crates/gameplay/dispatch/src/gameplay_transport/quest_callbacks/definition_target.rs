//! Ground handles share the World callback's selected-tool transaction.
use super::*;
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseIntent, encode_item_use_intent};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn use_ground_definition_callback(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        if !matches!(intent.use_with_map_item, Some(MapUseTarget::Item(_))) {
            return Ok(None);
        }
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        let encoded = encode_item_use_intent(intent).map_err(|_| ())?;
        let command_ref = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).map_err(|_| ())?,
        );
        if let Some(matches) = self
            .root
            .reconcile_quest_carried_use(
                self.character,
                fence.character_id,
                command_ref,
                encoded.clone(),
            )
            .await
            .map_err(|_| ())?
        {
            if matches {
                self.refresh_quest_session(command.game_session_id, false)
                    .await;
            }
            return Ok(Some(UseOutcome {
                disposition: if matches {
                    UseDisposition::Committed
                } else {
                    UseDisposition::Rejected
                },
                ..UseOutcome::rejected()
            }));
        }
        let (Some(ItemKey::Instance(source)), Some(ItemKey::Instance(target))) = (source, target)
        else {
            return Ok(None);
        };
        let Ok(inventory) = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
        else {
            // Unrelated Ground actions retain their own historical receipt probe.
            return Ok(None);
        };
        let entries = || {
            inventory.equipment.iter().chain(
                inventory
                    .backpack
                    .iter()
                    .flat_map(|bag| &bag.entries)
                    .chain(inventory.nested.iter().map(|(_, entry)| entry))
                    .map(|entry| &entry.item),
            )
        };
        let Some(item) = entries().find(|entry| entry.item_instance_id == source) else {
            return Ok(None);
        };
        let owns_tool = self.callback_content().iter().any(|callback| {
            callback.selector.target_definition.is_some()
                && callback
                    .item_on_target
                    .as_ref()
                    .is_some_and(|tool| tool.item == item.definition.production_key)
        });
        if !owns_tool && entries().any(|entry| entry.item_instance_id == target) {
            return Ok(None);
        }
        let ground = self
            .root
            .read_item_loop_ground_for_scope(fence.runtime_scope)
            .await;
        let ground = match ground {
            Ok(ground) => ground,
            Err(_) if !owns_tool => return Ok(None),
            Err(_) => return Err(()),
        };
        let Some(ground) = ground
            .into_iter()
            .find(|entry| entry.item.item_instance_id == target)
        else {
            return Ok(owns_tool.then(UseOutcome::rejected));
        };
        let mut candidates = self.callback_content().iter().filter(|callback| {
            callback.selector.target_definition.as_deref()
                == Some(ground.item.definition.production_key.as_str())
        });
        let Some(callback) = candidates.next() else {
            return Ok(owns_tool.then(UseOutcome::rejected));
        };
        if candidates.next().is_some()
            || !callback
                .item_on_target
                .as_ref()
                .is_some_and(|tool| tool.item == item.definition.production_key)
        {
            return Ok(Some(UseOutcome::rejected()));
        }
        let Some(map) = self.bundle_map() else {
            return Ok(Some(UseOutcome::rejected()));
        };
        use sha2::{Digest, Sha256};
        let map_revision = crate::map::overlay::map_revision(map.overlay.base());
        let native_revision = format!("sha256:{}", hex(&Sha256::digest(map_revision.as_bytes())));
        let cell = crate::durability::spell_items_abi::decode_ground_cell(
            &ground.item.ground.spatial_position,
        )
        .map_err(|_| ())?;
        let floor = if ground.item.ground.map_revision == native_revision {
            cell.z
        } else if ground.item.ground.map_revision == map_revision {
            -cell.z
        } else {
            return Ok(Some(UseOutcome::rejected()));
        };
        let target_position = crate::foundation::MovementLocalPosition {
            x: cell.x,
            y: cell.y,
            floor: i16::try_from(floor).map_err(|_| ())?,
        };
        let Some(catalogue) = self.quest_catalogue.as_ref() else {
            return Ok(Some(UseOutcome::rejected()));
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return Ok(Some(UseOutcome::rejected()));
        };
        let Some(current) = self
            .current_quest_fence(command.game_session_id)
            .await
            .map_err(|_| ())?
        else {
            return Ok(Some(UseOutcome::rejected()));
        };
        if current.item_fence() != fence {
            return Ok(Some(UseOutcome::rejected()));
        }
        let mut revision = self.revision_sequencer.acquire(current.character_id).await;
        // Keep the exact actor and its position stable until the durable commit.
        let runtime = self.runtime.lock().await;
        let at = runtime.read_actor_position(actor).map_err(|_| ())?;
        if runtime.owner_fence().is_err()
            || !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
            || self.states.spell_states.lock().await.is_dead(actor)
        {
            return Ok(Some(UseOutcome::rejected()));
        }
        let Some(map) = self.bundle_map() else {
            return Ok(Some(UseOutcome::rejected()));
        };
        let context = QuestWorldContext {
            map_revision: crate::map::overlay::map_revision(map.overlay.base()),
            content_revision: format!(
                "sha256:{}",
                hex(&runtime.content_pin().server_artifact_digest())
            ),
            placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
            reset_epoch: self.reset_epoch,
            actor_placement: actor.placement_identity(),
            movement_revision: at.revision(),
            party: Vec::new(),
        }
        .with_selected_ground_tool(
            source,
            encoded,
            target,
            [
                at.position().x,
                at.position().y,
                i32::from(at.position().floor),
            ],
            [
                target_position.x,
                target_position.y,
                i32::from(target_position.floor),
            ],
        )
        .ok_or(())?;
        let result = revision
            .commit_world_callback_with_items_selected(
                self.root,
                self.character,
                self.holder,
                current,
                QuestTransitionRequest {
                    transition_key: callback.key.clone(),
                    cause: QuestCause::Use(command_ref),
                },
                std::sync::Arc::clone(catalogue),
                &permit,
                context,
                self.active_generation
                    .and_then(|generation| generation.native_gameplay()),
                None,
                &self.premium_coordinator,
            )
            .await
            .map_err(|_| ())?;
        drop(runtime);
        drop(revision);
        drop(permit);
        let committed = matches!(
            result,
            QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)
        );
        if matches!(result, QuestTransitionOutcome::Committed(_))
            && let Some(effect) = callback.effects.target_magic
            && let Some(position) = native(target_position)
        {
            map.facts.emit_movement_effect(position, effect);
        }
        if matches!(result, QuestTransitionOutcome::Committed(_))
            && let Some(message) = &callback.success_message
        {
            self.states
                .chat
                .push_quest_feedback(command.game_session_id, message);
        }
        if committed {
            self.refresh_quest_session(command.game_session_id, false)
                .await;
        }
        Ok(Some(UseOutcome {
            disposition: if committed {
                UseDisposition::Committed
            } else {
                UseDisposition::Rejected
            },
            ..UseOutcome::rejected()
        }))
    }
}
