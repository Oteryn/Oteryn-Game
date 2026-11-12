use crate::combat::CombatReport;
pub use oteryn_session::{
    ActorCombatState, ActorPosition, ChaseMode, EntityDetail, EntityKind, FightMode, FightModes,
    StepDirection, WorldSpatialEntity,
};
use oteryn_session::{AttackIntentDisposition, EntityRef, Session, SessionError, SessionStream};
use std::time::Duration;

/// How long a command waits for its announced domain-10 target delta.
const SETTLE_DEADLINE: Duration = Duration::from_secs(5);
const SETTLE_SLICE: Duration = Duration::from_millis(20);

pub(crate) const fn attack_text(disposition: AttackIntentDisposition) -> &'static str {
    match disposition {
        AttackIntentDisposition::Ok => "Attacking",
        AttackIntentDisposition::TargetNotVisible => "Target not visible",
        AttackIntentDisposition::TargetNotACreature => "You cannot attack that",
        AttackIntentDisposition::ProtectionZone => "You cannot attack in a protection zone",
        AttackIntentDisposition::ReentryProtected => "You cannot attack yet",
        AttackIntentDisposition::Rejected => "Attack refused",
    }
}

/// Reads until the session holds the requested combat state. The target arrives as a pushed
/// delta after the command's result; if it does not arrive by the deadline the session fails.
async fn settle_state<S: SessionStream>(
    session: &mut Session<S>,
    wanted: impl Fn(&ActorCombatState) -> bool,
) -> Result<(), SessionError> {
    let until = tokio::time::Instant::now() + SETTLE_DEADLINE;
    loop {
        if session.combat_state().is_some_and(&wanted) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= until {
            return Err(SessionError::Timeout("combat state"));
        }
        session.service_liveness(SETTLE_SLICE).await?;
    }
}

/// Sends an attack and waits for the announced target before a following cast can read it.
pub(crate) async fn attack<S: SessionStream>(
    session: &mut Session<S>,
    target: EntityRef,
) -> Result<CombatReport, SessionError> {
    let outcome = session.attack_target(Some(&target)).await?;
    if outcome.disposition == AttackIntentDisposition::Ok {
        settle_state(session, |state| state.target == Some(target)).await?;
    }
    Ok(CombatReport::Attack(outcome.disposition))
}

#[cfg(test)]
#[path = "tests/protocol.rs"]
mod tests;

pub(crate) async fn stop<S: SessionStream>(
    session: &mut Session<S>,
) -> Result<CombatReport, SessionError> {
    let disposition = session.attack_target(None).await?.disposition;
    if disposition == AttackIntentDisposition::Ok {
        settle_state(session, |state| state.target.is_none()).await?;
    }
    Ok(CombatReport::Stop(disposition))
}

pub(crate) async fn modes<S: SessionStream>(
    session: &mut Session<S>,
    modes: oteryn_session::FightModes,
) -> Result<CombatReport, SessionError> {
    let disposition = session.set_fight_modes(&modes).await?.disposition;
    if disposition == AttackIntentDisposition::Ok {
        settle_state(session, |state| state.modes == modes).await?;
    }
    Ok(CombatReport::Modes(disposition))
}
