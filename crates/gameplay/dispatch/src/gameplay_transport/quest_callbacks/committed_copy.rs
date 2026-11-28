//! Reuse questl2b be6626b4: the next command observes a committed world stage.
use super::*;
use crate::durability::quest_state::CommittedQuestTransition;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// A death can advance the Character revision without changing its quest tracks.
    /// Encounter credit needs a coherent predecessor before the tomb receipt is applied.
    pub(super) async fn prepare_tomb_quest_copy(
        &self,
        fence: CurrentCharacterGameplayFence,
    ) -> bool {
        let version = {
            let Ok(sessions) = self.states.quest_sessions.lock() else {
                return false;
            };
            let Some(entry) = sessions.get(&fence.game_session_id) else {
                return false;
            };
            if entry
                .copy
                .as_ref()
                .and_then(|copy| copy.character_revision())
                == Some(fence.expected_character_revision)
            {
                return true;
            }
            entry.version
        };
        let Ok(copy) = self
            .root
            .read_character_quest_state(self.character, fence.character_id)
            .await
        else {
            return false;
        };
        if copy.character_revision() != Some(fence.expected_character_revision) {
            return false;
        }
        let Ok(mut sessions) = self.states.quest_sessions.lock() else {
            return false;
        };
        let Some(entry) = sessions.get_mut(&fence.game_session_id) else {
            return false;
        };
        if entry.version != version {
            return entry
                .copy
                .as_ref()
                .and_then(|copy| copy.character_revision())
                == Some(fence.expected_character_revision);
        }
        let Some(next) = version.checked_add(1) else {
            return false;
        };
        entry.copy = Some(copy);
        entry.version = next;
        true
    }

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
