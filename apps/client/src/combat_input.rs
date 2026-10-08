//! Combat input (CLIENT-COMBAT-INPUT-1): the commands the shell sends to the session task beyond
//! steps, and how the task runs them over `Session`.
//!
//! A click names a tile; the session task resolves it against the entities the session holds
//! (`Session::world_entities`), because only the session knows their `EntityRef`. The view keeps
//! no authority: it shows the outcome the server answered with.

use crate::input::StepDir;
use crate::spell::{SpellFeedback, cast_selected};
use oteryn_renderer::TileCoord;
use oteryn_session::{
    AttackIntentDisposition, AttackOutcome, EntityKind, EntityRef, Session, SessionError,
    SessionStream, SpellCastDisposition, WorldEntities,
};
use std::num::NonZeroU32;

/// What the shell asks of the session task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayCommand {
    Step(StepDir),
    Combat(CombatCommand),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatCommand {
    /// Attack the creature or player standing on this tile of this floor.
    AttackTile { tile: TileCoord, floor: i16 },
    /// Cast the 1-based spell-book entry, aimed at the attack target when there is one.
    Cast(NonZeroU32),
}

/// What the task reports back for one combat command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatReport {
    /// The server answered the attack-target intent.
    Attack(AttackIntentDisposition),
    /// No attackable entity on the clicked tile, or the session did not select the attack
    /// capability; nothing was sent.
    NoTarget,
    Cast(SpellCastDisposition),
}

impl CombatReport {
    /// The player-facing line, `None` for a successful attack (the target is the feedback).
    #[must_use]
    pub const fn text(self) -> Option<&'static str> {
        match self {
            Self::Attack(AttackIntentDisposition::Ok) => None,
            Self::Attack(AttackIntentDisposition::TargetNotVisible) => Some("Target not visible"),
            Self::Attack(AttackIntentDisposition::TargetNotACreature) => {
                Some("You cannot attack that")
            }
            Self::Attack(AttackIntentDisposition::ProtectionZone) => {
                Some("You may not attack in a protection zone")
            }
            Self::Attack(AttackIntentDisposition::ReentryProtected) => {
                Some("You may not attack yet")
            }
            Self::Attack(AttackIntentDisposition::Rejected) => Some("Attacking unavailable"),
            Self::NoTarget => None,
            Self::Cast(disposition) => Some(crate::spell::feedback_text(disposition)),
        }
    }
}

/// The attackable entity on `tile` of `floor`: a creature or another player, never the own actor.
#[must_use]
pub fn attackable_at(entities: &WorldEntities, tile: TileCoord, floor: i16) -> Option<EntityRef> {
    entities
        .iter()
        .find(|entity| {
            matches!(entity.kind, EntityKind::Creature | EntityKind::Player)
                && entity.entity.identity != *entities.own_identity()
                && entity.position.x == tile.x
                && entity.position.y == tile.y
                && entity.position.floor == floor
        })
        .map(|entity| entity.entity)
}

/// Runs one combat command. Every server disposition is a normal report; only a session error is
/// an `Err` (and ends the session).
pub async fn run_combat<S: SessionStream>(
    session: &mut Session<S>,
    feedback: &mut SpellFeedback,
    command: CombatCommand,
) -> Result<CombatReport, SessionError> {
    match command {
        CombatCommand::AttackTile { tile, floor } => {
            if session.combat_state().is_none() {
                return Ok(CombatReport::NoTarget);
            }
            let Some(target) = session
                .world_entities()
                .and_then(|entities| attackable_at(entities, tile, floor))
            else {
                return Ok(CombatReport::NoTarget);
            };
            let AttackOutcome { disposition, .. } = session.attack_target(Some(&target)).await?;
            Ok(CombatReport::Attack(disposition))
        }
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
    fn a_successful_attack_has_no_line_and_each_refusal_has_one() {
        assert_eq!(
            CombatReport::Attack(AttackIntentDisposition::Ok).text(),
            None
        );
        assert_eq!(CombatReport::NoTarget.text(), None);
        assert_eq!(
            CombatReport::Attack(AttackIntentDisposition::ProtectionZone).text(),
            Some("You may not attack in a protection zone")
        );
        assert_eq!(
            CombatReport::Cast(SpellCastDisposition::NotEnoughMana).text(),
            Some("Not enough mana")
        );
    }
}
