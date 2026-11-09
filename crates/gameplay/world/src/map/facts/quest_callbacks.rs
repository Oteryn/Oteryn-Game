//! Channel-local callback projection, sharing the bundle's existing fact/view owner.
use super::*;

#[path = "elemental_arrival.rs"]
mod elemental_arrival;
pub use elemental_arrival::ElementalArrivalReservation;

#[path = "quest_item_batch.rs"]
mod item_batch;
pub use item_batch::QuestMapItemMove;
#[path = "quest_timed_passages.rs"]
mod timed_passages;
pub use timed_passages::QuestTimedPassage;

#[cfg(test)]
#[path = "quest_timed_passages_tests.rs"]
mod timed_passages_tests;

#[path = "rafzane_elevator.rs"]
mod rafzane_elevator;
pub use rafzane_elevator::RafzaneElevator;

#[path = "arena_pillars.rs"]
mod arena_pillars;
pub use arena_pillars::ArenaPillarProjection;

#[derive(Clone, Debug)]
struct MovementDecay {
    revision: u64,
    restore: EntryFacts,
    /// Remote resets restore visibility and the previous collision override too.
    overlay: Option<(bool, Option<bool>)>,
}

#[derive(Clone, Debug)]
struct DynamicPortal {
    owner: u64,
    destination: TilePos,
    entry: Option<EntryId>,
    owner_revision: Option<u64>,
    expires: Option<u64>,
}

/// Already qualified facts for one atomic, Channel-local callback projection.
pub enum QuestCallbackProjection {
    /// Restore exactly this revision at an original source deadline.
    TimedTransform { due: u64, restore: EntryFacts },
    /// A source glyph and its native portal share one revision-fenced deadline.
    TimedEntrance {
        due: u64,
        restore: EntryFacts,
        position: TilePos,
        destination: TilePos,
    },
    /// Change a separate shared object and restore only that committed revision.
    TimedRemoteTransform {
        key: u64,
        expected: u64,
        after: EntryFacts,
        restore: EntryFacts,
        due: u64,
    },
    /// A separately selected source object; its own revision fences restoration.
    TimedTargetTransform {
        key: u64,
        expected: u64,
        after: EntryFacts,
        restore: EntryFacts,
        due: u64,
    },
    /// Restore a bounded remote group independently of the source lever deadline.
    TimedPassages {
        due: u64,
        entries: Vec<QuestTimedPassage>,
        source_reset: Option<(u64, EntryFacts)>,
    },
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
    pub(super) feroxa_phase: Option<Arc<super::feroxa_map_phases::FeroxaMapPhase>>,
    pub(super) reserved: BTreeSet<u64>,
    pub(super) reserved_positions: BTreeSet<TilePos>,
    pub(super) changed: BTreeMap<u64, (u64, EntryFacts)>,
    pub(super) actions: BTreeMap<u64, Option<u16>>,
    pub(super) hidden: BTreeSet<u64>,
    passable: BTreeMap<u64, bool>,
    arena_pillars: BTreeMap<u64, [u8; 16]>,
    arena_recreated: BTreeMap<TilePos, arena_pillars::RecreatedPillar>,
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
    fn key_reserved(&self, key: u64) -> bool {
        self.reserved.contains(&key)
            || view::placement_of(key)
                .map(|(pos, _)| pos)
                .or_else(|| self.movement_item(key).map(|(pos, _)| pos))
                .is_some_and(|position| self.reserved_positions.contains(&position))
    }
    fn position_reserved(&self, position: TilePos) -> bool {
        self.reserved_positions.contains(&position)
            || self.reserved.iter().any(|key| {
                view::placement_of(*key)
                    .map(|(pos, _)| pos)
                    .or_else(|| self.movement_item(*key).map(|(pos, _)| pos))
                    == Some(position)
            })
    }

    fn remove_portals(&mut self, owner: u64) {
        self.portals.retain(|pos, portal| {
            if portal.owner != owner {
                return true;
            }
            if let Some(id) = portal.entry
                && let Some(entries) = self.movement_items.get_mut(pos)
            {
                entries.retain(|entry| entry.source != view::EntrySource::Added(id));
                if entries.is_empty() {
                    self.movement_items.remove(pos);
                }
            }
            false
        });
    }

    fn readset_matches(&self, entries: impl IntoIterator<Item = (u64, u64)>) -> bool {
        let mut keys = BTreeSet::new();
        for (key, revision) in entries {
            if keys.len() == 8
                || !keys.insert(key)
                || revision == u64::MAX
                || self.hidden.contains(&key)
                || self.changed.get(&key).map_or(0, |(current, _)| *current) != revision
            {
                return false;
            }
        }
        !keys.is_empty()
    }

    fn project(&mut self, owner: u64, projection: &QuestCallbackProjection, close: bool) -> bool {
        match projection {
            QuestCallbackProjection::TimedEntrance {
                due,
                restore,
                position,
                destination,
            } => {
                if close
                    || view::placement_of(owner).map(|(pos, _)| pos) != Some(*position)
                    || self
                        .portals
                        .get(position)
                        .is_some_and(|portal| portal.owner != owner)
                {
                    return false;
                }
                let Some(revision) = self
                    .changed
                    .get(&owner)
                    .map_or(0, |(r, _)| *r)
                    .checked_add(1)
                else {
                    return false;
                };
                self.remove_portals(owner);
                self.movement_decays.insert(
                    (*due, owner),
                    MovementDecay {
                        revision,
                        restore: *restore,
                        overlay: None,
                    },
                );
                self.portals.insert(
                    *position,
                    DynamicPortal {
                        owner,
                        destination: *destination,
                        entry: None,
                        owner_revision: Some(revision),
                        expires: Some(*due),
                    },
                );
            }
            QuestCallbackProjection::TimedRemoteTransform {
                key,
                expected,
                after,
                restore,
                due,
            } => {
                if *key == owner
                    || *expected == u64::MAX
                    || self.hidden.contains(key)
                    || self.changed.get(key).map_or(0, |(revision, _)| *revision) != *expected
                {
                    return false;
                }
                let revision = expected + 1;
                self.changed.insert(*key, (revision, *after));
                self.movement_decays.insert(
                    (*due, *key),
                    MovementDecay {
                        revision,
                        restore: *restore,
                        overlay: None,
                    },
                );
            }
            QuestCallbackProjection::TimedTargetTransform {
                key,
                expected,
                after,
                restore,
                due,
            } => {
                if *key == owner
                    || self.hidden.contains(key)
                    || self.changed.get(key).map_or(0, |(r, _)| *r) != *expected
                {
                    return false;
                }
                let Some(revision) = expected.checked_add(1) else {
                    return false;
                };
                self.changed.insert(*key, (revision, *after));
                self.movement_decays.insert(
                    (*due, *key),
                    MovementDecay {
                        revision,
                        restore: *restore,
                        overlay: None,
                    },
                );
            }
            QuestCallbackProjection::TimedPassages {
                due,
                entries,
                source_reset,
            } => {
                if close || !self.project_timed_passages(owner, *due, entries, *source_reset) {
                    return false;
                }
            }
            QuestCallbackProjection::ReadsetTransforms { entries, reset } => {
                if entries.iter().any(|(key, _, _)| *key == owner)
                    || !self
                        .readset_matches(entries.iter().map(|(key, revision, _)| (*key, *revision)))
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
                            overlay: None,
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
                        overlay: None,
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
                            entry: Some(entry),
                            owner_revision: None,
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
    pub(super) fn phase_has_dynamic_blocker(&self, pos: TilePos) -> bool {
        self.solid_items.contains(&pos) || self.batch_blockers.contains(&pos)
    }

    pub(super) fn clear_feroxa_scenery(&mut self, inside: impl Fn(TilePos) -> bool) {
        let key_inside = |key: &u64| view::placement_of(*key).is_some_and(|(p, _)| inside(p));
        self.changed.retain(|key, _| !key_inside(key));
        self.hidden.retain(|key| !key_inside(key));
        self.passable.retain(|key, _| !key_inside(key));
        self.movement_decays.retain(|(_, key), _| !key_inside(key));
        let cleared = |p: &TilePos| {
            p.floor == -11 && (33415..=33445).contains(&p.x) && (31522..=31554).contains(&p.y)
        };
        self.movement_items.retain(|p, _| !cleared(p));
        self.solid_items.retain(|p| !cleared(p));
        self.batch_blockers.retain(|p| !inside(*p));
        self.portals.retain(|p, _| !cleared(p));
        self.presences.retain(|_, (p, _)| !cleared(p));
    }

    pub(super) fn has_capacity(&self, additional: usize) -> bool {
        let used = self.feroxa_phase.as_ref().map_or(0, |p| p.footprint())
            + self.reserved.len()
            + self.arena_pillars.len()
            + self.arena_recreated.len()
            + self.reserved_positions.len()
            + self.changed.len()
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
    /// Fence up to eight already qualified map placements without transforming
    /// them. The synchronous consequence runs under the same map owner lock,
    /// so an arrival can publish its prepared spawn while charges stay intact.
    /// The caller retains the Channel owner and must not reenter map callbacks
    /// from `publish`, or perform asynchronous/durable work inside this lock.
    pub fn with_quest_readset<R>(
        &self,
        readset: &[(u64, u64)],
        publish: impl FnOnce() -> R,
    ) -> Option<R> {
        let states = self.callbacks.lock().ok()?;
        if !states.readset_matches(readset.iter().copied()) {
            return None;
        }
        let result = publish();
        drop(states);
        Some(result)
    }

    /// Bind a protected movement-created Item to the callback which owns its
    /// presence. Readsets fence the owner's revision, including removal/expiry,
    /// rather than treating a volatile entry id as an immutable base placement.
    pub fn quest_callback_presence_readset(
        &self,
        position: TilePos,
        entry: EntryId,
    ) -> Option<(u64, u64)> {
        let states = self.callbacks.lock().ok()?;
        let (&owner, _) = states
            .presences
            .iter()
            .find(|(_, (pos, id))| *pos == position && *id == entry)?;
        let revision = states.changed.get(&owner).map_or(0, |(rev, _)| *rev);
        states
            .readset_matches([(owner, revision)])
            .then_some((owner, revision))
    }

    /// A transformed/removed callback entry changes floor routing with its visible stack.
    /// Unchanged tiles keep the compact boot-time table and incur no stack scan.
    pub(crate) fn quest_callback_floor_mask(
        &self,
        base: &super::super::WorldBase,
        pos: TilePos,
    ) -> Option<u8> {
        let states = self.callbacks.lock().ok().map(|s| Arc::clone(&s))?;
        let first = placement_key(pos, 0)?;
        if states.phase_entry(pos, 0).is_some() {
            return Some(0);
        }
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
        if states.key_reserved(key) {
            return false;
        }
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
        states.movement_decays.insert(
            (due, key),
            MovementDecay {
                revision,
                restore,
                overlay: None,
            },
        );
        true
    }

    pub fn next_movement_decay_time(&self) -> Option<u64> {
        let states = self.callbacks.lock().ok()?;
        states
            .movement_decays
            .iter()
            .find(|((_, key), _)| !states.key_reserved(*key))
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
        let due: Vec<_> = states
            .movement_decays
            .range(..=(now, u64::MAX))
            .filter(|((_, key), _)| !states.key_reserved(*key))
            .map(|(key, _)| *key)
            .collect();
        for at in due {
            let Some(decay) = states.movement_decays.remove(&at) else {
                continue;
            };
            let key = at.1;
            if states
                .changed
                .get(&key)
                .is_some_and(|(revision, _)| *revision == decay.revision)
                && let Some(revision) = decay.revision.checked_add(1)
            {
                states.changed.insert(key, (revision, decay.restore));
                if let Some((hidden, passable)) = decay.overlay {
                    if hidden {
                        states.hidden.insert(key);
                    } else {
                        states.hidden.remove(&key);
                    }
                    if let Some(passable) = passable {
                        states.passable.insert(key, passable);
                    } else {
                        states.passable.remove(&key);
                    }
                }
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
        if key >> 62 == 2 {
            return self.compare_transform_movement_item(
                key,
                self.object_revision(key),
                appearance,
            );
        }
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
    /// Transform one callback-created item without replacing its native identity.
    /// Base, durable Ground/House and stable world-change objects have other owners.
    pub fn compare_transform_movement_item(
        &self,
        key: u64,
        expected: u64,
        appearance: u16,
    ) -> bool {
        if key >> 62 != 2 || expected == u64::MAX {
            return false;
        }
        let Some(mut after) = self.movement_item_facts(appearance) else {
            return false;
        };
        if appearance == 26100 {
            after.pickupable = false;
        }
        let Ok(mut states) = self.callbacks.lock() else {
            return false;
        };
        if states
            .changed
            .get(&key)
            .map_or(0, |(revision, _)| *revision)
            != expected
            || !states.has_capacity(usize::from(!states.changed.contains_key(&key)))
        {
            return false;
        }
        let Some((pos, id)) = states.movement_item(key) else {
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
        after.bound = entry.facts.bound;
        after.count = entry.facts.count;
        entry.facts = after;
        states.changed.insert(key, (expected + 1, after));
        true
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
        if states.position_reserved(pos) {
            return false;
        }
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
        if states.position_reserved(pos) {
            return false;
        }
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
        if states.position_reserved(pos) {
            return false;
        }
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
        if states.position_reserved(pos) {
            return false;
        }
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
        if states.position_reserved(pos) {
            return Some(false);
        }
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

    /// Only live, current portals resolve; the timer and explicit removal retire both ends.
    pub fn quest_portal_destination(&self, pos: TilePos, now: u64) -> Option<TilePos> {
        let states = self.callbacks.lock().ok()?;
        let portal = states.portals.get(&pos)?;
        if portal.owner_revision.is_some_and(|revision| {
            states.hidden.contains(&portal.owner)
                || states.changed.get(&portal.owner).map_or(0, |(r, _)| *r) != revision
        }) {
            return None;
        }
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
            s.changed
                .get(&key)
                .map_or_else(|| s.phase_revision(key).unwrap_or(0), |(r, _)| *r)
                == expected
                && expected < u64::MAX
                && !s.key_reserved(key)
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
        let projection_capacity = match projection {
            Some(QuestCallbackProjection::TimedPassages {
                entries,
                source_reset,
                ..
            }) => {
                // Changed entry, hidden entry and timer, plus the source revision/reset.
                if !passages.is_empty() || !replacements.is_empty() {
                    return false;
                }
                entries
                    .len()
                    .saturating_mul(3)
                    .saturating_add(1 + usize::from(source_reset.is_some()))
            }
            _ => usize::from(projection.is_some()) * 8,
        };
        if states
            .changed
            .get(&key)
            .map_or_else(|| states.phase_revision(key).unwrap_or(0), |(r, _)| *r)
            != expected
            || states.key_reserved(key)
            || passages.iter().any(|key| states.key_reserved(*key))
            || replacements
                .iter()
                .any(|(key, _, _)| states.key_reserved(*key))
            || projection.is_some_and(|projection| match projection {
                QuestCallbackProjection::ReadsetTransforms { entries, .. } => {
                    entries.iter().any(|(key, _, _)| states.key_reserved(*key))
                }
                QuestCallbackProjection::ItemPresence { position, .. }
                | QuestCallbackProjection::ItemCycle { position, .. } => {
                    states.position_reserved(*position)
                }
                QuestCallbackProjection::Portals { endpoints, .. } => endpoints
                    .iter()
                    .any(|(pos, _, _)| states.position_reserved(*pos)),
                QuestCallbackProjection::TimedPassages { entries, .. } => entries
                    .iter()
                    .any(|entry| states.reserved.contains(&entry.key)),
                QuestCallbackProjection::TimedRemoteTransform { key, .. }
                | QuestCallbackProjection::TimedTargetTransform { key, .. } => {
                    states.key_reserved(*key)
                }
                QuestCallbackProjection::TimedEntrance { position, .. } => {
                    states.position_reserved(*position)
                }
                QuestCallbackProjection::TimedTransform { .. } => false,
            })
            || expected == u64::MAX
            || passages.len() + replacements.len() > 8
            || !states.has_capacity(passages.len() + replacements.len() + projection_capacity)
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
        if states.key_reserved(key) || states.key_reserved(wall) {
            return false;
        }
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
    /// Cake's donor bridge replaces the original ground and removes its stack.
    /// Restore clears the movement override so the original terrain owns collision again.
    pub fn install_cake_bridge_state(
        &self,
        key: u64,
        revision: u64,
        before: EntryFacts,
        after: Option<EntryFacts>,
    ) -> bool {
        let Some((pos, _)) = view::placement_of(key) else {
            return false;
        };
        if pos.floor != -7
            || !(33300..=33306).contains(&pos.x)
            || !(32052..=32059).contains(&pos.y)
            || after.is_some_and(|facts| {
                facts != before && !matches!(facts.appearance_id, 48227 | 12113)
            })
            || !self.install_gathering_state(key, revision, before, after)
        {
            return false;
        }
        let Ok(mut states) = self.callbacks.lock() else {
            return false;
        };
        let states = Arc::make_mut(&mut states);
        if after.is_some_and(|facts| facts != before) {
            states.passable.insert(key, true);
        } else {
            states.passable.remove(&key);
        }
        true
    }

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
        if states.solid_items.contains(&pos)
            || states.batch_blockers.contains(&pos)
            || states.arena_pillar_closed_at(pos)
        {
            return true;
        }
        if states.phase_entry(pos, 0).is_some() {
            return states
                .phase_spell(pos)
                .ok()
                .flatten()
                .is_none_or(|tile| tile.flags().block_solid);
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
    fn remote_source_reset_is_atomic_and_fences_the_target_revision() -> Result<(), String> {
        let facts = BundleFacts::default();
        let original = EntryFacts {
            definition: MapDefinition::Item(NonZeroU32::MIN),
            terrain_kind: None,
            appearance_id: 2074,
            blocks_projectile: false,
            pickupable: false,
            bound: true,
            count: 1,
            sub_type: 0,
        };
        let active = EntryFacts {
            appearance_id: 2075,
            ..original
        };
        let timer = |expected, due| QuestCallbackProjection::TimedTargetTransform {
            key: 2,
            expected,
            after: active,
            restore: original,
            due,
        };
        assert!(facts.apply_quest_callback(
            1,
            0,
            original,
            &[],
            &[],
            false,
            Some(&timer(0, 15_000_000))
        ));
        assert_eq!((facts.object_revision(1), facts.object_revision(2)), (1, 1));
        assert_eq!(facts.expire_movement_items(14_999_999), 0);
        assert!(facts.apply_quest_callback(2, 1, active, &[], &[], false, None));
        assert_eq!(facts.expire_movement_items(15_000_000), 0);
        assert!(facts.apply_quest_callback(
            1,
            1,
            original,
            &[],
            &[],
            false,
            Some(&timer(2, 30_000_000))
        ));
        assert!(!facts.apply_quest_callback(
            1,
            2,
            original,
            &[],
            &[],
            false,
            Some(&timer(1, 31_000_000))
        ));
        assert_eq!((facts.object_revision(1), facts.object_revision(2)), (2, 3));
        assert_eq!(facts.next_movement_decay_time(), Some(30_000_000));
        assert_eq!(facts.expire_movement_items(30_000_000), 1);
        assert_eq!((facts.object_revision(1), facts.object_revision(2)), (2, 4));
        let state = facts.callbacks.lock().map_err(|_| "callback lock")?;
        assert_eq!(
            state.changed.get(&2).map(|(_, value)| value.appearance_id),
            Some(2074)
        );
        Ok(())
    }

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
    fn shared_entrance_traversal_keeps_the_source_deadline_and_revision() -> Result<(), String> {
        let facts = BundleFacts::default();
        let source = TilePos {
            x: 32849,
            y: 32233,
            floor: -9,
        };
        let destination = TilePos {
            x: 32857,
            y: 32234,
            floor: -11,
        };
        let owner = placement_key(source, 1).ok_or("source key")?;
        let ashes = EntryFacts {
            definition: MapDefinition::Item(NonZeroU32::MIN),
            terrain_kind: None,
            appearance_id: 3134,
            blocks_projectile: false,
            pickupable: false,
            bound: true,
            count: 1,
            sub_type: 0,
        };
        let portal = EntryFacts {
            appearance_id: 1949,
            ..ashes
        };
        let entrance = QuestCallbackProjection::TimedEntrance {
            due: 20,
            restore: ashes,
            position: source,
            destination,
        };
        assert!(facts.apply_quest_callback(owner, 0, portal, &[], &[], false, Some(&entrance)));
        assert_eq!(
            facts.quest_portal_destination(source, 10),
            Some(destination)
        );
        assert_eq!(
            facts.quest_portal_destination(source, 19),
            Some(destination)
        );
        assert_eq!(facts.object_revision(owner), 1);
        {
            let states = facts.callbacks.lock().map_err(|_| "facts lock")?;
            assert!(
                !states.movement_items.contains_key(&source),
                "the base glyph is the only rendered portal"
            );
        }
        assert_eq!(facts.expire_movement_items(19), 0);
        assert_eq!(facts.quest_portal_destination(source, 20), None);
        assert_eq!(facts.expire_movement_items(20), 2);
        assert_eq!(facts.object_revision(owner), 2);
        assert_eq!(facts.quest_portal_destination(source, 0), None);
        let next = QuestCallbackProjection::TimedEntrance {
            due: 30,
            restore: ashes,
            position: source,
            destination,
        };
        assert!(facts.apply_quest_callback(owner, 2, portal, &[], &[], false, Some(&next)));
        let changed = EntryFacts {
            appearance_id: 1948,
            ..ashes
        };
        assert!(facts.apply_quest_callback(owner, 3, changed, &[], &[], false, None));
        assert_eq!(
            facts.quest_portal_destination(source, 25),
            None,
            "a stale source cannot teleport"
        );
        assert_eq!(
            facts.expire_movement_items(30),
            1,
            "the stale reset cannot overwrite a newer source"
        );
        assert_eq!(facts.object_revision(owner), 4);
        Ok(())
    }

    #[test]
    fn remote_trial_reset_is_atomic_and_revision_fenced() {
        let facts = BundleFacts::default();
        let original = EntryFacts {
            definition: MapDefinition::Item(NonZeroU32::MIN),
            terrain_kind: None,
            appearance_id: 2074,
            blocks_projectile: false,
            pickupable: false,
            bound: true,
            count: 1,
            sub_type: 0,
        };
        let lit = EntryFacts {
            appearance_id: 2075,
            ..original
        };
        let reset = QuestCallbackProjection::TimedRemoteTransform {
            key: 11,
            expected: 0,
            after: lit,
            restore: original,
            due: 15_000_000,
        };
        assert!(facts.apply_quest_callback(10, 0, original, &[], &[], false, Some(&reset)));
        // A second trial source cannot publish against the earlier shared revision.
        assert!(!facts.apply_quest_callback(12, 0, original, &[], &[], false, Some(&reset)));
        assert_eq!(facts.object_revision(12), 0);
        assert_eq!(facts.expire_movement_items(14_999_999), 0);
        assert_eq!(facts.expire_movement_items(15_000_000), 1);
        assert_eq!(facts.object_revision(11), 2);
        let next = QuestCallbackProjection::TimedRemoteTransform {
            key: 11,
            expected: 2,
            after: lit,
            restore: original,
            due: 30_000_000,
        };
        assert!(facts.apply_quest_callback(12, 0, original, &[], &[], false, Some(&next)));
        // An intervening edit owns the newer revision; an old reset cannot overwrite it.
        assert!(facts.apply_quest_callback(11, 3, lit, &[], &[], false, None));
        assert_eq!(facts.expire_movement_items(30_000_000), 0);
        assert_eq!(facts.object_revision(11), 4);
    }

    #[test]
    fn m49_arrival_readset_holds_all_eight_cells_without_mutating_charges() -> Result<(), String> {
        let facts = BundleFacts::default();
        let cells: Vec<_> = (10..18).map(|key| (key, 0)).collect();
        let published = facts.with_quest_readset(&cells, || {
            // Keep the same map owner locked through consequence publication.
            assert!(facts.callbacks.try_lock().is_err());
            49
        });
        assert_eq!(published, Some(49));
        for (key, revision) in &cells {
            assert_eq!(facts.object_revision(*key), *revision);
        }
        let mut calls = 0;
        let mut publish = || {
            calls += 1;
        };
        assert!(facts.with_quest_readset(&[], &mut publish).is_none());
        assert!(
            facts
                .with_quest_readset(&[(10, 0), (10, 0)], &mut publish)
                .is_none()
        );
        assert!(
            facts
                .with_quest_readset(&[(10, u64::MAX)], &mut publish)
                .is_none()
        );
        let too_many: Vec<_> = (10..19).map(|key| (key, 0)).collect();
        assert!(facts.with_quest_readset(&too_many, &mut publish).is_none());
        assert_eq!(calls, 0);
        let mut stale = cells.clone();
        stale[7].1 = 1;
        assert!(facts.with_quest_readset(&stale, || 1).is_none());
        let mut states = facts.callbacks.lock().map_err(|_| "map owner")?;
        Arc::make_mut(&mut states).hidden.insert(17);
        drop(states);
        assert!(facts.with_quest_readset(&cells, || 1).is_none());
        Ok(())
    }

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
