//! M50: atomic cross-floor movement of fixed map scenery, never portable custody.
use super::*;

/// One fixed scenery mutation observed under the same Channel owner turn.
/// Portable ItemInstances are deliberately outside this map projection API.
#[derive(Clone, Copy, Debug)]
pub struct QuestMapItemMove {
    pub from: TilePos,
    pub to: TilePos,
    pub entry: view::ComposedEntry,
    pub expected_revision: u64,
    pub appearance: u16,
}

fn source_key(source: view::EntrySource) -> Option<u64> {
    match source {
        view::EntrySource::Base { placement_key, .. } => Some(placement_key),
        view::EntrySource::Added(id) if id.get() >> 62 == 2 => Some(id.get()),
        _ => None,
    }
}

impl BundleFacts {
    /// Compare the complete bounded scenery batch before publishing any move.
    /// Moving a base object hides its exact ordinal and creates one Channel-local
    /// identity; subsequent moves retain that identity and its revision fence.
    pub fn apply_quest_map_item_batch(
        &self,
        base: &super::super::super::WorldBase,
        moves: &[QuestMapItemMove],
    ) -> bool {
        if moves.is_empty() || moves.len() > 8 {
            return false;
        }
        let Ok(rotations) = self.rotations.lock().map(|s| Arc::clone(&s)) else {
            return false;
        };
        let Ok(mut states) = self.callbacks.lock() else {
            return false;
        };
        if !states.has_capacity(moves.len() * 4) {
            return false;
        }
        let pinned = PinnedFacts {
            facts: self,
            callbacks: Some(Arc::clone(&states)),
            doors: self.doors.snapshot(),
            rotations: Some(rotations),
        };
        let mut seen = BTreeSet::new();
        let mut prepared = Vec::new();
        for movement in moves {
            let Some(key) = source_key(movement.entry.source) else {
                return false;
            };
            if !seen.insert(key)
                || states.position_reserved(movement.from)
                || states.position_reserved(movement.to)
                || self.house_tile(movement.from)
                || self.house_tile(movement.to)
                || movement.entry.facts.pickupable
                || movement.expected_revision == u64::MAX
                || pinned.object_revision(key) != movement.expected_revision
                || base
                    .tile(movement.to.x, movement.to.y, movement.to.floor)
                    .is_none()
            {
                return false;
            }
            let mut observed = Vec::new();
            if view::compose_into(base, None, &pinned, movement.from, &mut observed).is_err()
                || !observed.contains(&movement.entry)
            {
                return false;
            }
            let Some(mut after) = self.movement_item_facts(movement.appearance) else {
                return false;
            };
            if after.pickupable || self.override_blocks(after).is_none() {
                return false;
            }
            after.bound = movement.entry.facts.bound;
            after.count = movement.entry.facts.count;
            after.sub_type = movement.entry.facts.sub_type;
            prepared.push((movement, key, after));
        }
        let mut staged = (**states).clone();
        for (movement, key, after) in prepared {
            let source = match movement.entry.source {
                view::EntrySource::Base { .. } if movement.from == movement.to => {
                    staged
                        .changed
                        .insert(key, (movement.expected_revision + 1, after));
                    staged
                        .passable
                        .insert(key, self.override_blocks(after) == Some(false));
                    continue;
                }
                view::EntrySource::Base { .. } => {
                    staged.hidden.insert(key);
                    staged.passable.remove(&key);
                    let Some(sequence) = staged.movement_sequence.checked_add(1) else {
                        return false;
                    };
                    let Some(id) = EntryId::callback(sequence) else {
                        return false;
                    };
                    staged.movement_sequence = sequence;
                    staged
                        .changed
                        .insert(key, (movement.expected_revision + 1, after));
                    view::EntrySource::Added(id)
                }
                view::EntrySource::Added(id) if movement.from == movement.to => {
                    let Some(entry) =
                        staged
                            .movement_items
                            .get_mut(&movement.from)
                            .and_then(|entries| {
                                entries
                                    .iter_mut()
                                    .find(|entry| entry.source == movement.entry.source)
                            })
                    else {
                        return false;
                    };
                    entry.facts = after;
                    staged
                        .changed
                        .insert(id.get(), (movement.expected_revision + 1, after));
                    continue;
                }
                view::EntrySource::Added(id) => {
                    let Some(entries) = staged.movement_items.get_mut(&movement.from) else {
                        return false;
                    };
                    entries.retain(|entry| entry.source != movement.entry.source);
                    if entries.is_empty() {
                        staged.movement_items.remove(&movement.from);
                    }
                    view::EntrySource::Added(id)
                }
                _ => return false,
            };
            let Some(new_key) = source_key(source) else {
                return false;
            };
            staged
                .changed
                .insert(new_key, (movement.expected_revision + 1, after));
            staged
                .movement_items
                .entry(movement.to)
                .or_default()
                .push(view::ComposedEntry {
                    source,
                    facts: after,
                });
        }
        for pos in moves.iter().flat_map(|m| [m.from, m.to]) {
            if staged.movement_items.get(&pos).is_some_and(|entries| {
                entries
                    .iter()
                    .any(|entry| self.override_blocks(entry.facts) != Some(false))
            }) {
                staged.batch_blockers.insert(pos);
            } else {
                staged.batch_blockers.remove(&pos);
            }
        }
        *states = Arc::new(staged);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::boot::tests as fixture;

    #[test]
    fn qm50_map_item_batch_crosses_floors_and_refuses_stale_or_partial_replay() -> Result<(), String>
    {
        let (bytes, mut pins) = fixture::bundle_with_item_states(
            ("oteryn:item.tibia.i17938", 0),
            ("oteryn:item.tibia.i17939", 1),
            &[("oteryn:item.tibia.i17942", 2)],
        );
        let mut data = oteryn_world_bundle::bundle::read(&bytes).map_err(|e| e.to_string())?;
        let mut upper = data.sectors[0].clone();
        upper.floor = -6;
        data.sectors.push(upper);
        data.manifest.world.floors.push(-6);
        data.manifest.world.floors.sort_unstable();
        data.manifest.world.floors.dedup();
        let bytes =
            oteryn_world_bundle::writer::write(&data.manifest, &data.sectors, &Default::default())
                .map_err(|e| e.to_string())?;
        pins.digest = oteryn_world_bundle::bundle::read(&bytes)
            .map_err(|e| e.to_string())?
            .digest;
        let base = crate::map::load(&bytes, &pins).map_err(|e| e.to_string())?;
        let items = |key: &str| {
            let reference = match key {
                "oteryn:item.tibia.i17938" => 1,
                "oteryn:item.tibia.i17939" => 2,
                "oteryn:item.tibia.i17942" => 3,
                _ => return None,
            };
            Some(ItemDefinition {
                reference: NonZeroU32::new(reference)?,
                solid: Some(false),
                blocks_projectile: false,
                pickupable: false,
                door: false,
            })
        };
        let mut facts =
            BundleFacts::palette(&data.manifest, items).map_err(|e| format!("{e:?}"))?;
        for sector in &data.sectors {
            facts.push(&data.manifest, sector, &mut BTreeSet::new());
        }
        facts.finish();
        let from = TilePos {
            x: 6,
            y: 0,
            floor: -7,
        };
        let to = TilePos { floor: -6, ..from };
        let mut entries = Vec::new();
        view::compose_into(&base, None, &facts, from, &mut entries)
            .map_err(|e| format!("{e:?}"))?;
        let entry = *entries
            .iter()
            .find(|e| e.facts.appearance_id == 17938)
            .ok_or("platform")?;
        let movement = QuestMapItemMove {
            from,
            to,
            entry,
            expected_revision: 0,
            appearance: 17942,
        };
        let neighbor = TilePos { x: 7, ..from };
        let mut other_entries = Vec::new();
        view::compose_into(&base, None, &facts, neighbor, &mut other_entries)
            .map_err(|e| format!("{e:?}"))?;
        let other = QuestMapItemMove {
            from: neighbor,
            entry: *other_entries
                .iter()
                .find(|e| e.facts.appearance_id == 17939)
                .ok_or("second platform item")?,
            ..movement
        };
        let bad = QuestMapItemMove {
            expected_revision: 1,
            ..other
        };
        assert!(!facts.apply_quest_map_item_batch(&base, &[movement, bad]));
        assert!(!facts.base_hidden(match entry.source {
            view::EntrySource::Base { placement_key, .. } => placement_key,
            _ => return Err("base".into()),
        }));
        assert!(facts.apply_quest_map_item_batch(&base, &[movement, other]));
        assert!(!facts.apply_quest_map_item_batch(&base, &[movement]));
        let landed = facts.movement_items(to);
        assert_eq!(landed.len(), 2);
        assert_eq!(landed[0].facts.appearance_id, 17942);
        let back = QuestMapItemMove {
            from: to,
            to: from,
            entry: landed[0],
            expected_revision: 1,
            appearance: 17938,
        };
        assert!(facts.apply_quest_map_item_batch(&base, &[back]));
        assert_eq!(facts.movement_items(to).len(), 1);
        assert_eq!(facts.movement_items(from)[0].source, landed[0].source);
        assert_eq!(facts.movement_items(from)[0].facts.appearance_id, 17938);
        Ok(())
    }
}
