//! Channel-local callback projection, sharing the bundle's existing fact/view owner.
use super::*;

#[path = "quest_item_batch.rs"]
mod item_batch;
pub use item_batch::QuestMapItemMove;

#[derive(Clone, Debug)]
struct MovementDecay {
    revision: u64,
    restore: EntryFacts,
}

#[derive(Clone, Debug)]
struct DynamicPortal {
    owner: u64,
    destination: TilePos,
    entry: EntryId,
    expires: Option<u64>,
}

/// Already qualified facts for one atomic, Channel-local callback projection.
pub enum QuestCallbackProjection {
    /// Restore exactly this revision at an original source deadline.
    TimedTransform { due: u64, restore: EntryFacts },
    /// Each read revision, including cells which do not change, fences the batch.
    ReadsetTransforms {
        entries: Vec<(u64, u64, Option<EntryFacts>)>,
        reset: Option<(u64, EntryFacts)>,
    },
    Portals {
        endpoints: [(TilePos, TilePos, EntryFacts); 2],
        expires: Option<u64>,
    },
    ItemPresence {
        position: TilePos,
        entry: EntryFacts,
    },
    ItemCycle {
        position: TilePos,
        remove: Option<u64>,
        create: Option<EntryFacts>,
    },
}

type EffectScope = (
    crate::foundation::RuntimeScopeRefV1,
    crate::foundation::ScopeOwnershipGeneration,
);
type ScopedMovementEffect = (u64, Option<EffectScope>, TilePos, u16);

#[derive(Clone, Debug, Default)]
pub(super) struct CallbackStates {
    pub(super) changed: BTreeMap<u64, (u64, EntryFacts)>,
    pub(super) actions: BTreeMap<u64, Option<u16>>,
    pub(super) hidden: BTreeSet<u64>,
    passable: BTreeMap<u64, bool>,
    pub(super) movement_items: BTreeMap<TilePos, Vec<view::ComposedEntry>>,
    movement_sequence: u64,
    pub(super) movement_effects: std::collections::VecDeque<ScopedMovementEffect>,
    pub(super) creature_visuals: std::collections::VecDeque<(u64, CreatureVisualKind)>,
    movement_speech: std::collections::VecDeque<(u64, MovementSpeech)>,
    movement_decays: BTreeMap<(u64, u64), MovementDecay>,
    portals: BTreeMap<TilePos, DynamicPortal>,
    solid_items: BTreeSet<TilePos>,
    batch_blockers: BTreeSet<TilePos>,
    presences: BTreeMap<u64, (TilePos, EntryId)>,
}

impl CallbackStates {
    fn remove_portals(&mut self, owner: u64) {
        self.portals.retain(|pos, portal| {
            if portal.owner != owner {
                return true;
            }
            if let Some(entries) = self.movement_items.get_mut(pos) {
                entries.retain(|entry| entry.source != view::EntrySource::Added(portal.entry));
                if entries.is_empty() {
                    self.movement_items.remove(pos);
                }
            }
            false
        });
    }

    fn project(&mut self, owner: u64, projection: &QuestCallbackProjection, close: bool) -> bool {
        match projection {
            QuestCallbackProjection::ReadsetTransforms { entries, reset } => {
                let keys: BTreeSet<_> = entries.iter().map(|(key, _, _)| *key).collect();
                if entries.is_empty()
                    || entries.len() > 8
                    || keys.len() != entries.len()
                    || keys.contains(&owner)
                    || entries.iter().any(|(key, expected, _)| {
                        *expected == u64::MAX
                            || self.hidden.contains(key)
                            || self.changed.get(key).map_or(0, |(revision, _)| *revision)
                                != *expected
                    })
                {
                    return false;
                }
                if let Some((due, restore)) = reset {
                    let Some(revision) = self
                        .changed
                        .get(&owner)
                        .map_or(0, |(r, _)| *r)
                        .checked_add(1)
                    else {
                        return false;
                    };
                    self.movement_decays.insert(
                        (*due, owner),
                        MovementDecay {
                            revision,
                            restore: *restore,
                        },
                    );
                }
                for (key, revision, after) in entries {
                    if let Some(after) = after {
                        self.changed.insert(*key, (revision + 1, *after));
                    }
                }
            }
            QuestCallbackProjection::TimedTransform { due, restore } => {
                let Some(revision) = self
                    .changed
                    .get(&owner)
                    .map_or(0, |(r, _)| *r)
                    .checked_add(1)
                else {
                    return false;
                };
                self.movement_decays.insert(
                    (*due, owner),
                    MovementDecay {
                        revision,
                        restore: *restore,
                    },
                );
            }
            QuestCallbackProjection::Portals { endpoints, expires } => {
                if close {
                    if endpoints
                        .iter()
                        .any(|(pos, _, _)| self.portals.get(pos).is_some_and(|p| p.owner != owner))
                    {
                        return false;
                    }
                    self.remove_portals(owner);
                    return true;
                }
                if endpoints
                    .iter()
                    .any(|(pos, _, _)| self.portals.contains_key(pos))
                {
                    return false;
                }
                for &(pos, destination, facts) in endpoints {
                    let Some(sequence) = self.movement_sequence.checked_add(1) else {
                        return false;
                    };
                    let Some(entry) = EntryId::callback(sequence) else {
                        return false;
                    };
                    self.movement_sequence = sequence;
                    self.movement_items
                        .entry(pos)
                        .or_default()
                        .push(view::ComposedEntry {
                            source: view::EntrySource::Added(entry),
                            facts,
                        });
                    self.portals.insert(
                        pos,
                        DynamicPortal {
                            owner,
                            destination,
                            entry,
                            expires: *expires,
                        },
                    );
                }
            }
            QuestCallbackProjection::ItemPresence {
                position,
                entry: facts,
            } => {
                if let Some((pos, id)) = self.presences.get(&owner).copied() {
                    if pos != *position {
                        return false;
                    }
                    if !close {
                        return true;
                    }
                    if let Some(entries) = self.movement_items.get_mut(&pos) {
                        entries.retain(|entry| entry.source != view::EntrySource::Added(id));
                        if entries.is_empty() {
                            self.movement_items.remove(&pos);
                        }
                    }
                    self.presences.remove(&owner);
                } else if !close {
                    if facts.pickupable || self.presences.values().any(|(pos, _)| pos == position) {
                        return false;
                    }
                    let Some(sequence) = self.movement_sequence.checked_add(1) else {
                        return false;
                    };
                    let Some(entry) = EntryId::callback(sequence) else {
                        return false;
                    };
                    self.movement_sequence = sequence;
                    self.movement_items
                        .entry(*position)
                        .or_default()
                        .push(view::ComposedEntry {
                            source: view::EntrySource::Added(entry),
                            facts: *facts,
                        });
                    self.presences.insert(owner, (*position, entry));
                }
            }
            QuestCallbackProjection::ItemCycle {
                position,
                remove,
                create,
            } => {
                if let Some(key) = remove {
                    self.hidden.insert(*key);
                }
                if let Some(facts) = create {
                    let Some(sequence) = self.movement_sequence.checked_add(1) else {
                        return false;
                    };
                    let Some(entry) = EntryId::callback(sequence) else {
                        return false;
                    };
                    self.movement_sequence = sequence;
                    self.movement_items
                        .entry(*position)
                        .or_default()
                        .push(view::ComposedEntry {
                            source: view::EntrySource::Added(entry),
                            facts: *facts,
                        });
                    self.solid_items.insert(*position);
                }
            }
        }
        true
    }
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
            + self.actions.len()
            + self.hidden.len()
            + self.movement_decays.len()
            + self.movement_effects.len()
            + self.creature_visuals.len()
            + self.movement_speech.len()
            + self.portals.len()
            + self.solid_items.len()
            + self.batch_blockers.len()
            + self.movement_items.values().map(Vec::len).sum::<usize>();
        used.saturating_add(additional) < super::super::overlay::OVERLAY_BUDGET_BYTES / 512
    }
}

impl BundleFacts {
    /// A transformed/removed callback entry changes floor routing with its visible stack.
    /// Unchanged tiles keep the compact boot-time table and incur no stack scan.
    pub(crate) fn quest_callback_floor_mask(
        &self,
        base: &super::super::WorldBase,
        pos: TilePos,
    ) -> Option<u8> {
        let states = self.callbacks.lock().ok().map(|s| Arc::clone(&s))?;
        let first = placement_key(pos, 0)?;
        if states.changed.range(first..=first + 63).next().is_none()
            && states.hidden.range(first..=first + 63).next().is_none()
        {
            return None;
        }
        let facts = PinnedFacts {
            facts: self,
            callbacks: Some(states),
            doors: self.doors.snapshot(),
            rotations: self.rotations.lock().ok().map(|s| Arc::clone(&s)),
        };
        let tile = base.tile(pos.x, pos.y, pos.floor)?;
        let mut mask = 0;
        for (ordinal, (&id, _)) in tile
            .ids()
            .iter()
            .zip(tile.depths())
            .filter(|(_, d)| **d == 0)
            .enumerate()
        {
            let ordinal = u8::try_from(ordinal).ok()?;
            if facts.base_hidden(placement_key(pos, ordinal)?) {
                continue;
            }
            let entry = facts.base_entry(pos, ordinal, id)?;
            let family = match entry.definition {
                MapDefinition::Item(_) => Family::Item,
                MapDefinition::Terrain(_) => Family::Terrain,
            };
            mask |= palette_mask(
                family,
                &format!("oteryn:terrain.tibia.i{}", entry.appearance_id),
            );
        }
        Some(mask)
    }

    pub fn emit_creature_visual(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
        cue: CreatureVisualCue,
    ) -> bool {
        if !cue.current(runtime) {
            return false;
        }
        let Ok(mut states) = self.callbacks.lock() else {
            return false;
        };
        let Some(sequence) = states.movement_sequence.checked_add(1) else {
            return false;
        };
        if !states.has_capacity(usize::from(states.creature_visuals.len() < 512)) {
            return false;
        }
        let states = Arc::make_mut(&mut states);
        states.movement_sequence = sequence;
        if states.creature_visuals.len() == 512 {
            states.creature_visuals.pop_front();
        }
        states
            .creature_visuals
            .push_back((sequence, cue.into_kind()));
        true
    }
    pub fn creature_visuals_since(&self, after: u64) -> Vec<(u64, CreatureVisualKind)> {
        self.callbacks
            .lock()
            .map(|states| {
                states
                    .creature_visuals
                    .iter()
                    .filter(|(sequence, _)| *sequence > after)
                    .copied()
                    .collect()
            })
            .unwrap_or_default()
    }
    /// Records a committed monster step for the dispatch-owned trap pass.
    pub fn record_monster_step(&self, step: MonsterStep) -> bool {
        let Ok(mut steps) = self.monster_steps.lock() else {
            return false;
        };
        if steps.len() == 4096 {
            steps.pop_front();
        }
        steps.push_back(step);
        true
    }

    /// Takes committed monster steps in commit order.
    pub fn take_monster_steps(&self) -> Vec<MonsterStep> {
        self.monster_steps
            .lock()
            .map(|mut steps| steps.drain(..).collect())
            .unwrap_or_default()
    }

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
        let states = self.callbacks.lock().ok()?;
        states
            .movement_decays
            .first_key_value()
            .map(|((due, _), _)| *due)
            .into_iter()
            .chain(states.portals.values().filter_map(|p| p.expires))
            .min()
    }

    /// The current Channel owner drains this even while no player is walking.
    /// An intervening transform invalidates the old timer's exact object revision.
    pub fn expire_movement_items(&self, now: u64) -> usize {
        let Ok(mut states) = self.callbacks.lock() else {
            return 0;
        };
        let states = Arc::make_mut(&mut states);
        let mut changed = 0;
        let expired: BTreeSet<_> = states
            .portals
            .values()
            .filter(|p| p.expires.is_some_and(|due| due <= now))
            .map(|p| p.owner)
            .collect();
        for owner in expired {
            states.remove_portals(owner);
            changed += 1;
        }
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

    /// Transform the actual current source object and its action under the same
    /// callback owner commit. Other bound attributes, including destination,
    /// unique id and description, remain on the same physical placement.
    pub fn transform_movement_item_with_action(
        &self,
        key: u64,
        expected_revision: u64,
        expected_appearance: u16,
        appearance: u16,
        action: Option<u16>,
    ) -> bool {
        if self.object_revision(key) != expected_revision {
            return false;
        }
        let Some(mut after) = self.movement_item_facts(appearance) else {
            return false;
        };
        if appearance == 26100 {
            after.pickupable = false;
        }
        let Some(original) = self
            .placements
            .binary_search_by_key(&key, |(key, _)| *key)
            .ok()
            .and_then(|index| self.palette.get(self.placements[index].1.palette as usize))
        else {
            return false;
        };
        let passable = self.palette.iter().find_map(|entry| {
            (entry.static_surface
                && entry.facts.definition == after.definition
                && entry.facts.appearance_id == appearance)
                .then_some(!entry.solid)
        });
        let Ok(mut state) = self.callbacks.lock() else {
            return false;
        };
        let current = state.changed.get(&key);
        if current.map_or(0, |(revision, _)| *revision) != expected_revision
            || current.map_or(original.facts.appearance_id, |(_, facts)| {
                facts.appearance_id
            }) != expected_appearance
            || state.hidden.contains(&key)
            || expected_revision == u64::MAX
            || !state.has_capacity(
                usize::from(!state.changed.contains_key(&key))
                    + usize::from(!state.actions.contains_key(&key))
                    + usize::from(passable.is_some() && !state.passable.contains_key(&key)),
            )
        {
            return false;
        }
        let state = Arc::make_mut(&mut state);
        state.changed.insert(key, (expected_revision + 1, after));
        state.actions.insert(key, action);
        if let Some(passable) = passable {
            state.passable.insert(key, passable);
        }
        true
    }

    /// The bath's occupied state cannot be picked up while its player stands inside it.
    pub fn transform_movement_item(&self, key: u64, appearance: u16) -> bool {
        let Some(mut after) = self.movement_item_facts(appearance) else {
            return false;
        };
        if appearance == 26100 {
            after.pickupable = false;
        }
        // These source-published states have no inventory Item identity. Their
        // native collision must change in the same existing map callback commit
        // as their visible state (Bakragore's four flame pairs).
        if let Some(passable) = self.palette.iter().find_map(|entry| {
            (entry.static_surface
                && entry.facts.definition == after.definition
                && entry.facts.appearance_id == appearance)
                .then_some(!entry.solid)
        }) {
            return self.apply_quest_callback(
                key,
                self.object_revision(key),
                after,
                &[],
                &[(key, after, passable)],
                false,
                None,
            );
        }
        self.apply_quest_callback(key, self.object_revision(key), after, &[], &[], false, None)
    }
    /// Crystal yellow_pillow.lua:9: native monster-say, independent of the player's chat limiter.
    pub fn emit_movement_speech(&self, speech: MovementSpeech) -> bool {
        if speech.text.is_empty()
            || speech.text.len() > oteryn_protocol_oteryn::chat::MAX_CHAT_TEXT_BYTES
            || !matches!(speech.talk_type, 36 | 37)
        {
            return false;
        }
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
        self.emit_effect_in_scope(None, pos, effect)
    }

    /// A private owner uses the same bounded map outbox and sequence. This is
    /// presentation data only; the caller must prove current scope custody.
    pub fn emit_scoped_movement_effect(
        &self,
        scope: crate::foundation::RuntimeScopeRefV1,
        generation: crate::foundation::ScopeOwnershipGeneration,
        pos: TilePos,
        effect: u16,
    ) -> bool {
        if !matches!(scope, crate::foundation::RuntimeScopeRefV1::Instance { .. }) {
            return false;
        }
        self.emit_effect_in_scope(Some((scope, generation)), pos, effect)
    }

    fn emit_effect_in_scope(&self, scope: Option<EffectScope>, pos: TilePos, effect: u16) -> bool {
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
        states
            .movement_effects
            .push_back((sequence, scope, pos, effect));
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
        states
            .movement_effects
            .push_back((sequence, None, pos, effect));
        true
    }

    /// Crystal data-global/scripts/systems/hazard_primal.lua:53,68 removes
    /// only the pod that still exists.
    /// The physical owner supplies this exact volatile occurrence, position and
    /// appearance; base/DUR-03/world-change identities cannot be removed here.
    pub fn remove_movement_item(&self, pos: TilePos, key: u64, appearance: u16) -> bool {
        if key >> 62 != 2 {
            return false;
        }
        let Ok(mut states) = self.callbacks.lock() else {
            return false;
        };
        let Some(index) = states.movement_items.get(&pos).and_then(|entries| {
            entries.iter().position(|entry| {
                matches!(entry.source,
                view::EntrySource::Added(id) if id.get()==key)
                    && entry.facts.appearance_id == appearance
            })
        }) else {
            return false;
        };
        let Some(sequence) = states.movement_sequence.checked_add(1) else {
            return false;
        };
        let states = Arc::make_mut(&mut states);
        let Some(entries) = states.movement_items.get_mut(&pos) else {
            return false;
        };
        entries.remove(index);
        if entries.is_empty() {
            states.movement_items.remove(&pos);
        }
        states.changed.remove(&key);
        states.movement_decays.retain(|(_, entry), _| *entry != key);
        states.movement_sequence = sequence;
        true
    }

    /// W05b (Crystal thawing.lua:21-27): a daily world change's volatile Item, placed at
    /// Channel boot; no magic effect. Lives until the Channel ends, like other callback items.
    /// Author a server-owned shared callback object through the existing Added
    /// source identity. A portable catalogue appearance is never pickup custody.
    pub fn create_world_change_callback_item(&self, pos: TilePos, appearance: u16) -> bool {
        if !self.create_world_change_item(pos, appearance) {
            return false;
        }
        let Some(id) = super::super::overlay::EntryId::world_change(pos, appearance) else {
            return false;
        };
        let Ok(mut states) = self.callbacks.lock() else {
            return false;
        };
        let states = Arc::make_mut(&mut states);
        let Some(entry) = states.movement_items.get_mut(&pos).and_then(|entries| {
            entries
                .iter_mut()
                .find(|entry| entry.source == view::EntrySource::Added(id))
        }) else {
            return false;
        };
        entry.facts.pickupable = false;
        entry.facts.bound = true;
        true
    }

    /// Remove a committed shared world source while retaining its revision fence.
    pub fn remove_world_change_callback_item(&self, key: u64, revision: u64) -> bool {
        let Some((_, x, y, z)) =
            oteryn_gameplay_items::event_planting::world_change_item_source(key)
        else {
            return false;
        };
        let pos = TilePos {
            x,
            y,
            floor: -(z as i8),
        };
        let Ok(mut states) = self.callbacks.lock() else {
            return false;
        };
        if states.changed.get(&key).map_or(0, |(r, _)| *r) != revision {
            return false;
        }
        let Some(entries) = Arc::make_mut(&mut states).movement_items.get_mut(&pos) else {
            return false;
        };
        let before = entries.len();
        entries
            .retain(|entry| !matches!(entry.source,view::EntrySource::Added(id) if id.get()==key));
        before == entries.len() + 1
    }

    pub fn create_world_change_item(&self, pos: TilePos, appearance: u16) -> bool {
        let Some(facts) = self.movement_item_facts(appearance) else {
            return false;
        };
        let Some(id) = super::super::overlay::EntryId::world_change(pos, appearance) else {
            return false;
        };
        if self.house_tile(pos) {
            return false;
        }
        let Ok(mut states) = self.callbacks.lock() else {
            return false;
        };
        // A repeated projection cannot undo a harvest or create a second source.
        if states.movement_items.get(&pos).is_some_and(|entries| {
            entries
                .iter()
                .any(|entry| entry.source == view::EntrySource::Added(id))
        }) {
            return true;
        }
        if !states.has_capacity(1) {
            return false;
        }
        Arc::make_mut(&mut states)
            .movement_items
            .entry(pos)
            .or_default()
            .push(view::ComposedEntry {
                source: view::EntrySource::Added(id),
                facts,
            });
        true
    }

    /// Install the existing durable harvest CAS on its stable Added object, including restart
    /// and regrowth. None delegates ordinary base/volatile/player-grown objects to their owner.
    pub fn install_world_change_harvest_state(
        &self,
        key: u64,
        revision: u64,
        appearance: u16,
    ) -> Option<bool> {
        let (source, x, y, z) =
            oteryn_gameplay_items::event_planting::world_change_item_source(key)?;
        let pos = TilePos {
            x,
            y,
            floor: -(z as i8),
        };
        if source != 13842
            || !matches!(appearance, 13842 | 13843)
            || revision == 0
            || self.house_tile(pos)
        {
            return Some(false);
        }
        let Some(after) = self.movement_item_facts(appearance) else {
            return Some(false);
        };
        let Ok(mut states) = self.callbacks.lock() else {
            return Some(false);
        };
        let Some((actual, _)) = states.movement_item(key) else {
            return Some(false);
        };
        if actual != pos {
            return Some(false);
        }
        if let Some((current, facts)) = states.changed.get(&key) {
            if *current > revision {
                return Some(true);
            }
            if *current == revision {
                return Some(*facts == after);
            }
        }
        if !states.has_capacity(1) {
            return Some(false);
        }
        let states = Arc::make_mut(&mut states);
        let Some(entry) = states.movement_items.get_mut(&pos).and_then(|entries| {
            entries.iter_mut().find(
                |entry| matches!(entry.source, view::EntrySource::Added(id) if id.get() == key),
            )
        }) else {
            return Some(false);
        };
        entry.facts = after;
        states.changed.insert(key, (revision, after));
        Some(true)
    }

    /// Bounded observation; cursors are per observer so nearby sessions see the same effect.
    pub fn movement_effects_since(&self, after: u64) -> Vec<(u64, TilePos, u16)> {
        self.callbacks
            .lock()
            .map(|s| {
                s.movement_effects
                    .iter()
                    .copied()
                    .filter(|(sequence, scope, _, _)| *sequence > after && scope.is_none())
                    .map(|(sequence, _, pos, effect)| (sequence, pos, effect))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Read only this exact private scope and owner generation. Legacy Channel
    /// effects and another House's entries are never a fallback.
    pub fn scoped_movement_effects_since(
        &self,
        scope: crate::foundation::RuntimeScopeRefV1,
        generation: crate::foundation::ScopeOwnershipGeneration,
        after: u64,
    ) -> Vec<(u64, TilePos, u16)> {
        self.callbacks
            .lock()
            .map(|states| {
                states
                    .movement_effects
                    .iter()
                    .copied()
                    .filter(|(sequence, owner, _, _)| {
                        *sequence > after && *owner == Some((scope, generation))
                    })
                    .map(|(sequence, _, pos, effect)| (sequence, pos, effect))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Only live paired portals resolve; the timer and explicit removal retire both ends.
    pub fn quest_portal_destination(&self, pos: TilePos, now: u64) -> Option<TilePos> {
        let states = self.callbacks.lock().ok()?;
        let portal = states.portals.get(&pos)?;
        portal
            .expires
            .is_none_or(|due| due > now)
            .then_some(portal.destination)
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

    /// Match an exact authored counterpart only while the retained base object is bare.
    /// Sparse placements and plain palette entries share the same original appearance proof.
    pub fn quest_callback_native_object(&self, key: u64, id: u32, appearance: u16) -> bool {
        let Some((pos, _)) = view::placement_of(key) else {
            return false;
        };
        if self.house_tile(pos) || self.base_hidden(key) {
            return false;
        }
        let index = match self.placements.binary_search_by_key(&key, |(k, _)| *k) {
            Ok(at) => {
                let placement = &self.placements[at].1;
                if placement.attrs != Attrs::default() || placement.dropped {
                    return false;
                }
                placement.palette
            }
            Err(_) => {
                let Some(index) = self.unique_index(id) else {
                    return false;
                };
                index
            }
        };
        self.palette
            .get(index as usize)
            .is_some_and(|p| p.facts.appearance_id == appearance)
    }

    /// Match a source-selected native portal against its original bound attributes.
    /// A legacy unique/action binding or a hidden placement cannot become this alias.
    pub fn quest_callback_native_teleport(
        &self,
        key: u64,
        appearance: u16,
        destination: TilePos,
    ) -> bool {
        let Some((pos, _)) = view::placement_of(key) else {
            return false;
        };
        if self.house_tile(pos) || self.base_hidden(key) {
            return false;
        }
        self.placements
            .binary_search_by_key(&key, |(key, _)| *key)
            .ok()
            .is_some_and(|i| {
                let p = &self.placements[i].1;
                p.attrs.unique.is_none()
                    && p.attrs.action.is_none()
                    && self.palette[p.palette as usize].facts.appearance_id == appearance
                    && p.attrs.teleport
                        == Some((destination.x, destination.y, destination.floor as u8))
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

    /// The authored base appearance is stable across Channel callback transformations.
    /// Sparse attributes select the palette exactly; plain entries require an unambiguous id.
    pub fn quest_callback_base_appearance(
        &self,
        pos: TilePos,
        ordinal: u8,
        id: u32,
    ) -> Option<u16> {
        let key = view::placement_key(pos, ordinal)?;
        let index = match self.placements.binary_search_by_key(&key, |(key, _)| *key) {
            Ok(found) => self.placements[found].1.palette,
            Err(_) => self.unique_index(id)?,
        };
        let palette = self.palette.get(index as usize)?;
        (palette.id == id).then_some(palette.facts.appearance_id)
    }

    /// A bridge replacement must already have unambiguous facts in the bundle palette.
    pub fn quest_callback_replacement(&self, key: u64, appearance: u16) -> Option<EntryFacts> {
        if let Some((_, x, y, z)) =
            oteryn_gameplay_items::event_planting::world_change_item_source(key)
        {
            let pos = TilePos {
                x,
                y,
                floor: -(z as i8),
            };
            if self.house_tile(pos) || self.callbacks.lock().ok()?.movement_item(key)?.0 != pos {
                return None;
            }
            return self.movement_item_facts(appearance);
        }
        let (pos, _) = view::placement_of(key)?;
        if self.house_tile(pos) {
            return None;
        }
        let mut entries = self
            .palette
            .iter()
            .filter(|p| p.facts.appearance_id == appearance);
        let Some(replacement) = entries.next() else {
            // Source transforms may name a served state which is never placed in the base map.
            return self.movement_definitions.get(&appearance).copied();
        };
        if entries.next().is_some() {
            return None;
        }
        Some(replacement.facts)
    }

    /// Apply a prevalidated lever and passage batch under the Channel owner lock.
    #[allow(
        clippy::too_many_arguments,
        reason = "one atomic projection binds the lever revision and its bounded map effects"
    )]
    pub fn apply_quest_callback(
        &self,
        key: u64,
        expected: u64,
        after: EntryFacts,
        passages: &[u64],
        replacements: &[(u64, EntryFacts, bool)],
        close: bool,
        projection: Option<&QuestCallbackProjection>,
    ) -> bool {
        let Ok(mut states) = self.callbacks.lock() else {
            return false;
        };
        if states.changed.get(&key).map_or(0, |(r, _)| *r) != expected
            || expected == u64::MAX
            || passages.len() + replacements.len() > 8
            || !states.has_capacity(
                passages.len() + replacements.len() + usize::from(projection.is_some()) * 8,
            )
        {
            return false;
        }
        if replacements
            .iter()
            .any(|(k, _, _)| states.changed.get(k).is_some_and(|(r, _)| *r == u64::MAX))
        {
            return false;
        }
        // Stage new volatile objects before publishing either endpoint, lever or wall.
        // A failed allocation/id/ownership check leaves the pinned projection unchanged.
        let mut staged = projection.map(|_| (**states).clone());
        if let (Some(projection), Some(staged)) = (projection, staged.as_mut())
            && !staged.project(key, projection, close)
        {
            return false;
        }
        if let Some(staged) = staged {
            *states = Arc::new(staged);
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

    /// Crystal Triangle Tower removes1270 but creates1025. The first created wall
    /// reuses the removed base slot; subsequent source creates retain distinct entries.
    pub fn apply_quest_wall_cycle(
        &self,
        key: u64,
        expected: u64,
        lever: EntryFacts,
        wall: u64,
        created: EntryFacts,
        remove: bool,
    ) -> bool {
        let Some((pos, _)) = view::placement_of(wall) else {
            return false;
        };
        if self.house_tile(pos) {
            return false;
        }
        let Ok(mut states) = self.callbacks.lock() else {
            return false;
        };
        let revision = states.changed.get(&wall).map_or(0, |(r, _)| *r);
        if states.changed.get(&key).map_or(0, |(r, _)| *r) != expected
            || expected == u64::MAX
            || revision == u64::MAX
            || !states.has_capacity(3)
        {
            return false;
        }
        let added = if !remove && !states.hidden.contains(&wall) {
            let Some(sequence) = states.movement_sequence.checked_add(1) else {
                return false;
            };
            let Some(id) = super::super::overlay::EntryId::callback(sequence) else {
                return false;
            };
            Some((
                sequence,
                view::ComposedEntry {
                    source: view::EntrySource::Added(id),
                    facts: created,
                },
            ))
        } else {
            None
        };
        let states = Arc::make_mut(&mut states);
        if remove {
            states.hidden.insert(wall);
        } else {
            if let Some((sequence, entry)) = added {
                states.movement_sequence = sequence;
                states.movement_items.entry(pos).or_default().push(entry);
            } else {
                states.hidden.remove(&wall);
                states.changed.insert(wall, (revision + 1, created));
            }
            states.passable.insert(wall, false);
        }
        states.changed.insert(key, (expected + 1, lever));
        true
    }

    /// Removing a wall must alter the collision index as well as its visible stack.
    pub fn quest_callback_blocked(
        &self,
        base: &super::super::WorldBase,
        pos: TilePos,
        baseline: bool,
    ) -> bool {
        let Ok(rotations) = self.rotations.lock().map(|states| Arc::clone(&states)) else {
            return true;
        };
        let Ok(states) = self.callbacks.lock() else {
            return true;
        };
        if states.solid_items.contains(&pos) || states.batch_blockers.contains(&pos) {
            return true;
        }
        let Some(tile) = base.tile(pos.x, pos.y, pos.floor) else {
            return true;
        };
        let Some(first) = placement_key(pos, 0) else {
            return true;
        };
        if states.hidden.range(first..=first + 63).next().is_none()
            && states.passable.range(first..=first + 63).next().is_none()
            && rotations.range(first..=first + 63).next().is_none()
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
            if let Some((_, after)) = rotations.get(&key)
                && let Some(after_blocked) = self.override_blocks(*after)
            {
                changed = true;
                blocked |= after_blocked;
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

#[cfg(test)]
mod primal_pod_removal_tests {
    use super::*;
    use std::num::NonZeroU32;

    #[test]
    fn volatile_pod_removal_is_exact_and_frees_the_physical_occurrence() -> Result<(), String> {
        let mut facts = BundleFacts::default();
        facts.movement_definitions.insert(
            39176,
            EntryFacts {
                definition: MapDefinition::Item(NonZeroU32::MIN),
                terrain_kind: None,
                appearance_id: 39176,
                blocks_projectile: false,
                pickupable: false,
                bound: false,
                count: 1,
                sub_type: 0,
            },
        );
        let position = TilePos {
            x: 1,
            y: 2,
            floor: -7,
        };
        assert!(facts.create_movement_item(position, 39176, 0, || true));
        assert!(facts.create_movement_item(position, 39176, 0, || true));
        let key = facts
            .callbacks
            .lock()
            .map_err(|_| "callback lock")?
            .movement_items
            .get(&position)
            .and_then(|entries| entries.first())
            .and_then(|entry| match entry.source {
                view::EntrySource::Added(id) => Some(id.get()),
                _ => None,
            })
            .ok_or("created pod occurrence")?;
        assert!(!facts.remove_movement_item(TilePos { x: 2, ..position }, key, 39176));
        assert!(!facts.remove_movement_item(position, key, 39175));
        assert!(!facts.remove_movement_item(position, key | (1 << 62), 39176));
        assert!(facts.remove_movement_item(position, key, 39176));
        assert!(!facts.remove_movement_item(position, key, 39176));
        let state = facts.callbacks.lock().map_err(|_| "callback lock")?;
        assert_eq!(state.movement_items.get(&position).map(Vec::len), Some(1));
        assert_eq!(state.movement_sequence, 3);
        Ok(())
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;

    #[test]
    fn source_reset_deadline_cannot_overwrite_a_newer_transform() {
        let facts = BundleFacts::default();
        let original = EntryFacts {
            definition: MapDefinition::Item(NonZeroU32::MIN),
            terrain_kind: None,
            appearance_id: 10186,
            blocks_projectile: false,
            pickupable: false,
            bound: true,
            count: 1,
            sub_type: 0,
        };
        let active = EntryFacts {
            appearance_id: 10187,
            ..original
        };
        let reset = QuestCallbackProjection::TimedTransform {
            due: 10_000_000,
            restore: original,
        };
        assert!(facts.apply_quest_callback(1, 0, active, &[], &[], false, Some(&reset)));
        assert_eq!(facts.expire_movement_items(9_999_999), 0);
        assert_eq!(facts.object_revision(1), 1);
        assert_eq!(facts.expire_movement_items(10_000_000), 1);
        assert_eq!(facts.object_revision(1), 2);
        assert!(facts.apply_quest_callback(1, 2, active, &[], &[], false, Some(&reset)));
        assert!(facts.apply_quest_callback(1, 3, active, &[], &[], false, None));
        assert_eq!(facts.expire_movement_items(10_000_001), 0);
        assert_eq!(facts.object_revision(1), 4);
    }
}

#[cfg(test)]
mod readset_tests {
    use super::*;
    #[test]
    fn multi_cell_readset_refuses_stale_unchanged_cell_without_partial_writes() {
        let facts = BundleFacts::default();
        let before = EntryFacts {
            definition: MapDefinition::Item(NonZeroU32::MIN),
            terrain_kind: None,
            appearance_id: 9935,
            blocks_projectile: false,
            pickupable: false,
            bound: true,
            count: 1,
            sub_type: 0,
        };
        let after = EntryFacts {
            appearance_id: 9933,
            ..before
        };
        // A transformed source state need not occur anywhere in the pinned base palette.
        let mut served = BundleFacts::default();
        served.movement_definitions.insert(9933, after);
        let key = placement_key(
            TilePos {
                x: 33352,
                y: 31126,
                floor: -5,
            },
            1,
        );
        assert_eq!(
            key.and_then(|key| served.quest_callback_replacement(key, 9933)),
            Some(after)
        );

        let projection = QuestCallbackProjection::ReadsetTransforms {
            entries: vec![(10, 0, Some(after)), (11, 0, None)],
            reset: Some((5_000_000, before)),
        };
        assert!(facts.apply_quest_callback(1, 0, after, &[], &[], false, Some(&projection)));
        assert_eq!(facts.object_revision(10), 1);
        assert_eq!(facts.object_revision(11), 0);
        assert_eq!(facts.expire_movement_items(4_999_999), 0);
        assert_eq!(facts.expire_movement_items(5_000_000), 1);
        assert!(facts.apply_quest_callback(11, 0, after, &[], &[], false, None));
        let stale = QuestCallbackProjection::ReadsetTransforms {
            entries: vec![(10, 1, Some(before)), (11, 0, None)],
            reset: None,
        };
        assert!(!facts.apply_quest_callback(1, 2, after, &[], &[], false, Some(&stale)));
        assert_eq!(facts.object_revision(1), 2);
        assert_eq!(facts.object_revision(10), 1);
        assert_eq!(facts.object_revision(11), 1);
        assert_eq!(facts.next_movement_decay_time(), None);
    }
}
