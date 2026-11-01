//! PAR-M10: Channel-local timed decay of a base map entry that a step transformed, the
//! `duration`/`decayTo` pair of Crystal `items.xml` (snow footprints: 6594 and 6595..=6608 decay
//! after 10 s, `items.xml:18274-18363`). A due entry reverts through the same callback fact
//! overlay that transformed it, and only while its object revision is still the one that
//! transform produced: a later change of the entry supersedes its decay. Restart restores the
//! base map, like every other Channel-local transform.
use super::*;

/// One pending decay per entry, bounded like the callback overlay it reverts through.
const ENTRY_DECAYS_MAX: usize = super::super::overlay::OVERLAY_BUDGET_BYTES / 512;

#[derive(Debug, Default)]
pub(super) struct EntryDecays {
    /// `(due owner time in µs, placement key)` to `(object revision, decayed appearance)`.
    due: BTreeMap<(u64, u64), (u64, u16)>,
    /// Placement key to its due time, so an entry holds at most one decay.
    by_key: BTreeMap<u64, u64>,
}

impl BundleFacts {
    /// Schedules the base entry `key`, now at object `revision`, to become `appearance` at owner
    /// time `due_us`, replacing any decay it already had. Refused when the schedule is full.
    pub fn schedule_entry_decay(
        &self,
        key: u64,
        revision: u64,
        due_us: u64,
        appearance: u16,
    ) -> bool {
        let Ok(mut decays) = self.entry_decays.lock() else {
            return false;
        };
        if let Some(previous) = decays.by_key.remove(&key) {
            decays.due.remove(&(previous, key));
        }
        if decays.due.len() >= ENTRY_DECAYS_MAX {
            return false;
        }
        decays.due.insert((due_us, key), (revision, appearance));
        decays.by_key.insert(key, due_us);
        true
    }

    /// Applies every decay due at owner time `now_us`; true when an entry changed.
    pub fn expire_entry_decays(&self, now_us: u64) -> bool {
        let due = {
            let Ok(mut decays) = self.entry_decays.lock() else {
                return false;
            };
            let later = match now_us.checked_add(1) {
                Some(after) => decays.due.split_off(&(after, 0)),
                None => BTreeMap::new(),
            };
            let due = std::mem::replace(&mut decays.due, later);
            for (_, key) in due.keys() {
                decays.by_key.remove(key);
            }
            due
        };
        let mut changed = false;
        for ((_, key), (revision, appearance)) in due {
            if self.object_revision(key) != revision {
                continue;
            }
            let Some(replacement) = self.quest_callback_replacement(key, appearance) else {
                continue;
            };
            changed |= self.apply_quest_callback(key, revision, replacement, &[], &[], false);
        }
        changed
    }

    /// The number of pending entry decays.
    pub fn pending_entry_decays(&self) -> usize {
        self.entry_decays
            .lock()
            .map_or(0, |decays| decays.due.len())
    }
}
