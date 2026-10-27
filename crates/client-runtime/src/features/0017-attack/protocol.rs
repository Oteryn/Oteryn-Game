use crate::combat::CombatReport;
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

/// Reads until the session holds `wanted` as its attack target. The target arrives as a pushed
/// delta after the command's result; if it does not arrive by the deadline the session fails.
async fn settle_target<S: SessionStream>(
    session: &mut Session<S>,
    wanted: EntityRef,
) -> Result<(), SessionError> {
    let until = tokio::time::Instant::now() + SETTLE_DEADLINE;
    loop {
        let held = session.combat_state().and_then(|state| state.target);
        if held == Some(wanted) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= until {
            return Err(SessionError::Timeout("attack target state"));
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
        settle_target(session, target).await?;
    }
    Ok(CombatReport::Attack(outcome.disposition))
}

#[cfg(test)]
#[path = "tests/protocol.rs"]
mod tests;
