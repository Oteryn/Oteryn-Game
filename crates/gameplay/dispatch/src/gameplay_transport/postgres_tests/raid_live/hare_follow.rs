//! Real paced STEP pursuit; the source Hare keeps its own preferred distance and AI.
use super::*;
use crate::gameplay_transport::world_spatial::{StepDirection, StepDisposition};
pub(super) async fn step(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    hare: crate::foundation::ExactActorRef,
    next_due: &mut u64,
) -> TestResult<bool> {
    let now = owner.owner_now().get();
    if now < *next_due {
        return Ok(false);
    }
    let (before, target) = {
        let runtime = owner.runtime.lock().await;
        (
            runtime.read_actor_position(actor).map_err(debug)?,
            runtime.read_actor_position(hare).map_err(debug)?,
        )
    };
    let (from, to) = (before.position(), target.position());
    if from.floor != to.floor {
        return Err("Hare chase floor changed unexpectedly".into());
    }
    let dx = i64::from(to.x) - i64::from(from.x);
    let dy = i64::from(to.y) - i64::from(from.y);
    if dx.abs().max(dy.abs()) <= 1 {
        return Ok(false);
    }
    let direction = if dx.abs() >= dy.abs() {
        if dx < 0 {
            StepDirection::West
        } else {
            StepDirection::East
        }
    } else if dy < 0 {
        StepDirection::North
    } else {
        StepDirection::South
    };
    let (result, delay) = owner
        .ordinary_paced_step(
            actor,
            session,
            direction,
            Some(before),
            Some((hare, target)),
        )
        .await;
    let delay = match delay {
        Some(d) => u64::try_from(d.as_micros())?,
        None if result.disposition == StepDisposition::Moved => {
            return Err("moved Hare pursuit step needs its mature pacing duration".into());
        }
        None => 200_000,
    };
    *next_due = owner
        .owner_now()
        .get()
        .checked_add(delay)
        .ok_or("Hare pacing deadline overflow")?;
    if std::env::var_os("OTERYN_SRV7_HARE_DIAGNOSTIC").is_some() {
        eprintln!(
            "SRV7_HARE_PACED_FOLLOW from={from:?} target={to:?} direction={direction:?} disposition={:?} next_due={next_due}",
            result.disposition
        );
    }
    Ok(result.disposition == StepDisposition::Moved)
}
