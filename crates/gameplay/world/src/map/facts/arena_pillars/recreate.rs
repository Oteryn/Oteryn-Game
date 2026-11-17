//! Source reset recreates a missing nonportable pillar. Crystal201189f
//! data-global/lib/quests/svargrond_arena.lua:235-245.
use super::*;
use crate::map::overlay::EntryId;

impl CallbackStates {
    pub(in crate::map::facts::quest_callbacks) fn arena_pillar_closed_at(
        &self,
        pos: TilePos,
    ) -> bool {
        self.arena_recreated.get(&pos).is_some_and(|pillar| {
            !pillar.open
                && self.movement_items.get(&pos).is_some_and(|items| {
                    items.iter().any(|item| {
                        item.source == view::EntrySource::Added(pillar.entry)
                            && item.facts.appearance_id == 1841
                    })
                })
        })
    }
}

impl BundleFacts {
    /// The caller proves this pillar position under the current native runtime
    /// guard and durable pit lease. Only Fighting recreates missing scenery;
    /// a death at a genuinely missing pillar never invents a portal.
    pub fn project_svargrond_pillar(
        &self,
        base: &crate::map::WorldBase,
        pos: TilePos,
        run: [u8; 16],
        projection: ArenaPillarProjection,
    ) -> bool {
        let Some(tile) = base.tile(pos.x, pos.y, pos.floor) else {
            return false;
        };
        if self.house_tile(pos) {
            return false;
        }
        let mut original = tile
            .ids()
            .iter()
            .zip(tile.depths())
            .filter(|(_, depth)| **depth == 0)
            .enumerate()
            .filter_map(|(ordinal, (&id, _))| {
                let ordinal = u8::try_from(ordinal).ok()?;
                (self.quest_callback_base_appearance(pos, ordinal, id) == Some(1841))
                    .then_some((ordinal, id))
            });
        if let Some((ordinal, id)) = original.next() {
            if original.next().is_some() {
                return false;
            }
            return self.set_svargrond_pillar(base, pos, ordinal, id, run, projection);
        }
        self.project_recreated_pillar(pos, run, projection)
    }

    fn project_recreated_pillar(
        &self,
        pos: TilePos,
        run: [u8; 16],
        projection: ArenaPillarProjection,
    ) -> bool {
        let open = projection == ArenaPillarProjection::Victory;
        let Some(mut after) = self.movement_item_facts(if open { 5022 } else { 1841 }) else {
            return false;
        };
        if after.pickupable || after.terrain_kind.is_some() {
            return false;
        }
        after.bound = true;
        after.pickupable = false;
        let Ok(mut held) = self.callbacks.lock() else {
            return false;
        };
        let before = held.arena_recreated.get(&pos).copied();
        if projection == ArenaPillarProjection::Release
            && before.is_none_or(|p| p.owner != Some(run))
        {
            return false;
        }
        let actual = before.and_then(|pillar| {
            held.movement_items
                .get(&pos)?
                .iter()
                .find(|entry| entry.source == view::EntrySource::Added(pillar.entry))
                .copied()
        });
        if actual.is_none() && projection != ArenaPillarProjection::Fighting {
            return false;
        }
        if before.is_none()
            && held.movement_items.get(&pos).is_some_and(|entries| {
                entries
                    .iter()
                    .any(|entry| matches!(entry.facts.appearance_id, 1841 | 5022))
            })
        {
            return false;
        }
        let owner = (projection != ArenaPillarProjection::Release).then_some(run);
        let changed = actual.is_none_or(|entry| entry.facts != after);
        let replacement = if changed {
            let Some(sequence) = held.movement_sequence.checked_add(1) else {
                return false;
            };
            let Some(id) = EntryId::callback(sequence) else {
                return false;
            };
            Some((sequence, id))
        } else {
            None
        };
        if !held.has_capacity(usize::from(actual.is_none()) + usize::from(before.is_none())) {
            return false;
        }
        if before.is_some_and(|p| p.owner == owner && p.open == open) && !changed {
            return true;
        }
        let states = Arc::make_mut(&mut held);
        let entry = if let Some((sequence, id)) = replacement {
            if let Some(before) = before
                && let Some(entries) = states.movement_items.get_mut(&pos)
            {
                entries.retain(|entry| entry.source != view::EntrySource::Added(before.entry));
            }
            states.movement_sequence = sequence;
            states
                .movement_items
                .entry(pos)
                .or_default()
                .push(view::ComposedEntry {
                    source: view::EntrySource::Added(id),
                    facts: after,
                });
            id
        } else {
            let Some(before) = before else {
                return false;
            };
            before.entry
        };
        states
            .arena_recreated
            .insert(pos, RecreatedPillar { owner, entry, open });
        true
    }
}
