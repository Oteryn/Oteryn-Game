//! C18b: ordinary spell cursor casts use the existing spell command, never the rune path.
use crate::combat::CombatReport;
use crate::spell::SpellFeedback;
use oteryn_session::{Session, SessionError, SessionStream, SpellTarget, SpellTargetPosition};
use std::num::NonZeroU32;

pub(crate) async fn cast_at<S: SessionStream>(
    session: &mut Session<S>,
    feedback: &mut SpellFeedback,
    spell: NonZeroU32,
    position: SpellTargetPosition,
) -> Result<CombatReport, SessionError> {
    let outcome = session
        .cast_spell(spell, SpellTarget::Position(position), false)
        .await?;
    feedback.record(outcome.disposition);
    Ok(CombatReport::Cast(outcome.disposition))
}
