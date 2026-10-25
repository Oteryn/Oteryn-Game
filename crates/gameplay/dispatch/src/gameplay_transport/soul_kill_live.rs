//! VITALS-SOUL-1: install a settled kill's soul hook into the killer's live actor.
//!
//! The kill settlement records the `SoulKillHook` of each committed XP award (applied
//! experience against the level before the award). The actor's own vitals cadence then opens,
//! replaces or closes its soul window (Canary `Player::onGainExperience` adding
//! `CONDITION_SOUL`). The last hook of a Character wins, as sequential kills would leave it.
use std::collections::BTreeMap;
use std::sync::Mutex;

use oteryn_gameplay_characters::soul::{AppliedKillSoulFacts, SoulKillHook, soul_kill_participant};

use super::ComposedFreshAdmission;
use crate::domain::CharacterId;
use crate::durability::character_progression::ExperienceCommitOutcome;
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::spell::soul_window::SoulWindowChange;

/// The latest unapplied soul hook of each Character.
#[derive(Default)]
pub struct SoulKills(Mutex<BTreeMap<CharacterId, SoulKillHook>>);

impl SoulKills {
    /// A `NoHook` award leaves an earlier hook in place.
    pub(super) fn mark(&self, hooks: impl IntoIterator<Item = (CharacterId, SoulKillHook)>) {
        if let Ok(mut marked) = self.0.lock() {
            for (character, hook) in hooks {
                if hook != SoulKillHook::NoHook {
                    marked.insert(character, hook);
                }
            }
        }
    }

    fn take(&self, character: CharacterId) -> Option<SoulKillHook> {
        self.0.lock().ok()?.remove(&character)
    }
}

/// The hook of a newly committed award. A replayed award already opened its window.
pub(super) fn soul_hook(outcome: &ExperienceCommitOutcome) -> Option<(CharacterId, SoulKillHook)> {
    let ExperienceCommitOutcome::Committed(award) = outcome else {
        return None;
    };
    let facts =
        AppliedKillSoulFacts::new(award.level_before, award.experience_awarded.get()).ok()?;
    Some((award.character_id, soul_kill_participant(facts).hook()))
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Applies the Character's pending hook to its live actor at the owner's clock. Promotion
    /// is the stored value read at admission; the fast cadence also needs live Premium.
    pub(super) async fn apply_soul_kills(&self, actor: ExactActorRef, session: GameSessionId) {
        let now = self.owner_now();
        let runtime = self.runtime.lock().await;
        let Ok(lease) = runtime.bound_attacker_lease(actor, session) else {
            return;
        };
        let Ok(character) = CharacterId::from_bytes(*lease.character_id().as_bytes()) else {
            return;
        };
        let mut states = self.states.spell_states.lock().await;
        // A pending cast commit installs its own successor; the hook waits for the next tick.
        if states.has_pending_spell_commit(actor, session) {
            return;
        }
        let Some(hook) = self.states.soul_kills.take(character) else {
            return;
        };
        let change = match hook {
            SoulKillHook::NoHook => return,
            SoulKillHook::CloseWindow => SoulWindowChange::Close,
            SoulKillHook::OpenWindow { seconds } => SoulWindowChange::Open {
                seconds,
                promoted: self
                    .states
                    .premium_sessions
                    .lock()
                    .is_ok_and(|sessions| sessions.promoted(session)),
            },
        };
        if let Some(state) = states.get_mut(&runtime, actor, session) {
            state.change_soul_window(change, now.get());
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    fn id(last: u8) -> CharacterId {
        let mut bytes = [0x11; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = last;
        CharacterId::from_bytes(bytes).expect("UUIDv7")
    }

    #[test]
    fn the_last_hook_wins_and_no_hook_keeps_the_earlier_one() {
        let kills = SoulKills::default();
        let open = SoulKillHook::OpenWindow { seconds: 240 };
        kills.mark([
            (id(1), open),
            (id(1), SoulKillHook::NoHook),
            (id(2), open),
            (id(2), SoulKillHook::CloseWindow),
        ]);
        assert_eq!(kills.take(id(1)), Some(open));
        assert_eq!(kills.take(id(2)), Some(SoulKillHook::CloseWindow));
        assert_eq!(kills.take(id(1)), None);
    }
}
