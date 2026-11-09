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
    /// Plan the source's move-then-transform sequence against one fact snapshot.
    /// The Channel owner must remain held through `apply_quest_map_item_batch`.
    /// Missing optional frames are skipped, as in the source; the platform itself
    /// must exist. Multiple operations on one Item collapse to one revision CAS.
    pub fn prepare_quest_platform_batch(
        &self,
        base: &super::super::super::WorldBase,
        from: TilePos,
        to: TilePos,
        items: &[u16],
        transforms: &[(TilePos, u16, u16)],
    ) -> Option<Vec<QuestMapItemMove>> {
        if from == to
            || items.is_empty()
            || items.len() + transforms.len() > 8
            || self.house_tile(from)
            || self.house_tile(to)
            || transforms
                .iter()
                .any(|(pos, _, _)| *pos != from && *pos != to)
        {
            return None;
        }
        let pinned = self.pinned();
        let mut entries = Vec::new();
        let mut batch: Vec<QuestMapItemMove> = Vec::new();
        // Original entries precede arrivals in the destination stack, matching
        // Tile:getItemById after Item:moveTo. Each identity keeps its original CAS.
        let mut observed = Vec::new();
        for pos in [to, from] {
            base.tile(pos.x, pos.y, pos.floor)?;
            entries.clear();
            view::compose_into(base, None, &pinned, pos, &mut entries).ok()?;
            observed.extend(entries.iter().copied().map(|entry| (pos, entry)));
        }
        for (index, appearance) in items.iter().enumerate() {
            let selected = observed.iter().find(|(pos, entry)| {
                *pos == from
                    && entry.facts.appearance_id == *appearance
                    && !batch.iter().any(|m| m.entry.source == entry.source)
            });
            let Some(&(_, entry)) = selected else {
                if index == 0 {
                    return None;
                }
                continue;
            };
            let key = source_key(entry.source)?;
            batch.push(QuestMapItemMove {
                from,
                to,
                entry,
                expected_revision: pinned.object_revision(key),
                appearance: *appearance,
            });
        }
        for &(pos, before, after) in transforms {
            // Search stationary entries first, followed by the arrivals in move order.
            let stationary = observed.iter().find(|(at, entry)| {
                if let Some(m) = batch.iter().find(|m| m.entry.source == entry.source) {
                    m.from == m.to && m.to == pos && m.appearance == before
                } else {
                    *at == pos && entry.facts.appearance_id == before
                }
            });
            if let Some(&(at, entry)) = stationary {
                if let Some(m) = batch.iter_mut().find(|m| m.entry.source == entry.source) {
                    m.appearance = after;
                } else {
                    let key = source_key(entry.source)?;
                    batch.push(QuestMapItemMove {
                        from: at,
                        to: at,
                        entry,
                        expected_revision: pinned.object_revision(key),
                        appearance: after,
                    });
                }
            } else if let Some(m) = batch
                .iter_mut()
                .find(|m| m.to == pos && m.appearance == before)
            {
                m.appearance = after;
            }
        }
        // Qualification here lets the caller reject before moving any occupants.
        for movement in &batch {
            let after = self.movement_item_facts(movement.appearance)?;
            if movement.entry.facts.pickupable
                || after.pickupable
                || self.override_blocks(after).is_none()
            {
                return None;
            }
        }
        Some(batch)
    }

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
    fn qm50_platform_plan_transforms_arrivals_and_round_trips_exact_identities()
    -> Result<(), String> {
        let (bytes, mut pins) = fixture::bundle_with_item_states(
            ("oteryn:item.tibia.i17938", 0),
            ("oteryn:item.tibia.i17939", 1),
            &[
                ("oteryn:item.tibia.i17942", 2),
                ("oteryn:item.tibia.i17943", 3),
            ],
        );
        let mut data = oteryn_world_bundle::bundle::read(&bytes).map_err(|e| e.to_string())?;
        let mut upper_sector = data.sectors[0].clone();
        upper_sector.floor = -6;
        let upper_tile = upper_sector
            .tiles
            .iter_mut()
            .find(|t| t.x == 6)
            .ok_or("upper")?;
        let frame = data.sectors[0]
            .tiles
            .iter()
            .find(|t| t.x == 7)
            .ok_or("frame")?
            .items[1]
            .clone();
        upper_tile.items.extend([frame.clone(), frame]);
        data.sectors[0]
            .tiles
            .iter_mut()
            .find(|t| t.x == 6)
            .ok_or("lower")?
            .items
            .truncate(1);
        data.sectors.push(upper_sector);
        data.manifest.world.floors = vec![-7, -6];
        let bytes =
            oteryn_world_bundle::writer::write(&data.manifest, &data.sectors, &Default::default())
                .map_err(|e| e.to_string())?;
        pins.digest = oteryn_world_bundle::bundle::read(&bytes)
            .map_err(|e| e.to_string())?
            .digest;
        let base = crate::map::load(&bytes, &pins).map_err(|e| e.to_string())?;
        let mut facts = BundleFacts::palette(&data.manifest, |key| {
            let reference = match key {
                "oteryn:item.tibia.i17938" => 1,
                "oteryn:item.tibia.i17939" => 2,
                "oteryn:item.tibia.i17942" => 3,
                "oteryn:item.tibia.i17943" => 4,
                _ => return None,
            };
            Some(ItemDefinition {
                reference: NonZeroU32::new(reference)?,
                solid: Some(false),
                blocks_projectile: false,
                pickupable: false,
                door: false,
            })
        })
        .map_err(|e| format!("{e:?}"))?;
        for sector in &data.sectors {
            facts.push(&data.manifest, sector, &mut BTreeSet::new());
        }
        facts.finish();
        let upper = TilePos {
            x: 6,
            y: 0,
            floor: -6,
        };
        let lower = TilePos { floor: -7, ..upper };
        let down = facts
            .prepare_quest_platform_batch(
                &base,
                upper,
                lower,
                &[17938, 17939],
                &[(upper, 17939, 17942), (lower, 17939, 17943)],
            )
            .ok_or("down plan")?;
        assert_eq!(down.len(), 3);
        assert_eq!(down.iter().filter(|m| m.from != m.to).count(), 2);
        assert!(
            down.iter()
                .any(|m| m.from == upper && m.to == lower && m.appearance == 17943)
        );
        assert!(
            down.iter()
                .any(|m| m.from == upper && m.to == upper && m.appearance == 17942)
        );
        assert!(facts.apply_quest_map_item_batch(&base, &down));
        assert!(
            !facts.apply_quest_map_item_batch(&base, &down),
            "stale plan cannot replay"
        );
        let identities: Vec<_> = facts
            .movement_items(lower)
            .iter()
            .map(|e| e.source)
            .collect();
        let up = facts
            .prepare_quest_platform_batch(
                &base,
                lower,
                upper,
                &[17938, 17943],
                &[(upper, 17942, 17939), (upper, 17943, 17939)],
            )
            .ok_or("up plan")?;
        assert_eq!(up.len(), 3);
        assert!(facts.apply_quest_map_item_batch(&base, &up));
        assert!(facts.movement_items(lower).is_empty());
        let landed = facts.movement_items(upper);
        assert_eq!(landed.len(), 2);
        assert!(
            identities
                .iter()
                .all(|id| landed.iter().any(|e| e.source == *id))
        );
        assert!(landed.iter().any(|e| e.facts.appearance_id == 17939));
        assert!(
            facts
                .prepare_quest_platform_batch(&base, upper, upper, &[17938], &[])
                .is_none()
        );
        assert!(
            facts
                .prepare_quest_platform_batch(&base, lower, upper, &[17938], &[])
                .is_none()
        );
        Ok(())
    }

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

#[cfg(test)]
mod unplaced_tests {
    use super::*;
    use crate::map::boot::tests as fixture;

    #[test]
    fn qm50_unplaced_elevator_frame_retains_qualified_collision_facts() -> Result<(), String> {
        let (bytes, _) = fixture::bundle();
        let bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(|e| e.to_string())?;
        let facts = BundleFacts::palette(&bundle.manifest, |key| {
            if key == "oteryn:item.tibia.i17939" {
                Some(ItemDefinition {
                    reference: NonZeroU32::new(50000)?,
                    solid: Some(false),
                    blocks_projectile: false,
                    pickupable: false,
                    door: false,
                })
            } else {
                fixture::items(key)
            }
        })
        .map_err(|e| format!("{e:?}"))?;
        let frame = facts
            .movement_item_facts(17939)
            .ok_or("unplaced source frame")?;
        assert!(!frame.pickupable);
        assert_eq!(facts.override_blocks(frame), Some(false));
        assert_eq!(
            facts.override_blocks(EntryFacts {
                definition: MapDefinition::Item(NonZeroU32::MIN),
                ..frame
            }),
            None
        );
        Ok(())
    }
}
