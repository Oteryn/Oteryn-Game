//! The Earth scene restores only entries changed by its own activation.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
struct BaseState {
    change: Option<(u64, EntryFacts)>,
    hidden: bool,
    passable: Option<bool>,
}

#[derive(Clone, Debug)]
pub(super) struct SceneDecay {
    entries: BTreeMap<u64, (BaseState, BaseState, EntryFacts)>,
    added: Vec<(TilePos, EntryId)>,
    clear_solid: BTreeSet<TilePos>,
    portal: u64,
}

impl SceneDecay {
    pub(super) fn footprint(&self) -> usize {
        1 + self.entries.len() + self.added.len() + self.clear_solid.len()
    }
}

impl CallbackStates {
    fn scene_base_state(&self, key: u64) -> BaseState {
        BaseState {
            change: self.changed.get(&key).copied(),
            hidden: self.hidden.contains(&key),
            passable: self.passable.get(&key).copied(),
        }
    }

    pub(super) fn project_timed_scene(
        &mut self,
        owner: u64,
        due: u64,
        edits: &[QuestSceneMutation],
        originals: &BTreeMap<u64, EntryFacts>,
        (source, destination, portal): (TilePos, TilePos, EntryFacts),
    ) -> bool {
        if edits.is_empty()
            || edits.len() > 512
            || portal.pickupable
            || self.portals.contains_key(&source)
            || self.scene_decays.keys().any(|(_, active)| *active == owner)
        {
            return false;
        }
        let mut base = BTreeMap::new();
        let mut creates = Vec::new();
        let mut clear_solid = BTreeSet::new();
        for edit in edits {
            match *edit {
                QuestSceneMutation::Remove { key, .. }
                | QuestSceneMutation::Replace { key, .. }
                | QuestSceneMutation::Restore { key, .. } => {
                    let Some(original) = originals.get(&key) else {
                        return false;
                    };
                    let old = self.scene_base_state(key);
                    if old
                        .change
                        .is_some_and(|(revision, _)| revision >= u64::MAX - 1)
                        || base.insert(key, (old, *original)).is_some()
                    {
                        return false;
                    }
                }
                QuestSceneMutation::Create {
                    position, passable, ..
                } => {
                    creates.push(position);
                    if !passable && !self.solid_items.contains(&position) {
                        clear_solid.insert(position);
                    }
                }
            }
        }
        let start = self.movement_sequence;
        if !project_scene(self, owner, edits, false, 512) {
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
            .entry(source)
            .or_default()
            .push(view::ComposedEntry {
                source: view::EntrySource::Added(entry),
                facts: portal,
            });
        self.portals.insert(
            source,
            DynamicPortal {
                owner: entry.get(),
                destination,
                entry: Some(entry),
                owner_revision: None,
                expires: Some(due),
            },
        );
        let entries = base
            .into_iter()
            .map(|(key, (old, original))| (key, (old, self.scene_base_state(key), original)))
            .collect();
        let added = creates
            .into_iter()
            .zip((start + 1)..sequence)
            .filter_map(|(position, sequence)| EntryId::callback(sequence).map(|id| (position, id)))
            .collect();
        self.scene_decays.insert(
            (due, owner),
            SceneDecay {
                entries,
                added,
                clear_solid,
                portal: entry.get(),
            },
        );
        true
    }

    pub(super) fn expire_timed_scenes(&mut self, now: u64) -> usize {
        let due: Vec<_> = self
            .scene_decays
            .range(..=(now, u64::MAX))
            .map(|(key, _)| *key)
            .collect();
        let mut changed = 0;
        for timer in due {
            let Some(scene) = self.scene_decays.remove(&timer) else {
                continue;
            };
            self.remove_portals(scene.portal);
            for (key, (old, active, original)) in scene.entries {
                if self.scene_base_state(key) != active {
                    continue;
                }
                let revision = active.change.map_or(0, |(revision, _)| revision);
                self.changed.insert(key, (revision + 1, original));
                if old.hidden {
                    self.hidden.insert(key);
                } else {
                    self.hidden.remove(&key);
                }
                if let Some(passable) = old.passable {
                    self.passable.insert(key, passable);
                } else {
                    self.passable.remove(&key);
                }
            }
            for (position, id) in scene.added {
                if let Some(entries) = self.movement_items.get_mut(&position) {
                    entries.retain(|entry| entry.source != view::EntrySource::Added(id));
                    if entries.is_empty() {
                        self.movement_items.remove(&position);
                    }
                }
            }
            for position in scene.clear_solid {
                self.solid_items.remove(&position);
            }
            changed += 1;
        }
        changed
    }
}
