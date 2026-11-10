//! Channel-local callback projection, sharing the bundle's existing fact/view owner.
use super::*;
use sha2::{Digest, Sha256};

// A committed callback can replace or remove one exact native door. Keep every
// other door on the tile authoritative; an overlay never opens its neighbours.
impl crate::map::floor::CellBlocker for BundleFacts {
    fn blocks(&self, cell: crate::content::LogicalCell) -> bool {
        let (Ok(x), Ok(y), Ok(z)) = (
            u16::try_from(cell.x),
            u16::try_from(cell.y),
            u8::try_from(cell.z),
        ) else {
            return false;
        };
        if z > 15 {
            return false;
        }
        let pos = TilePos {
            x,
            y,
            floor: -(z as i8),
        };
        let Ok(states) = self.callbacks.lock() else {
            return true;
        };
        self.doors.blocks_unless(pos, |key| {
            states.hidden.contains(&key) || states.passable.get(&key) == Some(&true)
        })
    }
}

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
    Portals {
        endpoints: [(TilePos, TilePos, EntryFacts); 2],
        expires: Option<u64>,
    },
    ItemPresence {
        position: TilePos,
        entry: EntryFacts,
    },
    Scene {
        mutations: Vec<QuestSceneMutation>,
        remove_self: bool,
    },
    ItemCycle {
        position: TilePos,
        remove: Option<u64>,
        create: Option<EntryFacts>,
    },
}

#[derive(Clone, Debug)]
struct NaginataPick {
    session: [u8; 16],
    command: u64,
    binding: [u8; 32],
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
    portals: BTreeMap<TilePos, DynamicPortal>,
    solid_items: BTreeSet<TilePos>,
    presences: BTreeMap<u64, (TilePos, EntryId)>,
    naginata_picks: Vec<NaginataPick>,
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
            QuestCallbackProjection::Scene {
                mutations,
                remove_self,
            } => {
                if !project_scene(self, owner, mutations, *remove_self) {
                    return false;
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
        let used = self.naginata_picks.len()
            + self.changed.len()
            + self.hidden.len()
            + self.movement_decays.len()
            + self.movement_effects.len()
            + self.movement_speech.len()
            + self.portals.len()
            + self.solid_items.len()
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

    /// The six source rock hits are Channel-local, like the rock's removal.
    /// Retain their exact command payload bindings beside that projection across reconnect.
    pub fn naginata_pick_replay(
        &self,
        session: [u8; 16],
        command: u64,
        payload: &[u8],
    ) -> Option<bool> {
        let binding: [u8; 32] = Sha256::digest(payload).into();
        self.callbacks
            .lock()
            .ok()?
            .naginata_picks
            .iter()
            .find(|hit| hit.session == session && hit.command == command)
            .map(|hit| hit.binding == binding)
    }

    /// Crystal register_actions.lua:751-774: the sixth shared pick removes the
    /// exact Naginata rock. The runtime owner applies its physical hit in the same
    /// turn; a refused hit leaves both the counter and the map unchanged.
    #[allow(
        clippy::too_many_arguments,
        reason = "exact map state, command binding and health-owner commit"
    )]
    pub fn apply_naginata_pick(
        &self,
        key: u64,
        expected: u64,
        after: EntryFacts,
        session: [u8; 16],
        command: u64,
        payload: &[u8],
        hit: impl FnOnce() -> bool,
    ) -> bool {
        let Some((pos, _)) = view::placement_of(key) else {
            return false;
        };
        if pos
            != (TilePos {
                x: 32356,
                y: 32074,
                floor: -10,
            })
            || after.appearance_id != 1791
            || self.action_at(key) != Some(40031)
            || command == 0
            || payload.len() > oteryn_protocol_oteryn::world_object::MAX_USE_INTENT_BYTES
        {
            return false;
        }
        let binding = Sha256::digest(payload).into();
        let Ok(mut states) = self.callbacks.lock() else {
            return false;
        };
        if states.naginata_picks.len() >= 6
            || states
                .naginata_picks
                .iter()
                .any(|hit| hit.session == session && hit.command == command)
            || states
                .changed
                .get(&key)
                .map_or(0, |(revision, _)| *revision)
                != expected
            || expected != states.naginata_picks.len() as u64
            || !states.has_capacity(2)
        {
            return false;
        }
        let states = Arc::make_mut(&mut states);
        if states.naginata_picks.try_reserve(1).is_err() || !hit() {
            return false;
        }
        states.naginata_picks.push(NaginataPick {
            session,
            command,
            binding,
        });
        states.changed.insert(key, (expected + 1, after));
        if states.naginata_picks.len() == 6 {
            states.hidden.insert(key);
        }
        true
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

    /// The bath's occupied state cannot be picked up while its player stands inside it.
    pub fn transform_movement_item(&self, key: u64, appearance: u16) -> bool {
        let Some(mut after) = self.movement_item_facts(appearance) else {
            return false;
        };
        if appearance == 26100 {
            after.pickupable = false;
        }
        self.apply_quest_callback(key, self.object_revision(key), after, &[], &[], false, None)
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
    /// Effect zero creates a silent source object, such as the Devil Helmet sewer.
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
        if !states.has_capacity(1 + usize::from(effect != 0)) {
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
        if effect != 0 {
            if states.movement_effects.len() == 512 {
                states.movement_effects.pop_front();
            }
            states.movement_effects.push_back((sequence, pos, effect));
        }
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
                    .filter(|(sequence, _, _)| *sequence > after)
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

    /// Source UID/AID bindings keep their explicit owner when a position callback overlaps.
    pub fn quest_callback_source_bound(&self, key: u64) -> bool {
        self.placements
            .binary_search_by_key(&key, |(key, _)| *key)
            .is_ok_and(|at| {
                let attrs = &self.placements[at].1.attrs;
                attrs.unique.is_some() || attrs.action.is_some()
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
        let replacement = entries.next()?;
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
                passages.len()
                    + replacements.len()
                    + match projection {
                        Some(QuestCallbackProjection::Scene { mutations, .. }) => {
                            mutations.len().saturating_mul(3).saturating_add(1)
                        }
                        Some(_) => 8,
                        None => 0,
                    },
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

    /// Qualify the future open state of exactly one selected native door.
    /// Other solid entries, live callback effects and other closed doors remain blocking.
    pub fn opening_door_enterable(&self, base: &super::super::WorldBase, key: u64) -> bool {
        let Some((pos, _)) = view::placement_of(key) else {
            return false;
        };
        if !self.doors.contains(key) || self.doors.blocks_except(pos, Some(key)) {
            return false;
        }
        let Some(tile) = base.tile(pos.x, pos.y, pos.floor) else {
            return false;
        };
        let Ok(states) = self.callbacks.lock() else {
            return false;
        };
        if !tile.walkable()
            || states.solid_items.contains(&pos)
            || states.changed.contains_key(&key)
            || states.hidden.contains(&key)
            || states.passable.contains_key(&key)
        {
            return false;
        }
        let mut ordinal = 0_u8;
        for (&id, &depth) in tile.ids().iter().zip(tile.depths()) {
            if depth != 0 {
                continue;
            }
            let Some(entry) = placement_key(pos, ordinal) else {
                return false;
            };
            ordinal = ordinal.saturating_add(1);
            if entry == key || states.hidden.contains(&entry) {
                continue;
            }
            if let Some(passable) = states.passable.get(&entry) {
                if !passable {
                    return false;
                }
                continue;
            }
            if self.doors.contains(entry) {
                continue;
            }
            let palette = self
                .placements
                .binary_search_by_key(&entry, |(k, _)| *k)
                .ok()
                .map(|i| self.placements[i].1.palette)
                .or_else(|| self.unique_index(id));
            if palette.is_none_or(|p| self.blocks(p)) {
                return false;
            }
        }
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
        if states.solid_items.contains(&pos) {
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

/// A fully resolved edit; only the projection owner can publish these together.
#[derive(Clone, Debug)]
pub enum QuestSceneMutation {
    Restore {
        key: u64,
        revision: u64,
        facts: EntryFacts,
        passable: bool,
    },
    Remove {
        key: u64,
        revision: u64,
    },
    Replace {
        key: u64,
        revision: u64,
        facts: EntryFacts,
        passable: bool,
    },
    Create {
        position: TilePos,
        facts: EntryFacts,
        passable: bool,
    },
}

fn project_scene(
    states: &mut CallbackStates,
    owner: u64,
    edits: &[QuestSceneMutation],
    remove_self: bool,
) -> bool {
    if edits.len() > 32 {
        return false;
    }
    for edit in edits {
        if matches!(edit, QuestSceneMutation::Create { facts, .. } | QuestSceneMutation::Replace { facts, .. } | QuestSceneMutation::Restore { facts, .. } if facts.pickupable)
        {
            return false;
        }
        if let QuestSceneMutation::Remove { key, revision }
        | QuestSceneMutation::Replace { key, revision, .. } = edit
            && (*key == owner
                || *revision == u64::MAX
                || states.hidden.contains(key)
                || states.changed.get(key).map_or(0, |(current, _)| *current) != *revision)
        {
            return false;
        }
    }
    for edit in edits {
        if let QuestSceneMutation::Restore { key, revision, .. } = edit
            && (*key == owner
                || *revision == u64::MAX
                || !states.hidden.contains(key)
                || states.changed.get(key).map_or(0, |(current, _)| *current) != *revision)
        {
            return false;
        }
    }
    for edit in edits {
        match edit {
            QuestSceneMutation::Restore {
                key,
                revision,
                facts,
                passable,
            } => {
                states.hidden.remove(key);
                states.changed.insert(*key, (revision + 1, *facts));
                states.passable.insert(*key, *passable);
            }
            QuestSceneMutation::Remove { key, .. } => {
                states.hidden.insert(*key);
                states.passable.remove(key);
            }
            QuestSceneMutation::Replace {
                key,
                revision,
                facts,
                passable,
            } => {
                states.changed.insert(*key, (revision + 1, *facts));
                states.passable.insert(*key, *passable);
            }
            QuestSceneMutation::Create {
                position,
                facts,
                passable,
            } => {
                let Some(sequence) = states.movement_sequence.checked_add(1) else {
                    return false;
                };
                let Some(entry) = EntryId::callback(sequence) else {
                    return false;
                };
                states.movement_sequence = sequence;
                states
                    .movement_items
                    .entry(*position)
                    .or_default()
                    .push(view::ComposedEntry {
                        source: view::EntrySource::Added(entry),
                        facts: *facts,
                    });
                if !passable {
                    states.solid_items.insert(*position);
                }
            }
        }
    }
    if remove_self {
        states.hidden.insert(owner);
    }
    true
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
mod door_projection_tests {
    use super::*;
    use crate::content::LogicalCell;
    use crate::map::floor::CellBlocker;
    use std::num::NonZeroU32;

    #[test]
    fn callback_opening_excludes_only_the_exact_projected_door() -> Result<(), String> {
        let pos = TilePos {
            x: 6,
            y: 7,
            floor: -8,
        };
        let first = placement_key(pos, 0).ok_or("first door")?;
        let second = placement_key(pos, 1).ok_or("second door")?;
        let open = EntryFacts {
            definition: MapDefinition::Item(NonZeroU32::MIN),
            terrain_kind: None,
            appearance_id: 1643,
            blocks_projectile: false,
            pickupable: false,
            bound: false,
            count: 1,
            sub_type: 0,
        };
        let mut facts = BundleFacts::default();
        for key in [first, second] {
            facts
                .doors
                .push(key, true, false, open, false, None, None, None);
        }
        facts.doors.finish();
        let cell = LogicalCell { x: 6, y: 7, z: 8 };
        assert!(CellBlocker::blocks(&facts, cell));
        {
            let mut callbacks = facts.callbacks.lock().map_err(|_| "callbacks")?;
            Arc::make_mut(&mut callbacks).hidden.insert(first);
        }
        assert!(
            CellBlocker::blocks(&facts, cell),
            "a different closed door still blocks"
        );
        {
            let mut callbacks = facts.callbacks.lock().map_err(|_| "callbacks")?;
            Arc::make_mut(&mut callbacks).passable.insert(second, true);
        }
        assert!(!CellBlocker::blocks(&facts, cell));
        {
            let mut callbacks = facts.callbacks.lock().map_err(|_| "callbacks")?;
            Arc::make_mut(&mut callbacks).passable.insert(second, false);
        }
        assert!(
            CellBlocker::blocks(&facts, cell),
            "a later closing projection blocks again"
        );
        Ok(())
    }
}
