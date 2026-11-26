//! Reuse questl2b be6626b4: the next command observes a committed world stage.
use super::*;
use crate::durability::quest_state::CommittedQuestTransition;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn apply_committed_quest_copy(
        &self,
        session: GameSessionId,
        fresh: bool,
        committed: &CommittedQuestTransition,
    ) {
        if let Ok(mut sessions) = self.states.quest_sessions.lock()
            && let Some(entry) = sessions.get_mut(&session)
        {
            entry.copy = entry.copy.take().and_then(|mut copy| {
                // Recovered receipts cannot replace a newer session snapshot.
                if !fresh
                    && copy.character_revision() != Some(committed.original_character_revision)
                {
                    return Some(copy);
                }
                let next = entry.version.checked_add(1)?;
                if !copy.apply(committed) {
                    return None;
                }
                entry.version = next;
                Some(copy)
            });
        }
    }
}
