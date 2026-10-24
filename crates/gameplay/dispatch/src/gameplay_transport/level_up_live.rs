//! LEVELUP-LIVE-1: install a committed level increase into the live actor.
//!
//! The kill settlement marks each Character whose XP award crossed a level boundary. The
//! actor's own vitals cadence then reads the committed cast facts and raises the live maxima,
//! current pools and base speed (Canary `Player::addExperience`). Without it the actor keeps
//! the admission maxima until it is admitted again.
use std::collections::BTreeSet;
use std::sync::Mutex;

use super::ComposedFreshAdmission;
use super::spell_character_facts::{CastFactsLoad, load_character_cast_facts};
use crate::domain::CharacterId;
use crate::durability::character_progression::ExperienceCommitOutcome;
use crate::foundation::{ExactActorRef, GameSessionId};

/// The Characters with a committed level increase not yet installed live.
#[derive(Default)]
pub struct LevelRefresh(Mutex<BTreeSet<CharacterId>>);

impl LevelRefresh {
    pub(super) fn mark(&self, characters: impl IntoIterator<Item = CharacterId>) {
        if let Ok(mut marked) = self.0.lock() {
            marked.extend(characters);
        }
    }

    fn is_marked(&self, character: CharacterId) -> bool {
        self.0
            .lock()
            .is_ok_and(|marked| marked.contains(&character))
    }

    fn unmark(&self, character: CharacterId) {
        if let Ok(mut marked) = self.0.lock() {
            marked.remove(&character);
        }
    }
}

/// The Character of an award that crossed a level boundary, replayed awards included.
pub(super) fn leveled_character(outcome: &ExperienceCommitOutcome) -> Option<CharacterId> {
    let (ExperienceCommitOutcome::Committed(award)
    | ExperienceCommitOutcome::AlreadyCommitted(award)) = outcome;
    (award.level_after > award.level_before).then_some(award.character_id)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// A failed read keeps the mark for the next tick. A pending commit or a dead actor waits
    /// too. A stale mark (already installed, or refused) is dropped.
    pub(super) async fn refresh_level_up_vitals(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) {
        let character = {
            let runtime = self.runtime.lock().await;
            let Ok(lease) = runtime.bound_attacker_lease(actor, session) else {
                return;
            };
            let Ok(character) = CharacterId::from_bytes(*lease.character_id().as_bytes()) else {
                return;
            };
            character
        };
        if !self.states.level_refresh.is_marked(character) {
            return;
        }
        let CastFactsLoad::Ready { facts, .. } =
            load_character_cast_facts(self.root, self.character, character).await
        else {
            return;
        };
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let Ok(lease) = runtime.bound_attacker_lease(actor, session) else {
            return;
        };
        if lease.character_id().as_bytes() != character.as_bytes()
            || states.has_pending_spell_commit(actor, session)
        {
            return;
        }
        let Some(state) = states.get(&runtime, actor, session) else {
            return;
        };
        if state.health == 0 {
            return;
        }
        let next = (facts.level > state.facts.level)
            .then(|| state.after_level_up(facts))
            .flatten();
        if let Some(next) = next
            && let Some(state) = states.get_mut(&runtime, actor, session)
        {
            *state = next;
        }
        self.states.level_refresh.unmark(character);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn marks_dedupe_and_unmark_exactly_one_character() {
        let refresh = LevelRefresh::default();
        let id = |last| {
            let mut bytes = [0x11; 16];
            bytes[6] = 0x70;
            bytes[8] = 0x80;
            bytes[15] = last;
            CharacterId::from_bytes(bytes).expect("UUIDv7")
        };
        let (a, b) = (id(1), id(2));
        refresh.mark([a, a, b]);
        refresh.unmark(a);
        assert!(!refresh.is_marked(a));
        assert!(refresh.is_marked(b));
    }
}
