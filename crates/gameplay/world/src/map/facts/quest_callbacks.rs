//! Channel-local callback projection, sharing the bundle's existing fact/view owner.
use super::*;

#[derive(Clone, Debug, Default)]
pub(super) struct CallbackStates {
    pub(super) changed: BTreeMap<u64, (u64, EntryFacts)>,
    pub(super) hidden: BTreeSet<u64>,
    passable: BTreeMap<u64, bool>,
}

impl BundleFacts {
    /// Source selector validation against the original bound placement, never a client id.
    pub fn quest_callback_matches(
        &self,
        key: u64,
        unique: Option<u16>,
        action: Option<u16>,
        appearance: Option<u16>,
    ) -> bool {
        let Some((pos, _)) = view::placement_of(key) else {
            return false;
        };
        if self.house_tile(pos) {
            return false;
        }
        let found = self
            .placements
            .binary_search_by_key(&key, |(key, _)| *key)
            .ok();
        let placement = found.map(|i| &self.placements[i].1);
        unique.is_none_or(|id| placement.is_some_and(|p| p.attrs.unique == Some(id)))
            && action.is_none_or(|id| placement.is_some_and(|p| p.attrs.action == Some(id)))
            && appearance.is_none_or(|id| {
                placement
                    .is_some_and(|p| self.palette[p.palette as usize].facts.appearance_id == id)
            })
    }

    /// Reserve no durable inputs unless the bounded local projection can be installed.
    pub fn quest_callback_capacity(&self, key: u64, expected: u64, passages: usize) -> bool {
        self.callbacks.lock().is_ok_and(|s| {
            s.changed.get(&key).map_or(0, |(r, _)| *r) == expected
                && expected < u64::MAX
                && passages <= 8
                && s.changed.len() + s.hidden.len() + passages
                    < super::super::overlay::OVERLAY_BUDGET_BYTES / 512
        })
    }

    /// A bridge replacement must already have unambiguous facts in the bundle palette.
    pub fn quest_callback_replacement(&self, key: u64, appearance: u16) -> Option<EntryFacts> {
        let (pos, _) = view::placement_of(key)?;
        if self.house_tile(pos) {
            return None;
        }
        let mut entries = self
            .palette
            .iter()
            .filter(|p| p.facts.appearance_id == appearance);
        let replacement = entries.next()?;
        if entries.next().is_some() {
            return None;
        }
        Some(replacement.facts)
    }

    /// Apply a prevalidated lever and passage batch under the Channel owner lock.
    pub fn apply_quest_callback(
        &self,
        key: u64,
        expected: u64,
        after: EntryFacts,
        passages: &[u64],
        replacements: &[(u64, EntryFacts, bool)],
        close: bool,
    ) -> bool {
        let Ok(mut states) = self.callbacks.lock() else {
            return false;
        };
        if states.changed.get(&key).map_or(0, |(r, _)| *r) != expected
            || expected == u64::MAX
            || passages.len() + replacements.len() > 8
            || (states.changed.len() + states.hidden.len() + passages.len() + replacements.len())
                >= super::super::overlay::OVERLAY_BUDGET_BYTES / 512
        {
            return false;
        }
        if replacements
            .iter()
            .any(|(k, _, _)| states.changed.get(k).is_some_and(|(r, _)| *r == u64::MAX))
        {
            return false;
        }
        let states = Arc::make_mut(&mut states);
        for passage in passages {
            if close {
                states.hidden.remove(passage);
            } else {
                states.hidden.insert(*passage);
            }
        }
        for &(entry, facts, passable) in replacements {
            let revision = states
                .changed
                .get(&entry)
                .map_or(0, |(r, _)| *r)
                .checked_add(1);
            let Some(revision) = revision else {
                return false;
            };
            states.changed.insert(entry, (revision, facts));
            states.passable.insert(entry, passable);
        }
        states.changed.insert(key, (expected + 1, after));
        true
    }

    /// Removing a wall must alter the collision index as well as its visible stack.
    pub fn quest_callback_blocked(
        &self,
        base: &super::super::WorldBase,
        pos: TilePos,
        baseline: bool,
    ) -> bool {
        let Ok(states) = self.callbacks.lock() else {
            return true;
        };
        let Some(tile) = base.tile(pos.x, pos.y, pos.floor) else {
            return true;
        };
        let Some(first) = placement_key(pos, 0) else {
            return true;
        };
        if states.hidden.range(first..=first + 63).next().is_none()
            && states.passable.range(first..=first + 63).next().is_none()
        {
            return !tile.walkable() || baseline;
        }
        let mut ordinal = 0_u8;
        let mut changed = false;
        let mut blocked = false;
        for (&id, &depth) in tile.ids().iter().zip(tile.depths()) {
            if depth != 0 {
                continue;
            }
            let Some(key) = placement_key(pos, ordinal) else {
                return true;
            };
            ordinal = ordinal.saturating_add(1);
            if let Some(passable) = states.passable.get(&key) {
                changed = true;
                blocked |= !passable;
                continue;
            }
            if states.hidden.contains(&key) {
                changed = true;
                continue;
            }
            if self.doors.contains(key) {
                continue;
            }
            let palette = self
                .placements
                .binary_search_by_key(&key, |(k, _)| *k)
                .ok()
                .map(|i| self.placements[i].1.palette)
                .or_else(|| self.unique_index(id));
            blocked |= palette.is_none_or(|p| self.blocks(p));
        }
        let ground_walkable = tile
            .ground()
            .and_then(|(o, _)| placement_key(pos, o))
            .and_then(|k| states.passable.get(&k).copied())
            .unwrap_or_else(|| tile.walkable());
        !ground_walkable || if changed { blocked } else { baseline }
    }
}
