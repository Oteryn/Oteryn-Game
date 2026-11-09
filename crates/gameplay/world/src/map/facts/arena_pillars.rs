//! M51 native pillar/portal view. Crystal 201189f
//! scripts/quests/svargrond_arena/creaturescripts_arena_kill.lua:22-35 and
//! lib/quests/svargrond_arena.lua:207-246.
use super::*;

/// Channel-local projection after custody is proved by the fenced arena owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArenaPillarProjection {
    Fighting,
    Victory,
    Release,
}

impl BundleFacts {
    /// Source action25200 is usable only while this exact run owns the opened
    /// immutable pillar occurrence; an appearance alone is not an admission.
    pub fn svargrond_portal_open(&self, pos: TilePos, run: [u8; 16]) -> bool {
        self.callbacks.lock().is_ok_and(|states| {
            states.arena_pillars.iter().any(|(key, owner)| {
                *owner == run
                    && view::placement_of(*key).is_some_and(|(at, _)| at == pos)
                    && !states.hidden.contains(key)
                    && states.passable.get(key) == Some(&true)
                    && states.changed.get(key).is_some_and(|(_, facts)| {
                        facts.appearance_id == 5022 && facts.bound && !facts.pickupable
                    })
            })
        })
    }

    /// Caller holds the current native runtime guard and, except for exact-run
    /// Release, the durable pit lease. A stale release never resets a newer run.
    pub fn set_svargrond_pillar(
        &self,
        base: &crate::map::WorldBase,
        pos: TilePos,
        ordinal: u8,
        id: u32,
        run: [u8; 16],
        projection: ArenaPillarProjection,
    ) -> bool {
        let Some(key) = placement_key(pos, ordinal) else {
            return false;
        };
        // Plain unique-id objects need no sparse placement record. Prove the
        // actual top-level occurrence in the pinned base before palette fallback.
        let Some(tile) = base.tile(pos.x, pos.y, pos.floor) else {
            return false;
        };
        if tile
            .ids()
            .iter()
            .zip(tile.depths())
            .filter(|(_, depth)| **depth == 0)
            .nth(usize::from(ordinal))
            .map(|(&actual, _)| actual)
            != Some(id)
        {
            return false;
        }
        let index = self
            .placements
            .binary_search_by_key(&key, |(key, _)| *key)
            .ok()
            .map(|index| self.placements[index].1.palette)
            .or_else(|| self.unique_index(id));
        let Some(palette) = index.and_then(|index| self.palette.get(index as usize)) else {
            return false;
        };
        if palette.id != id || palette.facts.appearance_id != 1841 || self.house_tile(pos) {
            return false;
        }
        let open = projection == ArenaPillarProjection::Victory;
        let mut after = if open {
            let Some(portal) = self.movement_item_facts(5022) else {
                return false;
            };
            if portal.pickupable {
                return false;
            }
            portal
        } else {
            palette.facts
        };
        after.pickupable = false;
        after.bound = true;
        let Ok(mut states) = self.callbacks.lock() else {
            return false;
        };
        if states.hidden.contains(&key)
            || (projection == ArenaPillarProjection::Release
                && states.arena_pillars.get(&key) != Some(&run))
        {
            return false;
        }
        let (revision, current) = states
            .changed
            .get(&key)
            .copied()
            .unwrap_or((0, palette.facts));
        if !matches!(current.appearance_id, 1841 | 5022) {
            return false;
        }
        let changed = current != after || states.passable.get(&key) != Some(&open);
        let next = if changed {
            let Some(next) = revision.checked_add(1) else {
                return false;
            };
            next
        } else {
            revision
        };
        if !states.has_capacity(3) {
            return false;
        }
        let states = Arc::make_mut(&mut states);
        if changed {
            states.changed.insert(key, (next, after));
        }
        states.passable.insert(key, open);
        if projection == ArenaPillarProjection::Release {
            states.arena_pillars.remove(&key);
        } else {
            states.arena_pillars.insert(key, run);
        }
        true
    }
}

#[cfg(test)]
#[path = "arena_pillars/tests.rs"]
mod tests;
