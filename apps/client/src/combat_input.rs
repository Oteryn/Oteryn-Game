//! Combat input (CLIENT-COMBAT-INPUT-1): the commands the shell sends to the session task beyond
//! steps, and how the task runs them over `Session`.
//!
//! The view keeps no authority: it shows the outcome the server answered with. A click on a
//! visible entity attacks it (CLIENT-ENTITY-VIEW-1).

use crate::input::StepDir;
use crate::spell::{SpellFeedback, cast_selected};
use oteryn_session::{
    AttackIntentDisposition, EntityRef, Session, SessionError, SessionStream, SpellCastDisposition,
};
use std::num::NonZeroU32;
use std::time::Duration;

/// How long a command waits for the pushed domain-10 delta its answer announced: a few short
/// reads, so a lost delta never stalls the task.
const SETTLE_POLLS: u32 = 10;
const SETTLE_SLICE: Duration = Duration::from_millis(20);
/// A read that applies only what has already arrived.
const PUMP_SLICE: Duration = Duration::from_millis(1);

/// What the shell asks of the session task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayCommand {
    Step(StepDir),
    Combat(CombatCommand),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatCommand {
    /// Attack the visible entity a click selected.
    Attack(EntityRef),
    /// Cast the 1-based spell-book entry, aimed at the attack target when there is one.
    Cast(NonZeroU32),
}

/// What the task reports back for one combat command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatReport {
    Attack(AttackIntentDisposition),
    Cast(SpellCastDisposition),
}

impl CombatReport {
    /// The player-facing line.
    #[must_use]
    pub const fn text(self) -> Option<&'static str> {
        match self {
            Self::Attack(disposition) => Some(attack_text(disposition)),
            Self::Cast(disposition) => Some(crate::spell::feedback_text(disposition)),
        }
    }
}

const fn attack_text(disposition: AttackIntentDisposition) -> &'static str {
    match disposition {
        AttackIntentDisposition::Ok => "Attacking",
        AttackIntentDisposition::TargetNotVisible => "Target not visible",
        AttackIntentDisposition::TargetNotACreature => "You cannot attack that",
        AttackIntentDisposition::ProtectionZone => "Not in a protection zone",
        AttackIntentDisposition::ReentryProtected => "You cannot attack yet",
        AttackIntentDisposition::Rejected => "Attack refused",
    }
}

/// Reads until the session holds `wanted` as its attack target, or the polls run out. The target
/// arrives as a pushed delta after the command's result.
async fn settle_target<S: SessionStream>(
    session: &mut Session<S>,
    wanted: EntityRef,
) -> Result<(), SessionError> {
    for _ in 0..SETTLE_POLLS {
        let held = session.combat_state().and_then(|state| state.target);
        if held == Some(wanted) {
            return Ok(());
        }
        session.service_liveness(SETTLE_SLICE).await?;
    }
    Ok(())
}

/// Runs one combat command. Every server disposition is a normal report; only a session error is
/// an `Err` (and ends the session).
pub async fn run_combat<S: SessionStream>(
    session: &mut Session<S>,
    feedback: &mut SpellFeedback,
    command: CombatCommand,
) -> Result<CombatReport, SessionError> {
    match command {
        CombatCommand::Attack(target) => {
            let outcome = session.attack_target(Some(&target)).await?;
            if outcome.disposition == AttackIntentDisposition::Ok {
                settle_target(session, target).await?;
            }
            Ok(CombatReport::Attack(outcome.disposition))
        }
        CombatCommand::Cast(spell) => {
            // Apply any domain-10 delta already on the wire before reading the target.
            session.service_liveness(PUMP_SLICE).await?;
            // The server aims at its own attack target; the client only says whether to.
            let aimed = session
                .combat_state()
                .is_some_and(|state| state.target.is_some());
            let outcome = cast_selected(session, feedback, spell, aimed).await?;
            Ok(CombatReport::Cast(outcome.disposition))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cast_report_has_its_refusal_line() {
        assert_eq!(
            CombatReport::Cast(SpellCastDisposition::NotEnoughMana).text(),
            Some("Not enough mana")
        );
    }
}
