//! The existing current-fenced carried-item craft route for authored World stations.
use super::*;
use crate::durability::terrain_tool::{TerrainMutation, TerrainScope};
use oteryn_gameplay_world_change::variants;

use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{GameSessionId, MovementLocalPosition};
use oteryn_gameplay_entities::carrier::native_actor_batch::PreparedNativeActorBatch;

#[derive(Debug)]
pub(in crate::gameplay_transport) struct PendingWorldStation {
    actor: ExactActorRef,
    session: GameSessionId,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    request: ItemCraftRequest,
    batch: PreparedNativeActorBatch,
}
fn park(pending: PendingWorldStation) -> Box<dyn std::any::Any + Send> {
    Box::new(
        super::super::actor_spell::native_combat_cast::UnresolvedSpellCommit::WorldStation(
            Box::new(pending),
        ),
    )
}
#[cfg(test)]
pub(in crate::gameplay_transport) fn qualified_pending_station(
    actor: ExactActorRef,
    session: GameSessionId,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    request: ItemCraftRequest,
    batch: PreparedNativeActorBatch,
) -> PendingWorldStation {
    PendingWorldStation {
        actor,
        session,
        fence,
        request,
        batch,
    }
}
#[cfg(test)]
pub(in crate::gameplay_transport) fn park_qualified_station(
    pending: PendingWorldStation,
) -> Box<dyn std::any::Any + Send> {
    park(pending)
}

fn spatial(position: MovementLocalPosition) -> [u8; 10] {
    let mut bytes = [0; 10];
    bytes[..4].copy_from_slice(&position.x.to_be_bytes());
    bytes[4..8].copy_from_slice(&position.y.to_be_bytes());
    bytes[8..].copy_from_slice(&position.floor.to_be_bytes());
    bytes
}

fn outcome(disposition: UseDisposition) -> Option<UseOutcome> {
    Some(UseOutcome {
        disposition,
        ..UseOutcome::rejected()
    })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Hook after the existing specialized craft handlers, before generic cooking.
    pub(super) async fn use_world_change_station(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        if intent.use_with_map_item.is_none() {
            return Ok(None);
        }
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        let Some(native) = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        else {
            return Ok(None);
        };
        let mut permit = self.spell_lane_permit().await.ok_or(())?;
        let mut request = ItemCraftRequest {
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
        };
        let weights = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        // The common receipt route answers a consumed material's retry before handles.
        if let Some(result) = self
            .root
            .craft_item(
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
            return Ok(outcome(result));
        }
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .map_err(|_| ())?;
        let index = native.item_index().ok_or(())?;
        let source = match intent.target {
            UseTarget::Item(_) => match source {
                Some(ItemKey::Instance(id)) => Some(id),
                _ => None,
            },
            UseTarget::ItemByDefinition(wanted) => {
                index.definition_at(wanted).and_then(|(key, revision)| {
                    inventory
                        .first_unreserved(
                            &TypedDefinitionRef {
                                family: "Item".into(),
                                production_key: key.into(),
                                revision_ref: revision.into(),
                            },
                            &[],
                        )
                        .ok()
                })
            }
            _ => None,
        };
        let Some(entry) = inventory
            .backpack
            .iter()
            .flat_map(|b| &b.entries)
            .find(|entry| Some(entry.item.item_instance_id) == source)
        else {
            return Ok(None);
        };
        let material = &entry.item.definition;
        if !variants::definitions().map_err(|_| ())?.iter().any(|d| {
            d.stations
                .iter()
                .any(|station| station.material == material.production_key)
        }) {
            return Ok(None);
        }
        if index
            .definition_ref(
                &material.family,
                &material.production_key,
                &material.revision_ref,
            )
            .is_none()
            || !self
                .states
                .player_trades
                .physical(command.game_session_id, source)
        {
            return Ok(outcome(UseDisposition::Rejected));
        }
        let (scope, mutation, attempt) = {
            let mut runtime = self.runtime.lock().await;
            let Ok(states) = self.states.spell_states.try_lock() else {
                return Ok(outcome(UseDisposition::Rejected));
            };
            if runtime.owner_fence().is_err()
                || runtime.binding().runtime_scope() != fence.runtime_scope
                || runtime.binding().scope_generation() != fence.scope_ownership_generation
                || !runtime
                    .bound_attacker_lease(actor, command.game_session_id)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    })
                || runtime.binding().world_id() != self.world_id
                || runtime.binding().channel_id() != Some(self.channel_id)
                || pinned_index(&runtime, self.active_generation).is_none()
                || !runtime
                    .player_control_facts(actor, command.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
                || states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, command.game_session_id)
            {
                return Ok(outcome(UseDisposition::Rejected));
            }
            let Some(at) =
                super::super::bundle_map_position(&mut runtime, self.movement_cells, actor)
            else {
                return Ok(outcome(UseDisposition::NothingToUse));
            };
            let facts = at.map.facts.pinned();
            let map = super::super::world_map::MapViewSource {
                overlay: &at.map.overlay,
                facts: &facts,
                content_generation: at.content_generation,
                reset_epoch: self.reset_epoch,
            };
            let resolved = match &intent.use_with_map_item {
                Some(MapUseTarget::Object(object)) => {
                    super::super::world_map::resolve_map_target(&map, object, |_| None::<()>)
                        .ok()
                        .map(|t| (t.pos, t.placement_key))
                }
                Some(MapUseTarget::Item(_)) => target
                    .and_then(|key| super::super::world_map::resolve_map_handle(&map, key).ok())
                    .and_then(|t| match t {
                        super::super::world_map::MapHandleTarget::Base {
                            pos,
                            placement_key,
                            ..
                        } => Some((pos, placement_key)),
                        _ => None,
                    }),
                None => None,
            };
            let Some((pos, key)) = resolved else {
                return Ok(outcome(UseDisposition::StaleState));
            };
            if i16::from(pos.floor) != at.at.floor
                || at.at.x.abs_diff(i32::from(pos.x)) > 1
                || at.at.y.abs_diff(i32::from(pos.y)) > 1
            {
                return Ok(outcome(UseDisposition::TooFar));
            }
            let tile = view::compose_tile(&at.map.overlay, &facts, pos)
                .map_err(|_| ())?
                .ok_or(())?;
            let before = tile
                .entries
                .iter()
                .find(|e| e.source.key() == MapEntryKey::Base(key))
                .ok_or(())?
                .facts;
            if facts.house_tile(pos)
                || before.pickupable
                || before.count != 1
                || before.sub_type != 0
            {
                return Ok(outcome(UseDisposition::NothingToUse));
            }
            let mut placement = at.map.overlay.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            let source_position = variants::station_position(&placement).ok_or(())?;
            let Some((_, station)) = variants::station_at(
                source_position,
                before.appearance_id,
                &material.production_key,
            ) else {
                return Ok(None);
            };
            if at.map.facts.terrain_tool_action(key) != Some(None)
                || !at
                    .map
                    .facts
                    .quest_callback_capacity(key, facts.object_revision(key), 0)
                || station.after.is_some_and(|after| {
                    at.map
                        .facts
                        .quest_callback_replacement(key, after)
                        .is_none()
                })
            {
                return Ok(outcome(UseDisposition::NothingToUse));
            }
            (
                TerrainScope {
                    map_revision: crate::map::overlay::map_revision(at.map.overlay.base()),
                    content_generation: native.source_digest(),
                    reset_epoch: i64::try_from(self.reset_epoch).map_err(|_| ())?,
                },
                TerrainMutation {
                    tool: entry.item.item_instance_id,
                    tool_key: material.production_key.clone(),
                    tool_revision: material.revision_ref.clone(),
                    placement,
                    expected_revision: i64::try_from(facts.object_revision(key)).map_err(|_| ())?,
                    before: u32::from(before.appearance_id),
                    after: station.after.map_or(0, u32::from),
                    decay: station
                        .restore_after_save_ms
                        .first()
                        .map(|offset| vec![(u32::from(station.appearance), i64::from(*offset))])
                        .unwrap_or_default(),
                },
                oteryn_gameplay_items::fishing::GatheringAttempt {
                    after: station.after,
                    yield_item: station.yield_item,
                    consumes_bait: true,
                },
            )
        };
        request.source = entry.item.item_instance_id;
        request.gathering = Some(crate::durability::item_craft::GatheringCraft {
            scope,
            mutation,
            attempt,
        });
        if material.production_key == "oteryn:item.tibia.i12722"
            && crate::durability::item_craft::source_station_wolves(&request).ok_or(())?
        {
            return self
                .commit_world_station_actors(
                    actor,
                    command,
                    fence,
                    request,
                    native,
                    &weights,
                    &mut permit,
                )
                .await;
        }
        let result = self
            .root
            .craft_item(
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
            .map_err(|_| ())?
            .ok_or(())?;
        drop(permit);
        if result == UseDisposition::Committed {
            self.refresh_terrain_tools().await;
        }
        Ok(outcome(result))
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    async fn world_station_actor_current(&self, pending: &PendingWorldStation) -> Result<bool, ()> {
        let Some(current) = self.current_quest_fence(pending.session).await? else {
            return Ok(false);
        };
        if current.item_fence() != pending.fence {
            return Ok(false);
        }
        {
            let runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err()
                || !runtime
                    .bound_attacker_lease(pending.actor, pending.session)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == pending.fence.character_id.as_bytes()
                            && lease.generation() == pending.fence.character_lease_generation
                    })
            {
                return Ok(false);
            }
        }
        let epoch = u64::try_from(
            pending
                .request
                .gathering
                .as_ref()
                .ok_or(())?
                .scope
                .reset_epoch,
        )
        .map_err(|_| ())?;
        Ok(self
            .root
            .world_changes(pending.fence.runtime_scope.world_id(), epoch)
            .await
            .map_err(|_| ())?
            .is_some())
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "existing craft owner plus exact current source actor"
    )]
    async fn commit_world_station_actors<C: crate::durability::item_loop_move::ItemLoopContent>(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        fence: crate::durability::item_transfer::CurrentCharacterItemFence,
        request: ItemCraftRequest,
        native: &crate::content::native_gameplay::NativeGameplayState,
        weights: &C,
        permit: &mut SpellLanePermit,
    ) -> Result<Option<UseOutcome>, ()> {
        // This existing scope read runs under the same spell lane as every Ground
        // writer. Conservatively require two source pads with no durable Ground
        // items at all; the native reserved slots then block concurrent actors.
        let ground = self
            .root
            .read_item_loop_ground_for_scope(fence.runtime_scope)
            .await
            .map_err(|_| ())?;
        if ground
            .iter()
            .any(|entry| entry.item.ground.spatial_position.len() != 10)
        {
            return Ok(outcome(UseDisposition::NotPossible));
        }
        let batch = {
            let mut runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err()
                || runtime.binding().runtime_scope() != fence.runtime_scope
                || runtime.binding().scope_generation() != fence.scope_ownership_generation
                || !runtime
                    .bound_attacker_lease(actor, command.game_session_id)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    })
            {
                return Ok(outcome(UseDisposition::StaleState));
            }
            let pads = crate::durability::item_craft::WOLF_PADS;
            let mut available = Vec::new();
            for position in pads {
                let cell = crate::content::LogicalCell {
                    x: position.x,
                    y: position.y,
                    z: i32::from(position.floor),
                };
                if !matches!(
                    self.movement_cells
                        .index()
                        .lookup(self.movement_cells.scope(), cell),
                    Ok(crate::content::CollisionClass::Walkable)
                ) || !runtime
                    .companion_creation_available(position)
                    .unwrap_or(false)
                {
                    continue;
                }
                // Ground's ordinary axis and spell-field axis are both recognized.
                let other = MovementLocalPosition {
                    floor: -position.floor,
                    ..position
                };
                if ground.iter().any(|entry| {
                    entry.item.ground.spatial_position == spatial(position)
                        || entry.item.ground.spatial_position == spatial(other)
                }) {
                    continue;
                }
                available.push(position);
            }
            if available.len() < 2 {
                return Ok(outcome(UseDisposition::NotPossible));
            }
            let mut selected = Vec::with_capacity(2);
            for purpose in 1..=2_u8 {
                let picked = crate::durability::item_craft::source_station_draw(
                    *command.game_session_id.as_bytes(),
                    command.command_id,
                    request.source,
                    purpose,
                    u64::try_from(available.len()).map_err(|_| ())?,
                )
                .ok_or(())?;
                selected.push(available.remove(usize::try_from(picked).map_err(|_| ())?));
            }
            let binding =
                crate::durability::item_craft::source_station_actor_binding(&request).ok_or(())?;
            let monsters = selected
                .into_iter()
                .map(|position| (crate::durability::item_craft::WOLF, position))
                .collect::<Vec<_>>();
            let mut batch = runtime
                .prepare_native_actor_batch(actor, command.game_session_id, binding, &monsters)
                .map_err(|_| ())?;
            runtime
                .reserve_native_actor_batch(&mut batch)
                .map_err(|_| ())?;
            batch
        };
        let proof = match batch.proof() {
            Ok(proof) => proof,
            Err(_) => {
                self.runtime
                    .lock()
                    .await
                    .rollback_native_actor_batch(&batch)
                    .map_err(|_| ())?;
                return Err(());
            }
        };
        let pending = PendingWorldStation {
            actor,
            session: command.game_session_id,
            fence,
            request: request.clone(),
            batch,
        };
        let mut window = permit.open_commit_window(pending, park);
        // No runtime or spell-state guard survives this durable await.
        let result = self
            .root
            .craft_item_with_source_actors(
                self.character,
                self.holder,
                fence,
                request,
                native,
                weights,
                &mut window,
                &proof,
            )
            .await;
        match result {
            Ok(Some(result)) => {
                let publish = result == UseDisposition::Committed && window.commit_called();
                let current = if publish {
                    self.world_station_actor_current(window.attempt()).await?
                } else {
                    false
                };
                let mut runtime = self.runtime.lock().await;
                let finished = if publish && current {
                    runtime.install_native_actor_batch(&window.attempt().batch)
                } else {
                    runtime.rollback_native_actor_batch(&window.attempt().batch)
                };
                if finished.is_err() {
                    window.park();
                    return Err(());
                }
                drop(runtime);
                if publish && current {
                    window.install();
                } else {
                    window.release();
                }
                if result == UseDisposition::Committed {
                    self.refresh_terrain_tools_in_lane(permit).await;
                }
                Ok(outcome(result))
            }
            Ok(None) => {
                window.park();
                Err(())
            }
            Err(_) => {
                match window.reclaim_uncommitted() {
                    Ok(pending) => {
                        self.runtime
                            .lock()
                            .await
                            .rollback_native_actor_batch(&pending.batch)
                            .map_err(|_| ())?;
                    }
                    Err(window) => window.park(),
                }
                Err(())
            }
        }
    }
    pub(in crate::gameplay_transport) async fn resolve_world_station(
        &self,
        permit: &mut SpellLanePermit,
        pending: PendingWorldStation,
    ) {
        let window = permit.open_commit_window(pending, park);
        let Some(native) = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
        else {
            window.park();
            return;
        };
        let result = self
            .root
            .craft_item(
                window.permit(),
                self.character,
                self.holder,
                window.attempt().fence,
                window.attempt().request.clone(),
                native,
                &NativeItemWeights(native),
                true,
            )
            .await;
        let Ok(result) = result else {
            window.park();
            return;
        };
        let committed = result == Some(UseDisposition::Committed);
        let current = if committed {
            match self.world_station_actor_current(window.attempt()).await {
                Ok(current) => current,
                Err(()) => {
                    window.park();
                    return;
                }
            }
        } else {
            false
        };
        let mut runtime = self.runtime.lock().await;
        let finished = if committed && current {
            runtime.install_native_actor_batch(&window.attempt().batch)
        } else {
            runtime.rollback_native_actor_batch(&window.attempt().batch)
        };
        if finished.is_err() {
            drop(runtime);
            window.park();
            return;
        }
        drop(runtime);
        if committed && current {
            window.install();
            self.refresh_terrain_tools_in_lane(permit).await;
        } else {
            window.release();
        }
    }
}
