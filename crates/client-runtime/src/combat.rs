//! Session command routing for the current attack and spell features.
use crate::attack::{attack, attack_text};
use crate::spell::{SpellFeedback, run_cast};
use oteryn_session::{
    AttackIntentDisposition, Session, SessionError, SessionStream, SpellCastDisposition,
};
pub use oteryn_session::{EntityRef, SpellTargetPosition, StepDirection};
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
        direction: Option<StepDirection>,
    },
    /// Cast towards the tile under the cursor without a rune: with capability 480 the cursor
    /// tile goes on the wire; without it the cast turns `direction` first, as `CastFacing`.
    CastCursor {
        spell: NonZeroU32,
        position: SpellTargetPosition,
        direction: Option<StepDirection>,
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
            cast_facing(session, feedback, spell, direction).await
        }
        CombatCommand::CastCursor {
            spell,
            position,
            direction,
        } => {
            let aimed = crate::spell::cast_aimed(
                session,
                feedback,
                spell,
                oteryn_session::AimedTarget::Cursor(position),
            )
            .await?;
            match aimed {
                // The server refuses CURSOR for a spell that is not a wave or beam.
                Some(outcome) if outcome.disposition != SpellCastDisposition::Rejected => {
                    Ok(CombatReport::Cast(outcome.disposition))
                }
                _ => cast_facing(session, feedback, spell, direction).await,
            }
        }
        CombatCommand::CastAt { spell, position } => {
            crate::cursor_spell::cast_at(session, feedback, spell, position).await
        }
    }
}

/// Casts along facing, optionally turning first and waiting for the server's acceptance. With
/// capability 480 and no turn the cast carries `FACING`.
async fn cast_facing<S: SessionStream>(
    session: &mut Session<S>,
    feedback: &mut SpellFeedback,
    spell: NonZeroU32,
    direction: Option<StepDirection>,
) -> Result<CombatReport, SessionError> {
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
    } else if let Some(outcome) = crate::spell::cast_aimed(
        session,
        feedback,
        spell,
        oteryn_session::AimedTarget::Facing,
    )
    .await?
    {
        return Ok(CombatReport::Cast(outcome.disposition));
    }
    let outcome = crate::spell::cast_selected(session, feedback, spell, false).await?;
    Ok(CombatReport::Cast(outcome.disposition))
}
