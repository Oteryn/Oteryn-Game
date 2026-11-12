use super::state::SpellFeedback;
use crate::combat::CombatReport;
use oteryn_session::{
    AimedTarget, CAPABILITY_SPELL_AIM_MODE_V1, CastOutcome, Session, SessionError, SessionStream,
    SpellCastDisposition, SpellTarget,
};
use std::num::NonZeroU32;
use std::time::Duration;

/// A read that applies only what has already arrived.
const PUMP_SLICE: Duration = Duration::from_millis(1);

/// Player-facing line for one cast result.
#[must_use]
pub const fn feedback_text(disposition: SpellCastDisposition) -> &'static str {
    match disposition {
        SpellCastDisposition::Cast => "Spell cast",
        SpellCastDisposition::CoolingDown => "Spell is cooling down",
        SpellCastDisposition::LevelTooLow => "Level too low",
        SpellCastDisposition::MagicLevelTooLow => "Magic level too low",
        SpellCastDisposition::NotEnoughMana => "Not enough mana",
        SpellCastDisposition::NotEnoughSoul => "Not enough soul",
        SpellCastDisposition::NotAvailable => "Spell not available",
        SpellCastDisposition::TargetRequired => "Select a target first",
        SpellCastDisposition::TargetIllegal => "Illegal target",
        SpellCastDisposition::Rejected => "Spells unavailable",
    }
}

/// Casts `spell` at the attack target (`aimed`) or without one, and records the disposition.
/// Vitals after a `Cast` are read from `Session::actor_vitals`.
pub async fn cast_selected<S: SessionStream>(
    session: &mut Session<S>,
    feedback: &mut SpellFeedback,
    spell: NonZeroU32,
    aimed: bool,
) -> Result<CastOutcome, SessionError> {
    let (intent, aim) = if aimed {
        (SpellTarget::AttackTarget, true)
    } else {
        (SpellTarget::None, false)
    };
    let outcome = session.cast_spell(spell, intent, aim).await?;
    feedback.record(outcome.disposition);
    Ok(outcome)
}

/// Casts `spell` with the action button's target option when the session negotiated capability
/// 480, and records the disposition; `None` when 480 is not selected so the caller keeps the old
/// encoding.
pub async fn cast_aimed<S: SessionStream>(
    session: &mut Session<S>,
    feedback: &mut SpellFeedback,
    spell: NonZeroU32,
    aim: AimedTarget,
) -> Result<Option<CastOutcome>, SessionError> {
    if !session.is_selected(CAPABILITY_SPELL_AIM_MODE_V1) {
        return Ok(None);
    }
    let outcome = session.cast_spell_aimed(spell, aim).await?;
    feedback.record(outcome.disposition);
    Ok(Some(outcome))
}

/// Applies queued combat state before aiming the selected spell.
pub(crate) async fn run_cast<S: SessionStream>(
    session: &mut Session<S>,
    feedback: &mut SpellFeedback,
    spell: NonZeroU32,
) -> Result<CombatReport, SessionError> {
    // Apply any domain-10 delta already on the wire before reading the target.
    session.service_liveness(PUMP_SLICE).await?;
    // Keep ATTACK_TARGET for normal casts, including target-or-direction strikes. The
    // server also resolves wave/beam direction from this intent; NONE+TARGET probing
    // can succeed as a facing strike and lose the held target without a refusal.
    // The server aims at its own attack target; the client only says whether to.
    let aimed = session
        .combat_state()
        .is_some_and(|state| state.target.is_some());
    let outcome = cast_selected(session, feedback, spell, aimed).await?;
    Ok(CombatReport::Cast(outcome.disposition))
}

#[cfg(test)]
#[path = "tests/cast.rs"]
mod tests;
