//! Pure paired source projections follow committed actor presence, including creature end paths.
use super::*;

pub(super) fn synchronize(
    runtime: &mut ChannelRuntimeV1,
    map: &BundleMap,
    callbacks: &[Callback],
    cells: &crate::content::NativeEntryMovementCells,
) -> bool {
    for entry in callbacks
        .iter()
        .filter(|c| c.occupancy_driven && c.trigger == Trigger::StepIn)
    {
        let Some(exit) = callbacks.iter().find(|c| {
            c.occupancy_driven
                && c.trigger == Trigger::StepOut
                && c.selector == entry.selector
                && c.quest == entry.quest
        }) else {
            return false;
        };
        let Some(at) = entry.selector.position.map(tile) else {
            return false;
        };
        let Some(t) = map.overlay.base().tile(at.x, at.y, at.floor) else {
            continue;
        };
        let keys: Vec<_> = t
            .ids()
            .iter()
            .zip(t.depths())
            .filter(|(_, d)| **d == 0)
            .enumerate()
            .filter_map(|(o, _)| placement_key(at, u8::try_from(o).ok()?))
            .filter(|key| matches(map, entry, *key))
            .collect();
        let [key] = keys.as_slice() else {
            if keys.is_empty() {
                continue;
            }
            return false;
        };
        let Ok(occupied) = runtime.committed_world_cell_occupied(MovementLocalPosition {
            x: i32::from(at.x),
            y: i32::from(at.y),
            floor: -i16::from(at.floor),
        }) else {
            return false;
        };
        let callback = if occupied { entry } else { exit };
        let mut group_occupied = occupied;
        if let Some(required) = &entry.occupancy_requirement {
            group_occupied = true;
            for &position in &required.positions {
                let read = if required.players_only {
                    runtime.committed_player_cell_occupied(local(position))
                } else {
                    runtime.committed_world_cell_occupied(local(position))
                };
                match read {
                    Ok(true) => {}
                    Ok(false) => group_occupied = false,
                    Err(_) => return false,
                }
            }
        }
        let close = !group_occupied;
        let Some((_, ordinal)) = crate::map::view::placement_of(*key) else {
            return false;
        };
        let Some((id, _)) = t
            .ids()
            .iter()
            .zip(t.depths())
            .filter(|(_, d)| **d == 0)
            .nth(usize::from(ordinal))
        else {
            return false;
        };
        let Some(before) = map.facts.base_entry(at, ordinal, *id) else {
            return false;
        };
        let Some(appearance) = callback.effects.transform else {
            return false;
        };
        let Some(after) = map.facts.quest_callback_replacement(*key, appearance) else {
            return false;
        };
        let mut changed = before.appearance_id != after.appearance_id;
        let mut replacements = Vec::new();
        let mut removed = Vec::new();
        for passage in &callback.passages {
            let p = tile(passage.position);
            let Some(t) = map.overlay.base().tile(p.x, p.y, p.floor) else {
                return false;
            };
            let candidates: Vec<_> = t
                .ids()
                .iter()
                .zip(t.depths())
                .filter(|(_, d)| **d == 0)
                .enumerate()
                .filter_map(|(o, (id, _))| {
                    let o = u8::try_from(o).ok()?;
                    let k = placement_key(p, o)?;
                    let f = map.facts.base_entry(p, o, *id)?;
                    ((f.appearance_id == passage.appearance
                        || passage.replacement == Some(f.appearance_id))
                        && !map.facts.house_tile(p))
                    .then_some((k, f.appearance_id))
                })
                .collect();
            let [(k, current)] = candidates.as_slice() else {
                return false;
            };
            let Some(replacement) = passage.replacement else {
                changed |= map.facts.base_hidden(*k) != !close;
                removed.push(*k);
                continue;
            };
            let wanted = if close {
                passage.appearance
            } else {
                replacement
            };
            let Some(facts) = map.facts.quest_callback_replacement(*k, wanted) else {
                return false;
            };
            changed |= *current != wanted;
            replacements.push((*k, facts, !close));
        }
        let mut items = Vec::new();
        for item in &entry.occupancy_items {
            let at = tile(item.position);
            if map.facts.house_tile(at) || map.overlay.base().tile(at.x, at.y, at.floor).is_none() {
                return false;
            }
            let Some(facts) = map.facts.movement_item_facts(item.appearance) else {
                return false;
            };
            use oteryn_world_bundle::bundle::TerrainKind;
            if facts.pickupable
                || !matches!(
                    facts.terrain_kind,
                    Some(TerrainKind::Common | TerrainKind::Ground | TerrainKind::Border)
                )
            {
                return false;
            }
            items.push((at, facts));
        }
        let projection = if items.is_empty() {
            None
        } else {
            changed |= !map.facts.quest_occupancy_items_match(*key, &items, close);
            Some(crate::map::facts::QuestCallbackProjection::OccupancyItems { entries: items })
        };
        if !changed {
            continue;
        }
        // Every wall occupant is qualified before either its movement or the map
        // projection is published. Both owners remain serialized by lane/runtime locks.
        let closed = crate::map::floor::BundleDoors(map.facts.doors());
        let mut ejections = Vec::new();
        if close
            && let Some(destination) = entry.occupancy_requirement.as_ref().and_then(|r| r.eject)
        {
            let Ok(actors) = runtime.positioned_melee_census() else {
                return false;
            };
            for (actor, position, _) in actors {
                if entry
                    .passages
                    .iter()
                    .any(|p| local(p.position) == position.position())
                {
                    let Ok(proof) = crate::map::floor::prepare_occupancy_ejection(
                        runtime,
                        cells,
                        actor,
                        position,
                        local(destination),
                        &closed,
                    ) else {
                        return false;
                    };
                    ejections.push(proof);
                }
            }
        }
        if !map.facts.apply_quest_callback(
            *key,
            map.facts.object_revision(*key),
            after,
            &removed,
            &replacements,
            close,
            projection.as_ref(),
        ) {
            return false;
        }
        for proof in ejections {
            if runtime.commit_source_step(proof).is_err() {
                return false;
            }
        }
    }
    true
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) fn synchronize_occupancy_locked(
        &self,
        runtime: &mut ChannelRuntimeV1,
        map: &BundleMap,
    ) -> bool {
        synchronize(runtime, map, self.callback_content(), self.movement_cells)
    }
    pub(in crate::gameplay_transport) async fn reconcile_occupancy_callbacks(&self) -> bool {
        let Some(map) = self.bundle_map() else {
            return true;
        };
        let callbacks = self.callback_content();
        // Unrelated small/legacy Worlds have no bound occupancy source.
        if !callbacks.iter().any(|c| {
            c.occupancy_driven
                && c.selector
                    .position
                    .is_some_and(|p| map.overlay.base().tile(p.x, p.y, p.floor).is_some())
        }) {
            return true;
        }
        let Some(_permit) = self.spell_lane_permit().await else {
            return false;
        };
        let mut runtime = self.runtime.lock().await;
        synchronize(&mut runtime, map, callbacks, self.movement_cells)
    }
}
