//! Soul R5 pure policy: cap, regeneration interval and the typed kill-receipt participant.
//! D1251: cap reads the current PremiumGate; fast regeneration also needs stored promotion.
//! The kill writer must create the participant from its actual applied XP inside its own
//! fenced transaction. This pure proposal neither commits a receipt nor proves authority.

use crate::domain::premium::{PremiumGate, soul_maximum};

pub const SOUL_KILL_WINDOW_SECONDS: u32 = 240;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoulSchedule {
    pub maximum: u32,
    pub interval_seconds: u32,
}

/// Resolve afresh at use; a retained schedule is not current Premium evidence.
#[must_use]
pub const fn soul_schedule(stored_promotion: bool, gate: PremiumGate) -> SoulSchedule {
    SoulSchedule {
        maximum: soul_maximum(gate.premium_current()),
        interval_seconds: if stored_promotion && gate.premium_current() {
            15
        } else {
            120
        },
    }
}

/// Facts from one applied monster-kill XP calculation, before any hook is staged.
/// This is not proof of a kill, a Character binding, a clock or a current fence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppliedKillSoulFacts {
    level: u32,
    applied_experience: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidKillSoulFacts;

impl AppliedKillSoulFacts {
    /// `level` is the level used by the kill calculation, not a post-level-up projection.
    pub const fn new(level: u32, applied_experience: i64) -> Result<Self, InvalidKillSoulFacts> {
        if level == 0 || applied_experience < 0 {
            return Err(InvalidKillSoulFacts);
        }
        Ok(Self {
            level,
            applied_experience,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoulKillHook {
    /// Zero applied XP must not touch an existing window or schedule a hook.
    NoHook,
    /// Positive applied XP below the level closes any existing window.
    CloseWindow,
    /// Replace the remaining window with this duration; never add to the old window.
    OpenWindow { seconds: u32 },
}

/// Typed semantic participant for the owning kill receipt. The final writer must bind this
/// together with its real Character, occurrence and revision in the same transaction.
/// Replaying a historical receipt must not reapply this hook or start a new window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoulKillReceiptParticipant {
    facts: AppliedKillSoulFacts,
    hook: SoulKillHook,
}

impl SoulKillReceiptParticipant {
    #[must_use]
    pub const fn facts(self) -> AppliedKillSoulFacts {
        self.facts
    }

    #[must_use]
    pub const fn hook(self) -> SoulKillHook {
        self.hook
    }
}

#[must_use]
pub const fn soul_kill_participant(facts: AppliedKillSoulFacts) -> SoulKillReceiptParticipant {
    let hook = if facts.applied_experience == 0 {
        SoulKillHook::NoHook
    } else if facts.applied_experience < facts.level as i64 {
        SoulKillHook::CloseWindow
    } else {
        SoulKillHook::OpenWindow {
            seconds: SOUL_KILL_WINDOW_SECONDS,
        }
    };
    SoulKillReceiptParticipant { facts, hook }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::premium::PremiumStatus;

    // The public status owner's gate; integration test crates cannot reach `from_fence_read`.
    fn gate(premium: bool) -> PremiumGate {
        if premium {
            PremiumStatus::Current.gate()
        } else {
            PremiumStatus::NotCurrent.gate()
        }
    }

    #[test]
    fn premium_alone_controls_cap_but_promotion_is_also_required_for_fast_regeneration() {
        for (promoted, premium, maximum, interval_seconds) in [
            (false, false, 100, 120),
            (true, false, 100, 120),
            (false, true, 200, 120),
            (true, true, 200, 15),
        ] {
            assert_eq!(
                soul_schedule(promoted, gate(premium)),
                SoulSchedule {
                    maximum,
                    interval_seconds,
                }
            );
        }
        assert_eq!(soul_schedule(true, PremiumGate::default()).maximum, 100);
    }

    #[test]
    fn premium_expiry_reverts_both_benefits_on_the_next_resolution() {
        let open = soul_schedule(true, gate(true));
        let expired = soul_schedule(true, PremiumGate::CLOSED);
        assert_eq!((open.maximum, open.interval_seconds), (200, 15));
        assert_eq!((expired.maximum, expired.interval_seconds), (100, 120));
    }

    #[test]
    fn zero_xp_is_no_hook_while_a_positive_sublevel_award_closes_the_window()
    -> Result<(), InvalidKillSoulFacts> {
        for (level, xp, expected) in [
            (1, 0, SoulKillHook::NoHook),
            (50, 0, SoulKillHook::NoHook),
            (50, 1, SoulKillHook::CloseWindow),
            (50, 49, SoulKillHook::CloseWindow),
        ] {
            let facts = AppliedKillSoulFacts::new(level, xp)?;
            assert_eq!(soul_kill_participant(facts).hook(), expected);
        }
        Ok(())
    }

    #[test]
    fn applied_xp_threshold_and_large_values_open_exactly_four_minutes()
    -> Result<(), InvalidKillSoulFacts> {
        for (level, xp) in [(1, 1), (50, 50), (50, 51), (u32::MAX, i64::MAX)] {
            let facts = AppliedKillSoulFacts::new(level, xp)?;
            let participant = soul_kill_participant(facts);
            assert_eq!(participant.facts(), facts);
            assert_eq!(
                participant.hook(),
                SoulKillHook::OpenWindow { seconds: 240 }
            );
        }
        Ok(())
    }

    #[test]
    fn invalid_kill_facts_cannot_become_receipt_participants() {
        assert_eq!(AppliedKillSoulFacts::new(0, 1), Err(InvalidKillSoulFacts));
        assert_eq!(AppliedKillSoulFacts::new(1, -1), Err(InvalidKillSoulFacts));
        assert_eq!(
            AppliedKillSoulFacts::new(1, i64::MIN),
            Err(InvalidKillSoulFacts)
        );
    }
}
