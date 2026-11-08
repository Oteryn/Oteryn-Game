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
    /// Caller holds the current native runtime guard and, except for exact-run
    /// Release, the durable pit lease. A stale release never resets a newer run.
    pub fn set_svargrond_pillar(
        &self,
        pos: TilePos,
        ordinal: u8,
        id: u32,
        run: [u8; 16],
        projection: ArenaPillarProjection,
    ) -> bool {
        let Some(key) = placement_key(pos, ordinal) else {
            return false;
        };
        let Ok(index) = self.placements.binary_search_by_key(&key, |(key, _)| *key) else {
            return false;
        };
        let Some(palette) = self.palette.get(self.placements[index].1.palette as usize) else {
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
