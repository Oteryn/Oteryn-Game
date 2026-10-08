//! Combat input (CLIENT-COMBAT-INPUT-1): the commands the shell sends to the session task beyond
//! steps, and how the task runs them over `Session`.
//!
//! The view keeps no authority: it shows the outcome the server answered with. Click-to-attack
//! returns once the view carries the visible entities (CLIENT-ENTITY-VIEW-1).

use crate::input::StepDir;
use crate::spell::{SpellFeedback, cast_selected};
use oteryn_session::{Session, SessionError, SessionStream, SpellCastDisposition};
use std::num::NonZeroU32;

/// What the shell asks of the session task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayCommand {
    Step(StepDir),
    Combat(CombatCommand),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatCommand {
    /// Cast the 1-based spell-book entry, aimed at the attack target when there is one.
    Cast(NonZeroU32),
}

/// What the task reports back for one combat command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatReport {
    Cast(SpellCastDisposition),
}

impl CombatReport {
    /// The player-facing line.
    #[must_use]
    pub const fn text(self) -> Option<&'static str> {
        match self {
            Self::Cast(disposition) => Some(crate::spell::feedback_text(disposition)),
        }
    }
}

/// Runs one combat command. Every server disposition is a normal report; only a session error is
/// an `Err` (and ends the session).
pub async fn run_combat<S: SessionStream>(
    session: &mut Session<S>,
    feedback: &mut SpellFeedback,
    command: CombatCommand,
) -> Result<CombatReport, SessionError> {
    match command {
        CombatCommand::Cast(spell) => {
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
