//! Channel-local callback projection, sharing the bundle's existing fact/view owner.
use super::*;

#[derive(Clone, Debug)]
struct MovementDecay {
    revision: u64,
    restore: EntryFacts,
}

#[derive(Clone, Debug, Default)]
pub(super) struct CallbackStates {
    pub(super) changed: BTreeMap<u64, (u64, EntryFacts)>,
    pub(super) hidden: BTreeSet<u64>,
    passable: BTreeMap<u64, bool>,
    pub(super) movement_items: BTreeMap<TilePos, Vec<view::ComposedEntry>>,
    movement_sequence: u64,
    pub(super) movement_effects: std::collections::VecDeque<(u64, TilePos, u16)>,
    movement_speech: std::collections::VecDeque<(u64, MovementSpeech)>,
    movement_decays: BTreeMap<(u64, u64), MovementDecay>,
}

impl CallbackStates {
    pub(super) fn movement_item(
        &self,
        id: u64,
    ) -> Option<(TilePos, super::super::overlay::EntryId)> {
        self.movement_items.iter().find_map(|(pos, entries)| {
            entries.iter().find_map(|entry| match entry.source {
                view::EntrySource::Added(entry) if entry.get() == id => Some((*pos, entry)),
                _ => None,
            })
        })
    }
    fn has_capacity(&self, additional: usize) -> bool {
        let used = self.changed.len()
            + self.hidden.len()
            + self.movement_decays.len()
            + self.movement_effects.len()
            + self.movement_speech.len()
            + self.movement_items.values().map(Vec::len).sum::<usize>();
        used.saturating_add(additional) < super::super::overlay::OVERLAY_BUDGET_BYTES / 512
    }
}

impl BundleFacts {
    /// Crystal snow.lua:14-21 and items.xml:18274 onwards: transform and arm one decay.
    pub fn start_movement_decay(
        &self,
        key: u64,
        restore: EntryFacts,
        appearance: u16,
        now: u64,
        delay_us: u64,
    ) -> bool {
        let Some(due) = now.checked_add(delay_us) else {
            return false;
        };
        let Some(mut after) = self.movement_item_facts(appearance) else {
            return false;
        };
        // A ground Item retains its terrain role and original placement attributes.
        after.terrain_kind = restore.terrain_kind;
        after.blocks_projectile = restore.blocks_projectile;
        after.pickupable = restore.pickupable;
        after.bound = restore.bound;
        after.count = restore.count;
        after.sub_type = restore.sub_type;
        let Ok(mut states) = self.callbacks.lock() else {
            return false;
        };
        let Some(revision) = states
            .changed
            .get(&key)
            .map_or(0, |(r, _)| *r)
            .checked_add(1)
        else {
            return false;
        };
        if !states.has_capacity(2) {
            return false;
        }
        let states = Arc::make_mut(&mut states);
        states.changed.insert(key, (revision, after));
        states
            .movement_decays
            .insert((due, key), MovementDecay { revision, restore });
        true
    }

    pub fn next_movement_decay_time(&self) -> Option<u64> {
        self.callbacks
            .lock()
            .ok()?
            .movement_decays
            .first_key_value()
            .map(|((due, _), _)| *due)
    }

    /// The current Channel owner drains this even while no player is walking.
    /// An intervening transform invalidates the old timer's exact object revision.
    pub fn expire_movement_items(&self, now: u64) -> usize {
        let Ok(mut states) = self.callbacks.lock() else {
            return 0;
        };
        let states = Arc::make_mut(&mut states);
        let mut changed = 0;
        while states
            .movement_decays
            .first_key_value()
            .is_some_and(|((due, _), _)| *due <= now)
        {
            let Some(((_, key), decay)) = states.movement_decays.pop_first() else {
                break;
            };
            if states
                .changed
                .get(&key)
                .is_some_and(|(revision, _)| *revision == decay.revision)
                && let Some(revision) = decay.revision.checked_add(1)
            {
                states.changed.insert(key, (revision, decay.restore));
                changed += 1;
            }
        }
        changed
    }
    /// A movement callback may use a served Item even if the bundle never places that state.
    /// Unlike lever callbacks, house furniture movement is explicitly allowed by the source.
    pub fn movement_item_facts(&self, appearance: u16) -> Option<EntryFacts> {
        if let Some(facts) = self.movement_definitions.get(&appearance) {
            return Some(*facts);
        }
        let mut entries = self
            .palette
            .iter()
            .filter(|p| p.facts.appearance_id == appearance);
        let facts = entries.next()?.facts;
        entries.next().is_none().then_some(facts)
    }

    /// The bath's occupied state cannot be picked up while its player stands inside it.
    pub fn transform_movement_item(&self, key: u64, appearance: u16) -> bool {
        let Some(mut after) = self.movement_item_facts(appearance) else {
            return false;
        };
        if appearance == 26100 {
            after.pickupable = false;
        }
        self.apply_quest_callback(key, self.object_revision(key), after, &[], &[], false)
    }
    /// Crystal yellow_pillow.lua:9: native monster-say, independent of the player's chat limiter.
    pub fn emit_movement_speech(&self, speech: MovementSpeech) -> bool {
        let Ok(mut states) = self.callbacks.lock() else {
            return false;
        };
        let Some(sequence) = states.movement_sequence.checked_add(1) else {
            return false;
        };
        if !states.has_capacity(usize::from(states.movement_speech.len() < 512)) {
            return false;
        }
        let states = Arc::make_mut(&mut states);
        states.movement_sequence = sequence;
        if states.movement_speech.len() == 512 {
            states.movement_speech.pop_front();
        }
        states.movement_speech.push_back((sequence, speech));
        true
    }

    pub fn movement_speech_since(&self, after: u64) -> Vec<(u64, MovementSpeech)> {
        self.callbacks
            .lock()
            .map(|s| {
                s.movement_speech
                    .iter()
                    .filter(|(sequence, _)| *sequence > after)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
    /// Native movement magic effects share the map callback's bounded occurrence stream.
    pub fn emit_movement_effect(&self, pos: TilePos, effect: u16) -> bool {
        let Ok(mut states) = self.callbacks.lock() else {
            return false;
        };
        let Some(sequence) = states.movement_sequence.checked_add(1) else {
            return false;
        };
        if !states.has_capacity(usize::from(states.movement_effects.len() < 512)) {
            return false;
        }
        let states = Arc::make_mut(&mut states);
        states.movement_sequence = sequence;
        if states.movement_effects.len() == 512 {
            states.movement_effects.pop_front();
        }
        states.movement_effects.push_back((sequence, pos, effect));
        true
    }
    /// Crystal trap.lua:39-42. Reserve the map projection before consuming the wolf.
    /// The caller holds the exclusive runtime owner through the physical removal.
    pub fn create_movement_item(
        &self,
        pos: TilePos,
        appearance: u16,
        effect: u16,
        consume: impl FnOnce() -> bool,
    ) -> bool {
        let Some(facts) = self.movement_item_facts(appearance) else {
            return false;
        };
        let Ok(mut states) = self.callbacks.lock() else {
            return false;
        };
        let Some(sequence) = states.movement_sequence.checked_add(1) else {
            return false;
        };
        let Some(id) = super::super::overlay::EntryId::callback(sequence) else {
            return false;
        };
        if !states.has_capacity(2) {
            return false;
        }
        let entry = view::ComposedEntry {
            source: view::EntrySource::Added(id),
            facts,
        };
        let states = Arc::make_mut(&mut states);
        if !consume() {
            return false;
        }
        states.movement_sequence = sequence;
        states.movement_items.entry(pos).or_default().push(entry);
        if states.movement_effects.len() == 512 {
            states.movement_effects.pop_front();
        }
        states.movement_effects.push_back((sequence, pos, effect));
        true
    }

    /// Bounded observation; cursors are per observer so nearby sessions see the same effect.
    pub fn movement_effects_since(&self, after: u64) -> Vec<(u64, TilePos, u16)> {
        self.callbacks
            .lock()
            .map(|s| {
                s.movement_effects
                    .iter()
                    .copied()
                    .filter(|(sequence, _, _)| *sequence > after)
                    .collect()
            })
            .unwrap_or_default()
    }
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
                && s.has_capacity(passages)
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
            || !states.has_capacity(passages.len() + replacements.len())
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
