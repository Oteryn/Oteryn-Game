//! Resolve source USE facts through the existing custody and composed map owners.
use super::*;
use crate::durability::item_use::soul_war_source::{
    QualifiedSoulWarUseTarget, SelectedSoulWarUseTarget, SoulWarUseTargetRead, source_use,
};
use crate::map::view::{self, MapEntryKey};
use oteryn_protocol_oteryn::world_map::MapDefinition;
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseIntent, UseTarget};

struct CurrentSelection<'a> {
    command: UseCommand,
    actor: ExactActorRef,
    source: &'a TypedDefinitionRef,
    selected: SelectedSoulWarUseTarget,
}
impl SoulWarUseTargetRead for CurrentSelection<'_> {
    fn source_definition(&self, _: &UseIntent) -> Option<TypedDefinitionRef> {
        Some(self.source.clone())
    }
    fn current(&self, runtime: &ChannelRuntimeV1) -> bool {
        rotten_essence::current(runtime, self.actor, self.command)
    }
    fn resolve(&self, _: &UseIntent) -> Option<SelectedSoulWarUseTarget> {
        Some(self.selected.clone())
    }
}
impl ComposedFreshAdmission<'_, '_, '_> {
    /// The returned entry is read from the actual fenced backpack tree. The
    /// caller must pass that same entry and proof to the paired Item/Quest owner.
    pub(in crate::gameplay_transport) async fn qualify_soul_war_source_use(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Option<(BackpackEntry, QualifiedSoulWarUseTarget, String, String)> {
        let fence = command.item_fence?;
        let native = self.active_generation?.native_gameplay()?;
        {
            let runtime = self.runtime.lock().await;
            if !rotten_essence::current(&runtime, actor, command)
                // A House actor is a proxy: its map must be resolved by the
                // House owner, never the origin Channel map used below.
                || runtime.house_actor_state(actor, command.game_session_id).is_ok()
            {
                return None;
            }
        }
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .ok()?;
        let mut runtime = self.runtime.lock().await;
        if !rotten_essence::current(&runtime, actor, command) {
            return None;
        }
        let index =
            super::super::item_ref_admission::pinned_index(&runtime, self.active_generation)?;
        let item = match intent.target {
            UseTarget::Item(_) => match source? {
                ItemKey::Instance(id) => id,
                _ => return None,
            },
            UseTarget::ItemByDefinition(reference) => {
                let (key, revision) = index.definition_at(reference)?;
                let definition = TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: key.into(),
                    revision_ref: revision.into(),
                };
                source_use(&definition)?;
                let mut states = self.states.spell_states.lock().await;
                inventory
                    .first_unreserved(
                        &definition,
                        &rotten_essence::gate(
                            &mut states,
                            &runtime,
                            actor,
                            command.game_session_id,
                        )
                        .unspendable,
                    )
                    .ok()?
            }
            _ => return None,
        };
        let entry = inventory
            .backpack
            .as_ref()
            .into_iter()
            .flat_map(|bag| bag.entries.iter())
            .chain(inventory.nested.iter().map(|(_, entry)| entry))
            .find(|entry| entry.item.item_instance_id == item)?
            .clone();
        source_use(&entry.item.definition)?;
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(item))
        {
            return None;
        }
        let selected = if let Some(wanted) = intent.use_with {
            let visible = runtime.visible_entities();
            let creature = visible
                .creatures
                .iter()
                .find(|entry| {
                    entry.actor.placement_identity() == wanted.identity
                        && entry.generation == wanted.generation
                })
                .map(|entry| entry.actor);
            let selected = creature.or_else(|| {
                visible
                    .players
                    .iter()
                    .find(|entry| {
                        entry.actor.placement_identity() == wanted.identity
                            && entry.generation == wanted.generation
                    })
                    .map(|entry| entry.actor)
            })?;
            let own = runtime.read_actor_position(actor).ok()?.position();
            let target_position = runtime.read_actor_position(selected).ok()?.position();
            if own.floor != target_position.floor
                || own.x.abs_diff(target_position.x) > 1
                || own.y.abs_diff(target_position.y) > 1
            {
                return None;
            }
            if creature.is_some() {
                SelectedSoulWarUseTarget::Creature(selected)
            } else {
                SelectedSoulWarUseTarget::OtherMapEntry(target_position)
            }
        } else if matches!(intent.use_with_map_item, Some(MapUseTarget::Item(_)))
            && matches!(target, Some(ItemKey::Instance(_)))
        {
            let Some(ItemKey::Instance(target_id)) = target else {
                return None;
            };
            let target_entry = inventory
                .backpack
                .as_ref()
                .into_iter()
                .flat_map(|bag| bag.entries.iter())
                .chain(inventory.nested.iter().map(|(_, entry)| entry))
                .find(|entry| entry.item.item_instance_id == target_id)?;
            SelectedSoulWarUseTarget::Item {
                definition: target_entry.item.definition.clone(),
                position: runtime.read_actor_position(actor).ok()?.position(),
            }
        } else if intent.use_with_map_item.is_some() {
            let at = super::super::bundle_map_position(
                &mut runtime,
                self.effective_movement_cells(),
                actor,
            )?;
            let facts = at.map.facts.pinned();
            let map = super::super::world_map::MapViewSource {
                overlay: &at.map.overlay,
                facts: &facts,
                content_generation: at.content_generation,
                reset_epoch: self.reset_epoch,
            };
            let (position, key) = match &intent.use_with_map_item {
                Some(MapUseTarget::Object(object)) => {
                    let resolved =
                        super::super::world_map::resolve_map_target(&map, object, |_| None::<()>)
                            .ok()?;
                    (resolved.pos, MapEntryKey::Base(resolved.placement_key))
                }
                Some(MapUseTarget::Item(_)) => {
                    match super::super::world_map::resolve_map_handle(&map, target?).ok()? {
                        super::super::world_map::MapHandleTarget::Base {
                            pos,
                            placement_key,
                            ..
                        } => (pos, MapEntryKey::Base(placement_key)),
                        _ => return None,
                    }
                }
                None => return None,
            };
            // Crystal actions.cpp:269-280,688-694: these literal Actions
            // never enable far use; floor and one-cell Chebyshev reach apply.
            if at.at.floor != i16::from(position.floor)
                || at.at.x.abs_diff(i32::from(position.x)) > 1
                || at.at.y.abs_diff(i32::from(position.y)) > 1
            {
                return None;
            }
            let tile = view::compose_tile(&at.map.overlay, &facts, position).ok()??;
            let entry = tile
                .entries
                .iter()
                .find(|entry| entry.source.key() == key)?;
            let position = crate::foundation::MovementLocalPosition {
                x: i32::from(position.x),
                y: i32::from(position.y),
                floor: i16::from(position.floor),
            };
            match entry.facts.definition {
                MapDefinition::Item(reference) => SelectedSoulWarUseTarget::Item {
                    definition: super::super::item_look::typed_definition(index, reference)?,
                    position,
                },
                _ => SelectedSoulWarUseTarget::OtherMapEntry(position),
            }
        } else {
            SelectedSoulWarUseTarget::OtherMapEntry(
                runtime.read_actor_position(actor).ok()?.position(),
            )
        };
        let reader = CurrentSelection {
            command,
            actor,
            source: &entry.item.definition,
            selected,
        };
        let qualified = QualifiedSoulWarUseTarget::from_current_owner(
            &runtime,
            native,
            actor,
            command.game_session_id,
            &entry.item.definition,
            intent,
            &reader,
        )?;
        Some((
            entry,
            qualified,
            inventory.content_revision,
            inventory.ruleset_revision,
        ))
    }
}

#[derive(Debug, Clone)]
pub(super) struct PendingSourceUse {
    actor: ExactActorRef,
    command: UseCommand,
    wire: [u8; 32],
    input: crate::durability::quest_state::soul_war_source::SoulWarSourceUse,
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn dispatch_soul_war_source_use(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        use crate::durability::quest_state::{
            QuestTransitionOutcome,
            soul_war_source::{SoulWarSourceAction, SoulWarSourceUse},
        };
        let wire: [u8; 32] = {
            let mut hash = Sha256::new();
            hash.update(b"oteryn:soul-war-source-use-wire:v1");
            hash.update(
                oteryn_protocol_oteryn::world_object::encode_item_use_intent(intent)
                    .map_err(|_| ())?,
            );
            hash.finalize().into()
        };
        let pending = {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            let held = rotten_essence::gate(&mut states, &runtime, actor, command.game_session_id);
            if let Some((id, previous, disposition)) = held.source_last
                && id == command.command_id
            {
                return if previous == wire {
                    Ok(Some(UseOutcome {
                        disposition,
                        ..UseOutcome::rejected()
                    }))
                } else {
                    Err(())
                };
            }
            held.source_pending.as_deref().cloned()
        };
        let pending = if let Some(pending) = pending {
            if pending.actor != actor || pending.command != command || pending.wire != wire {
                return Err(());
            }
            pending
        } else {
            let Some((entry, qualified, content_revision, ruleset_revision)) = self
                .qualify_soul_war_source_use(actor, command, intent, source, target)
                .await
            else {
                return Ok(None);
            };
            if qualified.source_use()
                == crate::durability::item_use::soul_war_source::SourceUse::MortalEssence
                && !qualified.target_accepted()
            {
                return Ok(Some(UseOutcome {
                    disposition: UseDisposition::NotPossible,
                    ..UseOutcome::rejected()
                }));
            }
            // Sorrow has no persistent Quest transition and stays with its exact
            // item-only source owner rather than manufacturing a cursor write.
            if qualified.source_use()
                == crate::durability::item_use::soul_war_source::SourceUse::Sorrow
            {
                return Ok(Some(UseOutcome::rejected()));
            }
            let pending = PendingSourceUse {
                actor,
                command,
                wire,
                input: SoulWarSourceUse {
                    target: qualified,
                    item: ItemUseRequest {
                        kind: ItemUseKind::SoulWarSourceUse { wire_binding: wire },
                        command_id: command.command_id,
                        content_revision,
                        ruleset_revision,
                        sim_revision: entry_chest::SIM_REVISION.into(),
                        transaction_id: crate::operator_files::uuid_v7().map_err(|_| ())?,
                        event_id: crate::operator_files::uuid_v7().map_err(|_| ())?,
                        occurred_at_unix_ms: unix_ms().map_err(|_| ())?,
                        server_build_id: SERVER_BUILD_ID.into(),
                        source_item_instance_id: entry.item.item_instance_id,
                        source_definition: entry.item.definition,
                        source_quantity_before: entry.item.quantity,
                        flask: ItemUseFlaskPlan::None,
                    },
                },
            };
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            if !rotten_essence::current(&runtime, actor, command)
                || states.is_dead(actor)
                || states.has_pending_spell_commit(actor, command.game_session_id)
            {
                return Err(());
            }
            let held = rotten_essence::gate(&mut states, &runtime, actor, command.game_session_id);
            held.admit(command.command_id, self.owner_ms())
                .map_err(|_| ())?;
            if held
                .unspendable
                .contains(&pending.input.item.source_item_instance_id)
            {
                return Err(());
            }
            // The shared one-write-in-flight gate starts without inventing a
            // gameplay cooldown; the source thirty-second clock is durable.
            held.start_drink(self.owner_ms()).map_err(|_| ())?;
            held.unspendable
                .push(pending.input.item.source_item_instance_id);
            held.source_pending = Some(Box::new(pending.clone()));
            pending
        };
        {
            let runtime = self.runtime.lock().await;
            let pin = runtime.content_pin();
            let selected = &pending.input.target;
            if !rotten_essence::current(&runtime, actor, command)
                || pin.activation_sequence() != selected.activation_sequence()
                || pin.server_artifact_digest() != selected.server_artifact_digest()
                || pin.client_artifact_digest() != selected.client_artifact_digest()
                || pin.frame_binding_digest() != selected.frame_binding_digest()
                || runtime.map_revision_digest() != selected.map_revision_digest()
            {
                return Err(());
            }
        }
        let _permit = self.spell_lane_permit().await.ok_or(())?;
        let fence = self
            .current_quest_fence(command.game_session_id)
            .await?
            .ok_or(())?;
        if Some(fence.item_fence()) != pending.command.item_fence {
            return Err(());
        }
        let catalogue = self.quest_catalogue.as_ref().ok_or(())?;
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let result = slot
            .commit_soul_war_source_action(
                self.root,
                self.character,
                self.holder,
                fence,
                SoulWarSourceAction::Use(pending.input.clone()),
                std::sync::Arc::clone(catalogue),
                None,
            )
            .await;
        drop(slot);
        let known_transition = matches!(
            &result,
            Ok(QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_))
        );
        let mut disposition = match result {
            Ok(
                QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_),
            ) => UseDisposition::Committed,
            Ok(QuestTransitionOutcome::Refused(_) | QuestTransitionOutcome::ObligationClosed) => {
                UseDisposition::NotPossible
            }
            Err(
                crate::durability::character_progression::CharacterProgressionError::Unavailable(_),
            ) => return Err(()),
            Err(_) => UseDisposition::Rejected,
        };
        if known_transition
            && pending.input.target.source_use()
                == crate::durability::item_use::soul_war_source::SourceUse::CleansedSanity
            && !pending.input.target.target_accepted()
        {
            // Lua stores the cooldown before testing the target, then returns
            // false. A known Quest commit is not an action success or Item burn.
            disposition = UseDisposition::NotPossible;
        }
        {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            let held = rotten_essence::gate(&mut states, &runtime, actor, command.game_session_id);
            held.source_pending = None;
            held.source_last = Some((command.command_id, wire, disposition));
            held.unspendable
                .retain(|id| *id != pending.input.item.source_item_instance_id);
            held.settle(command.command_id, disposition, &[]);
        }
        if known_transition {
            Box::pin(self.refresh_quest_session(command.game_session_id, false)).await;
        }
        Ok(Some(UseOutcome {
            disposition,
            ..UseOutcome::rejected()
        }))
    }
}
