//! P01 skinning uses a current carried tool and the exact selected durable corpse handle.
use super::*;
use crate::durability::item_craft::SkinningRequest;

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::too_many_lines,
        reason = "one ordinary use action keeps replay, exact corpse resolution and runtime fencing together"
    )]
    pub(super) async fn use_skinning(
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
        let outcome = |disposition| {
            Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            })
        };
        let mut request = SkinningRequest {
            craft: ItemCraftRequest {
                command: CommandRef::new(
                    command.game_session_id,
                    CommandId::new(command.command_id).map_err(|_| ())?,
                ),
                source: [0; 16],
                tool: None,
                ingredient: None,
                station: None,
                gathering: None,
                enchanting: None,
                intent: oteryn_protocol_oteryn::world_object::encode_item_use_intent(intent)
                    .map_err(|_| ())?,
            },
            corpse: [0; 16],
            actor_position: [0; 3],
        };
        let weights = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        if let Some(result) = self
            .root
            .skin_corpse(
                &permit,
                self.character,
                self.holder,
                fence,
                request.clone(),
                native,
                &weights,
                self.imported_charms,
                true,
            )
            .await
            .map_err(|_| ())?
        {
            if result == UseDisposition::Committed {
                self.refresh_skinning_projection(fence).await;
            }
            return Ok(outcome(result));
        }
        let Some(ItemKey::Instance(tool)) = source else {
            return Ok(None);
        };
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .map_err(|_| ())?;
        let entry = inventory
            .backpack
            .iter()
            .flat_map(|bag| &bag.entries)
            .find(|entry| entry.item.item_instance_id == tool);
        let source_id = |key: &str| {
            key.strip_prefix("oteryn:item.tibia.i")
                .and_then(|id| id.parse::<u16>().ok())
        };
        let Some(tool_id) =
            entry.and_then(|entry| source_id(&entry.item.definition.production_key))
        else {
            return Ok(None);
        };
        let rabbit_knife = oteryn_gameplay_items::fishing::rabbit_skinning_knife(tool_id);
        if !rabbit_knife && tool_id != 5942 {
            return Ok(None);
        }
        let corpse = match target {
            Some(ItemKey::Instance(item)) => item,
            Some(ItemKey::Entity(identity)) => {
                let runtime = self.runtime.lock().await;
                let Some(corpse) = runtime.bound_corpse(identity) else {
                    return Ok(outcome(UseDisposition::StaleState));
                };
                corpse.item.item_instance_id
            }
            _ => return Ok(None),
        };
        // Other obsidian-knife uses (ice/marble) retain their existing owners.
        // This identifies the rule shape only; the durable provenance supplies the race.
        let Some(ground) = self
            .root
            .read_ground_action_source(self.character, fence, corpse)
            .await
            .map_err(|_| ())?
        else {
            return Ok(None);
        };
        let corpse_id = source_id(&ground.definition().production_key);
        let fresh_rabbit = rabbit_knife && corpse_id == Some(6017);
        if !fresh_rabbit
            && !corpse_id
                .and_then(|target| {
                    oteryn_gameplay_items::fishing::corpse_skinning_rule(tool_id, target)
                })
                .is_some_and(|(target, _)| {
                    matches!(
                        target,
                        oteryn_gameplay_items::fishing::GatheringTarget::Corpse { spent_item: 1.. }
                    )
                })
        {
            return Ok(None);
        }

        {
            let runtime = self.runtime.lock().await;
            let Ok(states) = self.states.spell_states.try_lock() else {
                return Err(());
            };
            if !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
                || states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, command.game_session_id)
                || pinned_index(&runtime, self.active_generation).is_none()
                || !self
                    .states
                    .player_trades
                    .physical(command.game_session_id, Some(tool))
                || !self
                    .states
                    .player_trades
                    .physical(command.game_session_id, Some(corpse))
            {
                return Ok(outcome(UseDisposition::Rejected));
            }
            let position = runtime
                .read_actor_position(actor)
                .map_err(|_| ())?
                .position();
            request.actor_position = [position.x, position.y, i32::from(position.floor)];
        }
        if fresh_rabbit {
            let cell = ground.position();
            if request.actor_position[0].abs_diff(cell.x) <= 1
                && request.actor_position[1].abs_diff(cell.y) <= 1
                && request.actor_position[2] == i32::from(cell.floor)
            {
                self.states.chat.push_quest_feedback(
                    command.game_session_id,
                    "You have to wait a bit before you can skin this animal.",
                );
            }
            return Ok(outcome(UseDisposition::NothingToUse));
        }
        request.craft.source = tool;
        request.corpse = corpse;
        let result = self
            .root
            .skin_corpse(
                &permit,
                self.character,
                self.holder,
                fence,
                request,
                native,
                &weights,
                self.imported_charms,
                false,
            )
            .await
            .map_err(|_| ())?
            .ok_or(())?;
        if result == UseDisposition::Committed {
            self.refresh_skinning_projection(fence).await;
        }
        drop(permit);
        Ok(outcome(result))
    }

    /// Refresh only existing corpse bindings from the complete durable owner snapshot.
    /// The caller retains its item lane across this read and runtime update, including replay.
    async fn refresh_skinning_projection(
        &self,
        fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    ) {
        let Ok(schedule) = self
            .root
            .read_scoped_corpse_decay_schedule(self.character, fence.runtime_scope)
            .await
        else {
            return;
        };
        let mut runtime = self.runtime.lock().await;
        if runtime.binding().runtime_scope() != fence.runtime_scope
            || !runtime
                .owner_fence()
                .is_ok_and(|owner| owner.generation() == fence.scope_ownership_generation)
        {
            return;
        }
        for corpse in &schedule.corpses {
            let _ = runtime.refresh_corpse_item(
                corpse.corpse_item_instance_id,
                &corpse.definition.family,
                &corpse.definition.production_key,
                &corpse.definition.revision_ref,
            );
        }
        self.states.loop_ground.invalidate();
    }
}
