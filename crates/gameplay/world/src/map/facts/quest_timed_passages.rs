//! M07: bounded remote mutations and exact-revision restoration share one map write.
use super::*;

/// A source-read remote object; `None` hides it until the original deadline.
#[derive(Clone, Copy, Debug)]
pub struct QuestTimedPassage {
    pub key: u64,
    pub expected_revision: u64,
    pub before: EntryFacts,
    pub after: Option<EntryFacts>,
    pub passable: bool,
}

impl CallbackStates {
    pub(super) fn project_timed_passages(
        &mut self,
        owner: u64,
        due: u64,
        entries: &[QuestTimedPassage],
        source_reset: Option<(u64, EntryFacts)>,
    ) -> bool {
        let keys: BTreeSet<_> = entries.iter().map(|entry| entry.key).collect();
        if entries.is_empty()
            || entries.len() > 8
            || keys.len() != entries.len()
            || keys.contains(&owner)
            || entries.iter().any(|entry| {
                entry.expected_revision == u64::MAX
                    || entry.before.pickupable
                    || entry.after.is_some_and(|after| after.pickupable)
                    || self.hidden.contains(&entry.key)
                    || self
                        .changed
                        .get(&entry.key)
                        .map_or(0, |(revision, _)| *revision)
                        != entry.expected_revision
                    || self
                        .changed
                        .get(&entry.key)
                        .is_some_and(|(_, before)| *before != entry.before)
            })
        {
            return false;
        }
        if let Some((source_due, restore)) = source_reset {
            let Some(revision) = self
                .changed
                .get(&owner)
                .map_or(0, |(r, _)| *r)
                .checked_add(1)
            else {
                return false;
            };
            self.movement_decays.insert(
                (source_due, owner),
                MovementDecay {
                    revision,
                    restore,
                    overlay: None,
                },
            );
        }
        for entry in entries {
            let revision = entry.expected_revision + 1;
            self.movement_decays.insert(
                (due, entry.key),
                MovementDecay {
                    revision,
                    restore: entry.before,
                    overlay: Some((false, self.passable.get(&entry.key).copied())),
                },
            );
            self.changed
                .insert(entry.key, (revision, entry.after.unwrap_or(entry.before)));
            if entry.after.is_some() {
                self.passable.insert(entry.key, entry.passable);
            } else {
                self.hidden.insert(entry.key);
            }
        }
        true
    }
}
