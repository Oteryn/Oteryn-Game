//! The action button's target option as the cast command (capability 480 `SPELL_AIM_MODE_V1`).
use super::spell_action_bar::TargetOption;
use oteryn_client_runtime::combat::{CombatCommand, SpellTargetPosition, StepDirection};
use std::num::NonZeroU32;

/// The cursor tile under the pointer and, for a session without 480, the cardinal turn towards it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CursorAim {
    pub position: SpellTargetPosition,
    pub direction: Option<StepDirection>,
}

/// Command for casting `spell` with `option`: TARGET and FACING need no tile (`Cast` and
/// `CastFacing` carry `aim_mode` TARGET and FACING once 480 is negotiated), CURSOR needs the
/// cursor tile and is `None` without one.
#[must_use]
pub fn cast_command(
    option: TargetOption,
    spell: NonZeroU32,
    cursor: Option<CursorAim>,
) -> Option<CombatCommand> {
    match option {
        TargetOption::TowardsTarget => Some(CombatCommand::Cast(spell)),
        TargetOption::FacingDirection => Some(CombatCommand::CastFacing {
            spell,
            direction: None,
        }),
        TargetOption::TowardsCursor => cursor.map(|cursor| CombatCommand::CastCursor {
            spell,
            position: cursor.position,
            direction: cursor.direction,
        }),
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
