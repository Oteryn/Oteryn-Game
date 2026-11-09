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
/// Preserve that physical frame for both source and destination, including ordinary moves.
pub(super) fn normalize(
    request: &mut LoopMoveRequest,
    ground: &LoopGroundOverlay,
    index: &ItemDefinitionIndex,
) -> Option<TypedDefinitionRef> {
    let (key, revision) = source(ground, index, &request.item)?;
    let pickup = request.pickup_cell.as_mut()?;
    flip_floor(pickup)?;
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
