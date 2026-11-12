//! Session command routing for the current attack and spell features.
use crate::attack::{attack, attack_text};
use crate::spell::{SpellFeedback, run_cast};
pub use oteryn_session::EntityRef;
use oteryn_session::{
    AttackIntentDisposition, Session, SessionError, SessionStream, SpellCastDisposition,
    SpellTargetPosition,
};
use std::num::NonZeroU32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatCommand {
    /// Attack the visible entity a click selected.
    Attack(EntityRef),
    Stop,
    Modes(oteryn_session::FightModes),
    /// Cast the 1-based spell-book entry, aimed at the attack target when there is one.
    Cast(NonZeroU32),
    /// Cast along facing, optionally turning first and waiting for the server's acceptance.
    CastFacing {
        spell: NonZeroU32,
        direction: Option<oteryn_session::StepDirection>,
    },
    /// Cast an ordinary spell at the tile selected by the cursor.
    CastAt {
        spell: NonZeroU32,
        position: SpellTargetPosition,
    },
}

/// What the task reports back for one combat command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatReport {
    Attack(AttackIntentDisposition),
    Stop(AttackIntentDisposition),
    Modes(AttackIntentDisposition),
    Cast(SpellCastDisposition),
}

impl CombatReport {
    /// The player-facing line.
    #[must_use]
    pub const fn text(self) -> Option<&'static str> {
        match self {
            Self::Attack(disposition) => Some(attack_text(disposition)),
            Self::Stop(AttackIntentDisposition::Ok) => Some("Attack stopped"),
            Self::Modes(AttackIntentDisposition::Ok) => Some("Combat modes updated"),
            Self::Stop(disposition) | Self::Modes(disposition) => Some(attack_text(disposition)),
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
        CombatCommand::Attack(target) => attack(session, target).await,
        CombatCommand::Stop => crate::attack::stop(session).await,
        CombatCommand::Modes(modes) => crate::attack::modes(session, modes).await,
        CombatCommand::Cast(spell) => run_cast(session, feedback, spell).await,
        CombatCommand::CastFacing { spell, direction } => {
            if let Some(direction) = direction {
                if !session.is_selected(64) {
                    return Ok(CombatReport::Cast(SpellCastDisposition::Rejected));
                }
                let payload = oteryn_session::target_move::encode_target_move_intent(
                    oteryn_session::target_move::TargetMoveIntent::Turn(direction),
                )
                .map_err(|_| SessionError::SessionUnusable)?;
                let reply = session.gated_command(64, 81, &payload).await?;
                if !matches!(
                    reply.status,
                    oteryn_session::CommandStatus::Accepted
                        | oteryn_session::CommandStatus::DuplicateReplay
                ) || reply.payload != [8, 1]
                {
                    return Ok(CombatReport::Cast(SpellCastDisposition::Rejected));
                }
            }
            let outcome = crate::spell::cast_selected(session, feedback, spell, false).await?;
            Ok(CombatReport::Cast(outcome.disposition))
        }
        CombatCommand::CastAt { spell, position } => {
            crate::cursor_spell::cast_at(session, feedback, spell, position).await
        }
    }
}
