//! Source ItemType appearance cycles on the Channel's immutable map placements.
use super::*;
use oteryn_gameplay_quests::quest::world_callbacks::ObjectCycle as Binding;

const MAX_OBJECT_CYCLES: usize = 256;

#[derive(Clone, Debug)]
pub(super) struct ObjectCycle {
    current: u16,
    due: u64,
    stages: Arc<BTreeMap<u16, Stage>>,
}

#[derive(Clone, Copy, Debug)]
struct Stage {
    facts: EntryFacts,
    duration_us: u64,
    next: u16,
}

impl BundleFacts {
    pub fn quest_object_cycle_matches(&self, key: u64, appearance: u16) -> bool {
        self.callbacks.lock().is_ok_and(|state| {
            !state.hidden.contains(&key)
                && state.phase_revision(key).is_none()
                && state
                    .object_cycles
                    .get(&key)
                    .is_some_and(|cycle| cycle.current == appearance)
                && state
                    .changed
                    .get(&key)
                    .is_none_or(|(_, facts)| facts.appearance_id == appearance)
        })
    }

    /// Qualify an exact unmodified, immovable base entry once at owner startup.
    pub fn initialize_quest_object_cycle(
        &self,
        key: u64,
        base_id: u32,
        binding: &Binding,
        now: u64,
    ) -> bool {
        let Some((pos, ordinal)) = view::placement_of(key) else {
            return false;
        };
        if !self.quest_callback_native_object(key, base_id, binding.initial) {
            return false;
        }
        let Some(base) = self.base_entry(pos, ordinal, base_id) else {
            return false;
        };
        if base.appearance_id != binding.initial || base.pickupable || self.house_tile(pos) {
            return false;
        }
        let mut stages = BTreeMap::new();
        for stage in &binding.stages {
            let Some(mut facts) = self.quest_callback_replacement(key, stage.appearance) else {
                return false;
            };
            if facts.pickupable {
                return false;
            }
            facts.terrain_kind = base.terrain_kind;
            facts.blocks_projectile = base.blocks_projectile;
            facts.bound = base.bound;
            facts.count = base.count;
            facts.sub_type = base.sub_type;
            stages.insert(
                stage.appearance,
                Stage {
                    facts,
                    duration_us: u64::from(stage.duration_ms) * 1_000,
                    next: stage.next,
                },
            );
        }
        let Some(due) = stages
            .get(&binding.initial)
            .and_then(|s| now.checked_add(s.duration_us))
        else {
            return false;
        };
        let Ok(mut state) = self.callbacks.lock() else {
            return false;
        };
        if state.object_cycles.contains_key(&key) {
            return true;
        }
        if state.object_cycles.len() >= MAX_OBJECT_CYCLES
            || state.changed.contains_key(&key)
            || state.phase_revision(key).is_some()
            || state.hidden.contains(&key)
            || state.key_reserved(key)
            || !state.has_capacity(2)
        {
            return false;
        }
        Arc::make_mut(&mut state).object_cycles.insert(
            key,
            ObjectCycle {
                current: binding.initial,
                due,
                stages: Arc::new(stages),
            },
        );
        true
    }
}

impl CallbackStates {
    pub(super) fn next_object_cycle_due(&self) -> Option<u64> {
        self.object_cycles
            .iter()
            .filter(|(key, _)| !self.key_reserved(**key))
            .map(|(_, cycle)| cycle.due)
            .min()
    }

    pub(super) fn advance_object_cycles(&mut self, now: u64) -> usize {
        let due: Vec<_> = self
            .object_cycles
            .iter()
            .filter(|(key, cycle)| cycle.due <= now && !self.key_reserved(**key))
            .map(|(key, _)| *key)
            .collect();
        let mut advanced = 0;
        for key in due {
            let Some(mut cycle) = self.object_cycles.remove(&key) else {
                continue;
            };
            let current = self.changed.get(&key).copied();
            if self.hidden.contains(&key)
                || self.phase_revision(key).is_some()
                || current.is_some_and(|(_, facts)| facts.appearance_id != cycle.current)
            {
                continue;
            }
            let Some(revision) = current.map_or(0, |(revision, _)| revision).checked_add(1) else {
                continue;
            };
            let Some(next) = cycle.stages.get(&cycle.current).map(|stage| stage.next) else {
                continue;
            };
            let Some(stage) = cycle.stages.get(&next) else {
                continue;
            };
            let Some(due) = now.checked_add(stage.duration_us) else {
                continue;
            };
            if current.is_none() && !self.has_capacity(2) {
                continue;
            }
            self.changed.insert(key, (revision, stage.facts));
            cycle.current = next;
            cycle.due = due;
            self.object_cycles.insert(key, cycle);
            advanced += 1;
        }
        advanced
    }
}
