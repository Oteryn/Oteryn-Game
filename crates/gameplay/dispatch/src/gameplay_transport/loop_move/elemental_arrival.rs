//! Native AddItem ingress and owner-cycle realization for Elemental Spheres.
mod realization;
use super::*;
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::spell_items_abi::QualifiedItemDefinition;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::map::boot::BundleMap;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::elemental_spheres::{self, ArrivalPlan};
use realization::RealizationGuard;

pub(super) struct ObservedArrival {
    pub plan: ArrivalPlan,
    pub qualified: QualifiedItemDefinition,
    pub fields: Vec<crate::durability::item_loop_move::AltarFieldRead>,
}

pub(super) fn corpse_facts(
    native: &NativeGameplayState,
    definition: &TypedDefinitionRef,
) -> Option<ItemDefinitionFacts> {
    if !elemental_spheres::rules()
        .iter()
        .any(|r| r.corpse == definition.production_key)
    {
        return None;
    }
    let q = QualifiedItemDefinition::from_native_policy(
        native.item_policy(&definition.production_key, &definition.revision_ref)?,
    )
    .ok()?;
    (q.materializable
        && q.movable
        && q.ground_destination
        && !q.inventory_destination
        && q.stack_maximum == 1)
        .then_some(ItemDefinitionFacts {
            definition: definition.clone(),
            weight: None,
            stack: crate::durability::item_transfer::ItemStackClass::NonStackable,
            container_capacity: q.container_capacity.map(u32::from),
            container_slot_equip_pattern: false,
        })
}

fn source<'a>(
    ground: &LoopGroundOverlay,
    index: &'a ItemDefinitionIndex,
    item: &[u8; 16],
) -> Option<(&'a str, &'a str)> {
    let facts = ground.facts.get(item)?;
    if facts.pickupable {
        return None;
    }
    let MapDefinition::Item(reference) = facts.definition else {
        return None;
    };
    let (key, rev) = index.definition_at(reference)?;
    elemental_spheres::rules()
        .iter()
        .any(|r| r.corpse == key)
        .then_some((key, rev))
}

/// Native corpse persistence uses the spell frame; command 9 uses native floors.
/// The source projection retains its durable bytes; only the destination changes frame.
pub(super) fn normalize(
    request: &mut LoopMoveRequest,
    ground: &LoopGroundOverlay,
    index: &ItemDefinitionIndex,
) -> Option<TypedDefinitionRef> {
    let (key, revision) = source(ground, index, &request.item)?;
    request.pickup_cell.as_ref()?;
    let LoopDestination::Ground(to) = &mut request.destination else {
        return None;
    };
    flip_floor(&mut to.spatial_position)?;
    to.map_revision = sha256_revision(Sha256::digest(to.map_revision.as_bytes()).into());
    Some(TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: revision.into(),
    })
}
fn flip_floor(cell: &mut [u8]) -> Option<()> {
    let floor = i16::from_be_bytes(cell.get(8..10)?.try_into().ok()?).checked_neg()?;
    cell.get_mut(8..10)?.copy_from_slice(&floor.to_be_bytes());
    Some(())
}

fn map_item(
    map: &BundleMap,
    index: &ItemDefinitionIndex,
    pos: TilePos,
    key: &str,
) -> Option<(u64, u64)> {
    if map.facts.house_tile(pos) {
        return None;
    }
    let facts = map.facts.pinned();
    let tile = view::compose_tile(&map.overlay, &facts, pos).ok()??;
    for entry in tile.entries {
        let matches = match entry.facts.definition {
            MapDefinition::Item(reference) => index
                .definition_at(reference)
                .is_some_and(|(found, _)| found == key),
            MapDefinition::Terrain(_) => {
                entry.facts.appearance_id == view::appearance_id(key, None)
            }
        };
        if !matches {
            continue;
        }
        match entry.source {
            view::EntrySource::Added(id) => {
                return map.facts.quest_callback_presence_readset(pos, id);
            }
            view::EntrySource::Base { .. } => {
                let view::MapEntryKey::Base(key) = entry.source.key() else {
                    continue;
                };
                return Some((key, map.facts.object_revision(key)));
            }
            _ => continue,
        }
    }
    None
}

pub(super) fn observe(
    runtime: &mut ChannelRuntimeV1,
    map: &BundleMap,
    index: &ItemDefinitionIndex,
    native: &NativeGameplayState,
    ground: &LoopGroundOverlay,
    to: WorldTilePosition,
    definition: &TypedDefinitionRef,
) -> Option<ObservedArrival> {
    runtime.owner_fence().ok()?;
    if runtime.content_pin().server_artifact_digest() != native.source_digest() {
        return None;
    }
    let rule = elemental_spheres::rules()
        .iter()
        .find(|r| r.corpse == definition.production_key)?;
    let pos = TilePos {
        x: to.x.try_into().ok()?,
        y: to.y.try_into().ok()?,
        floor: to.floor.try_into().ok()?,
    };
    map_item(map, index, pos, rule.altar)?;
    native.install_companion_policies(runtime).ok()?;
    runtime.companion_policy(rule.creature).ok()?;
    let census = runtime.current_live_creature_candidates(4096).ok()?;
    let boss = census
        .into_iter()
        .any(|actor| runtime.matches_live_creature_identity(actor, rule.creature.as_bytes()));
    let mut fields = Vec::new();
    let plan = elemental_spheres::plan_arrival(rule.altar, rule.corpse, boss, |p, key| {
        let pos = TilePos {
            x: p.x,
            y: p.y,
            floor: p.floor,
        };
        if let Some(entry) = map_item(map, index, pos, key) {
            return Some(entry);
        }
        // Durable rune fields live in the item overlay. Fence their base tile
        // here, then lock and re-read the exact field in the arrival transaction.
        let tile = ground.overlay.tile(pos)?;
        let field = tile.added().iter().find_map(|entry| {
            let AddedItem::Ground(item) = entry.item() else {
                return None;
            };
            (item.definition.production_key == key
                && ground
                    .facts
                    .get(&item.item_instance_id)
                    .is_some_and(|f| !f.pickupable))
            .then_some(item)
        })?;
        let base = view::placement_key(pos, 0)?;
        fields.push(crate::durability::item_loop_move::AltarFieldRead::new(
            field.item_instance_id,
            ground_cell(i32::from(pos.x), i32::from(pos.y), -i16::from(pos.floor)),
        ));
        Some((base, map.facts.object_revision(base)))
    })?;
    map.facts.with_quest_readset(&plan.entries, || ())?;
    let qualified = QualifiedItemDefinition::from_native_policy(
        native.item_policy(&definition.production_key, &definition.revision_ref)?,
    )
    .ok()?;
    Some(ObservedArrival {
        plan,
        qualified,
        fields,
    })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn drain_elemental_altar_bosses(&self) {
        let Some(permit) = self.spell_lane_permit().await else {
            return;
        };
        self.realize_elemental_altar_bosses(&permit).await;
    }
    pub(super) async fn realize_elemental_altar_bosses(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) {
        let Some(map) = self.bundle_map() else {
            return;
        };
        let Some(native) = self.active_generation.and_then(|g| g.native_gameplay()) else {
            return;
        };
        let (scope, generation) = {
            let runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err()
                || runtime.content_pin().server_artifact_digest() != native.source_digest()
            {
                return;
            }
            runtime.encounter_owner_scope()
        };
        let physical_map =
            sha256_revision(Sha256::digest(map_revision(map.overlay.base()).as_bytes()).into());
        let Ok(epoch) = self.reset_epoch.try_into() else {
            return;
        };
        let Ok(pending) = self
            .root
            .read_pending_altar_bosses(
                permit,
                self.character,
                self.holder,
                scope,
                generation,
                epoch,
                native.source_digest(),
                &physical_map,
            )
            .await
        else {
            return;
        };
        // Drop settled/dead occurrences from the bounded volatile acknowledgement cache.
        if let Ok(mut actors) = self.states.quest_callbacks.elemental.lock() {
            actors.retain(|id, actor| {
                actor.runtime_scope() == scope
                    && actor.scope_generation() == generation
                    && pending.iter().any(|p| p.source.transaction_id == *id)
            });
        }
        for pending in pending {
            let Some(rule) = elemental_spheres::rules()
                .iter()
                .find(|r| r.key == pending.source.rule)
            else {
                continue;
            };
            let actor = {
                let mut runtime = self.runtime.lock().await;
                if runtime.owner_fence().is_err()
                    || runtime.encounter_owner_scope() != (scope, generation)
                    || runtime.content_pin().server_artifact_digest() != native.source_digest()
                {
                    return;
                }
                let Ok(census) = runtime.current_live_creature_candidates(4096) else {
                    continue;
                };
                if let Some((id, generation)) = pending.actor {
                    // A fresh committed ACK can release the exact source reservation
                    // even after losing the volatile actor memo. It never respawns.
                    if let Some(actor) = census.iter().find(|actor| {
                        actor.0.actor_local_id.0 == id
                            && actor.0.actor_local_generation.0 == generation
                    }) {
                        RealizationGuard::new(
                            *actor,
                            pending.source.transaction_id,
                            rule.creature,
                            native.source_digest(),
                        )
                        .release(&mut runtime);
                    }
                    continue;
                }
                let retained = self
                    .states
                    .quest_callbacks
                    .elemental
                    .lock()
                    .ok()
                    .and_then(|actors| actors.get(&pending.source.transaction_id).copied())
                    .or_else(|| {
                        census.iter().copied().find(|actor| {
                            // Recover only an already reserved exact occurrence. A live
                            // lookalike cannot be adopted as this pending arrival.
                            runtime.matches_live_creature_identity(*actor, rule.creature.as_bytes())
                                && runtime.actor_spell_reserved(*actor)
                                && RealizationGuard::new(
                                    *actor,
                                    pending.source.transaction_id,
                                    rule.creature,
                                    native.source_digest(),
                                )
                                .hold(&mut runtime)
                        })
                    });
                if let Some(actor) = retained {
                    if runtime.matches_live_creature_identity(actor, rule.creature.as_bytes()) {
                        RealizationGuard::new(
                            actor,
                            pending.source.transaction_id,
                            rule.creature,
                            native.source_digest(),
                        )
                        .hold(&mut runtime)
                        .then_some(actor)
                    } else {
                        None
                    }
                } else {
                    if census.iter().any(|actor| {
                        runtime.matches_live_creature_identity(*actor, rule.creature.as_bytes())
                    }) {
                        continue;
                    }
                    if native.install_companion_policies(&mut runtime).is_err() {
                        continue;
                    }
                    let Ok(policy) = runtime.companion_policy(rule.creature) else {
                        continue;
                    };
                    let monsters = [(
                        rule.creature,
                        policy.maximum_health,
                        MovementLocalPosition {
                            x: i32::from(rule.spawn.x),
                            y: i32::from(rule.spawn.y),
                            floor: -i16::from(rule.spawn.floor),
                        },
                    )];
                    let Ok(mut retained) = self.states.quest_callbacks.elemental.lock() else {
                        continue;
                    };
                    if retained.len() >= 4 {
                        continue;
                    }
                    // Complete the already committed source choice under independently
                    // current scope/epoch/pins. Requiring its historical charge revisions
                    // again would discard a pending summon after handoff or expiry.
                    let Ok(actors) = runtime.commit_quest_creature_spawn(&monsters) else {
                        continue;
                    };
                    let Some(actor) = actors.first().copied() else {
                        continue;
                    };
                    if !RealizationGuard::new(
                        actor,
                        pending.source.transaction_id,
                        rule.creature,
                        native.source_digest(),
                    )
                    .hold(&mut runtime)
                    {
                        let _ = runtime.remove_encounter_actor(actor);
                        continue;
                    }
                    retained.insert(pending.source.transaction_id, actor);
                    Some(actor)
                }
            };
            if let Some(actor) = actor {
                // Unknown ACK retains both exact actor and source reservation:
                // it cannot die before the latch knows which actor death releases it.
                if matches!(
                    self.root
                        .record_altar_boss_realization(
                            permit,
                            self.character,
                            self.holder,
                            pending.source.transaction_id,
                            actor,
                        )
                        .await,
                    Ok(true)
                ) {
                    let mut runtime = self.runtime.lock().await;
                    RealizationGuard::new(
                        actor,
                        pending.source.transaction_id,
                        rule.creature,
                        native.source_digest(),
                    )
                    .release(&mut runtime);
                }
            }
        }
    }
}
// M49 live item-arrival adapter; the lane owns all nine map observations through SQL.
use crate::durability::{
    DurabilityError,
    item_loop_move::{ElementalArrivalProbe, ElementalArrivalTarget, LoopMoveReceipt},
    spell_owner_commit::SpellLanePermit,
};
use oteryn_gameplay_quests::quest::{
    elemental_arrival::{self as rules, ArrivalFence},
    world_callbacks::Position,
};

type MoveCommit = Result<Result<LoopMoveReceipt, LoopMoveRefusal>, DurabilityError>;

pub(in crate::gameplay_transport) struct PendingElementalArrival {
    command: CommandRef,
    fence: CurrentCharacterItemFence,
    request: LoopMoveRequest,
    destination: WorldTilePosition,
    _reservation: crate::map::facts::ElementalArrivalReservation,
}
impl std::fmt::Debug for PendingElementalArrival {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingElementalArrival")
            .field("command", &self.command)
            .field("request", &self.request)
            .finish_non_exhaustive()
    }
}
fn park(pending: PendingElementalArrival) -> Box<dyn std::any::Any + Send> {
    Box::new(
        super::super::actor_spell::native_combat_cast::UnresolvedSpellCommit::ElementalArrival(
            Box::new(pending),
        ),
    )
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn commit_elemental_arrival(
        &self,
        permit: &mut SpellLanePermit,
        actor: ExactActorRef,
        command: UseCommand,
        native: &crate::content::native_gameplay::NativeGameplayState,
        request: &LoopMoveRequest,
        ground: Option<&LoopGroundOverlay>,
    ) -> Option<MoveCommit> {
        let LoopDestination::Ground(to) = &request.destination else {
            return None;
        };
        let ground = ground?;
        let source = ground.facts.get(&request.item)?;
        if !rules::RECIPES
            .iter()
            .any(|recipe| recipe.corpse == source.appearance_id)
        {
            return None;
        }
        let at =
            crate::durability::spell_items_abi::decode_ground_cell(&to.spatial_position).ok()?;
        let position = Position {
            x: at.x.try_into().ok()?,
            y: at.y.try_into().ok()?,
            floor: at.z.try_into().ok()?,
        };
        let destination = WorldTilePosition {
            x: at.x,
            y: at.y,
            floor: at.z.try_into().ok()?,
        };
        let fence = command.item_fence?;
        let command_ref = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).ok()?,
        );
        let map = self.bundle_map()?;
        let weights = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        let corpse = match self
            .root
            .item_loop_elemental_corpse(
                permit,
                self.character,
                self.holder,
                fence,
                &weights,
                ElementalArrivalProbe {
                    command: command_ref,
                    item: request.item,
                    position,
                },
            )
            .await
        {
            Ok(Some(corpse)) => corpse,
            Ok(None) => return None,
            Err(error) => return Some(Err(error)),
        };
        let (plan, reservation) = {
            let mut runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if runtime.owner_fence().is_err()
                || runtime.binding().runtime_scope() != fence.runtime_scope
                || runtime.binding().scope_generation() != fence.scope_ownership_generation
                || states
                    .get(&runtime, actor, command.game_session_id)
                    .is_none()
                || states.is_dead(actor)
                || states.has_pending_spell_commit(actor, command.game_session_id)
                || native_for_move(&runtime, self.active_generation, false, false).is_none()
            {
                return Some(Ok(Err(LoopMoveRefusal::Stale)));
            }
            let wake = Arc::clone(&states.owner_wake);
            drop(states);
            let map_digest: [u8; 32] =
                Sha256::digest(map_revision(map.overlay.base()).as_bytes()).into();
            if runtime.content_pin().map_revision_digest() != map_digest {
                return Some(Ok(Err(LoopMoveRefusal::Stale)));
            }
            let actor_at =
                super::super::bundle_map_position(&mut runtime, self.movement_cells(), actor)?;
            if !in_reach(
                (actor_at.at.x, actor_at.at.y, actor_at.at.floor),
                (at.x, at.y, destination.floor),
            ) {
                return Some(Ok(Err(LoopMoveRefusal::Stale)));
            }
            let binding = runtime.binding();
            let channel = binding.channel_id()?;
            let recipe = rules::RECIPES
                .iter()
                .find(|recipe| recipe.corpse == corpse.item)?;
            let busy = runtime.boss_spawns().iter().any(|spawn| {
                spawn.creature == recipe.creature
                    && spawn
                        .actor
                        .is_some_and(|actor| runtime.contains_live_creature(actor))
            });
            let scope = ArrivalFence {
                world: *binding.world_id().as_bytes(),
                channel: *channel.as_bytes(),
                scope_generation: binding.scope_generation().get(),
                map: map.overlay.base().digest(),
                content: native.source_digest(),
                reset_epoch: self.reset_epoch,
            };
            let ground_facts = ground.facts(&map.facts);
            let ground_view = Some((
                ground.overlay(),
                &ground_facts as &dyn crate::map::view::MapFacts,
            ));
            let plan = map.facts.observe_elemental_arrival_with_ground(
                map.overlay.base(),
                scope,
                TilePos {
                    x: position.x,
                    y: position.y,
                    floor: position.floor,
                },
                corpse,
                busy,
                ground_view,
            )?;
            let Some(reservation) = map.facts.reserve_elemental_arrival_with_ground(
                map.overlay.base(),
                &plan,
                wake,
                ground_view,
            ) else {
                return Some(Ok(Err(LoopMoveRefusal::Stale)));
            };
            (plan, reservation)
        };
        let target = ElementalArrivalTarget::new(plan);
        let pending = PendingElementalArrival {
            command: command_ref,
            fence,
            request: request.clone(),
            destination,
            _reservation: reservation,
        };
        let mut window = permit.open_commit_window(pending, park);
        let result = self
            .root
            .commit_item_loop_elemental_arrival(
                &mut window,
                self.character,
                self.holder,
                fence,
                command_ref,
                &weights,
                request.clone(),
                target,
            )
            .await;
        match &result {
            Ok(Ok(_)) => {
                drop(window.install());
            }
            Ok(Err(_)) => {
                drop(window.release());
            }
            Err(_) => {} // Drop parks the entire reservation until the exact receipt is known.
        }
        Some(result)
    }

    pub(in crate::gameplay_transport) async fn resolve_elemental_arrival(
        &self,
        permit: &mut SpellLanePermit,
        pending: PendingElementalArrival,
    ) {
        let window = permit.open_commit_window(pending, park);
        let pending = window.attempt();
        let result = self
            .root
            .reconcile_item_loop_move(
                self.character,
                pending.fence.character_id,
                pending.command,
                pending.request.clone(),
            )
            .await;
        match result {
            Ok(Some(Ok(_))) => {
                self.states.loop_ground.invalidate();
                self.states.browse_fields.committed_move(
                    ItemKey::Instance(pending.request.item),
                    Some(pending.destination),
                );
                drop(window.install());
            }
            Ok(_) => {
                drop(window.release());
            }
            Err(_) => {} // Cancellation and unavailable truth keep the lane fenced.
        }
    }
}

#[cfg(test)]
mod normalization_tests {
    use super::*;
    use crate::durability::item_loop_move::LoopGroundItem;
    use crate::durability::item_mint::{GroundItemInstance, GroundPlacement};
    use crate::foundation::{ChannelId, RuntimeScopeRefV1, WorldId};

    #[test]
    fn elemental_move_preserves_projected_source_custody_in_both_frames()
    -> Result<(), Box<dyn std::error::Error>> {
        let id = |tag| [1, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag];
        let world = WorldId::decode(&id(1))?;
        let scope = RuntimeScopeRefV1::channel(world, ChannelId::decode(&id(2))?);
        let index = ItemDefinitionIndex::decode(br#"{"schema":"OTERYN_NATIVE_ITEM_KEYS/v1","records":[["oteryn:item.tibia.i3031","definition-r1"],["oteryn:item.tibia.i8139","definition-r1"]]}"#)?;
        let (bytes, pins) = crate::map::boot::tests::bundle_with_item_ids(
            ("oteryn:item.tibia.i8139", 1),
            ("oteryn:item.tibia.i3031", 0),
        );
        let base = Arc::new(crate::map::load(&bytes, &pins)?);
        let pin = ([7; 32], [8; 32], 1);
        let map = map_revision(&base);
        let native_map = sha256_revision(Sha256::digest(map.as_bytes()).into());
        let definition = TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i8139".into(),
            revision_ref: "definition-r1".into(),
        };
        // A new corpse retains runtime coordinates. A prior ordinary drop may
        // instead have persisted map coordinates; both are exact custody facts.
        for (source_map, source_floor) in [(&native_map, 7), (&map, -7)] {
            let source_cell = ground_cell(1, 0, source_floor);
            let ground = LoopGroundOverlay::build_with_native(
                Arc::clone(&base),
                scope,
                &index,
                pin,
                vec![LoopGroundItem {
                    native_corpse: true,
                    source_native_ground: false,
                    source_plain_corpse: false,
                    field_collision: None,
                    stack_ordinal: 1,
                    item: GroundItemInstance {
                        item_instance_id: id(3),
                        world_id: world,
                        runtime_scope: scope,
                        definition: definition.clone(),
                        quantity: 1,
                        runtime_scope_ownership_generation: 1,
                        ground: GroundPlacement {
                            spatial_position: source_cell.clone(),
                            corpse_ref: vec![1],
                            map_revision: source_map.clone(),
                            content_revision: sha256_revision(pin.0),
                            native_room_placement_context: pin.1.to_vec(),
                        },
                        minted_transaction_id: id(4),
                    },
                }],
                None,
            );
            let mut request = LoopMoveRequest {
                item: id(3),
                count: None,
                pickup_cell: ground.pickup_cell(&id(3)),
                destination: LoopDestination::Ground(LoopGround {
                    spatial_position: ground_cell(0, 0, -7),
                    map_revision: map.clone(),
                    content_revision: sha256_revision(pin.0),
                    placement_context: pin.1.to_vec(),
                }),
            };
            assert_eq!(
                normalize(&mut request, &ground, &index),
                Some(definition.clone())
            );
            assert_eq!(request.pickup_cell, Some(source_cell));
            let LoopDestination::Ground(destination) = request.destination else {
                return Err("elemental destination changed custody".into());
            };
            assert_eq!(destination.spatial_position, ground_cell(0, 0, 7));
            assert_eq!(destination.map_revision, native_map);
        }
        Ok(())
    }
}
