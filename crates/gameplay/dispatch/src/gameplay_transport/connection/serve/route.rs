//! Execute command routing after ingress, input and capability gates.

use super::*;

pub(super) enum StepAction {
    Wait(tokio::time::Instant),
    Dispatch(Dispatch),
}

pub(super) async fn step<A: FreshAdmissionAuthority>(
    authority: &A,
    session: &mut ServeState,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    arrived_while_buffered: bool,
) -> StepAction {
    StepAction::Dispatch(
        // SPEED-1 (CONDITIONS-0 §4.3): every player step is paced. An early step waits in the
        // one-step buffer, without a result yet; one that arrived while the buffer was full is
        // refused and nothing moves.
        match decode_step_intent(command.payload) {
            Err(_) => Dispatch::Step(StepOutcome::rejected(), 0),
            // STEP-PACE-WIRE-1: the receipt is the time left until the pacing clock is due,
            // after the buffered step ran; none when that step did not move.
            Ok(_) if arrived_while_buffered => {
                let now = tokio::time::Instant::now();
                let left = session
                    .pacer
                    .wait_until(now)
                    .map(|due| due.duration_since(now));
                Dispatch::Step(StepOutcome::too_early(), too_early_step_wait_ms(left))
            }
            Ok(direction) => {
                // CHASE-1: client intent wins; the pending chase step and its route drop.
                session.chase_due = None;
                session.chase_yielded = false;
                authority
                    .yield_chase_to_client(session.actor, session.admitted.game_session_id)
                    .await;
                let now = tokio::time::Instant::now();
                let wait = match session.pacer.wait_until(now) {
                    Some(due) => Some(due),
                    None => {
                        begin_actor_step(
                            &mut session.pacer,
                            authority,
                            session.actor,
                            &session.step_holder,
                        )
                        .await
                    }
                };
                if let Some(due) = wait {
                    return StepAction::Wait(due);
                }
                let (outcome, duration) = authority
                    .paced_step(session.actor, session.admitted.game_session_id, direction)
                    .await;
                // The clock starts when the step committed, so a slow owner turn never
                // shortens the next step's wait.
                record_actor_step(
                    &mut session.pacer,
                    authority,
                    session.actor,
                    &session.step_holder,
                    duration,
                )
                .await;
                // STEP-PACE-WIRE-1: a moved step's receipt is the duration just recorded.
                let wait = if outcome.disposition == StepDisposition::Moved {
                    moved_step_wait_ms(duration)
                } else {
                    0
                };
                Dispatch::Step(outcome, wait)
            }
        },
    )
}

async fn logout<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    outcome: &mut CommandOutcome,
) -> Result<Dispatch, ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    Ok(
        // LOGOUT-WIRE-1 (§1.6): a non-empty intent is REJECTED with an empty payload before
        // any authority call. While the logout runs, frames are read and held, in order, for
        // after its result (LOGOUT-RL-01): a second logout among them is a protocol error.
        if decode_logout_intent(command.payload).is_err() {
            Dispatch::Unregistered
        } else {
            // A paced step can already have held the frames that follow this logout; a
            // second logout among them was read before this result too (#2127 Codex P2
            // 4279788741).
            outcome.second_logout = session
                .held
                .iter()
                .any(|(frame, _)| is_logout_command(frame, session.generation));
            let mut logout = std::pin::pin!(authority.logout(session.admitted));
            let reply = loop {
                let reading = !outcome.lost_while_logging_out
                    && session.held.len() < MAX_FRAMES_HELD_BEHIND_A_BUFFERED_STEP;
                let mut read = std::pin::pin!(session.frames.next(stream));
                let next = std::future::poll_fn(|context| {
                    if let std::task::Poll::Ready(reply) = logout.as_mut().poll(context) {
                        return std::task::Poll::Ready(Ok(reply));
                    }
                    if reading && let std::task::Poll::Ready(read) = read.as_mut().poll(context) {
                        return std::task::Poll::Ready(Err(read));
                    }
                    std::task::Poll::Pending
                })
                .await;
                match next {
                    Ok(reply) => break reply,
                    Err(Ok(frame)) => {
                        outcome.second_logout |= is_logout_command(&frame, session.generation);
                        session.held.push_back((frame, false));
                    }
                    Err(Err(_)) => outcome.lost_while_logging_out = true,
                }
            };
            match reply {
                LogoutReply::Result(result) => Dispatch::Logout(result),
                // §1.6 unknown outcome: no result; the loss lifecycle reconciles the row.
                LogoutReply::Unknown => {
                    return Err(ServeEnd::Disconnected);
                }
            }
        },
    )
}

include!(concat!(env!("OUT_DIR"), "/mechanic_routes.rs"));

#[cfg(test)]
#[path = "registry_tests.rs"]
mod registry_tests;
