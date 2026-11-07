//! PUSH-ROUTE-1: one push per command, paced by the connection's push delay.
use super::*;
use oteryn_protocol_oteryn::creature_push::{CreaturePushDisposition, decode_creature_push_intent};
use std::time::Duration;

/// Canary configmanager.cpp:337 `PUSH_DELAY`: 1000 ms between committed pushes (`PUSH-RL-03`).
const PUSH_DELAY: Duration = Duration::from_millis(1000);

fn reply(disposition: CreaturePushDisposition) -> Dispatch {
    Dispatch::Reply(
        if disposition == CreaturePushDisposition::Rejected {
            CommandStatus::Rejected
        } else {
            CommandStatus::Accepted
        },
        vec![8, disposition as u8],
    )
}

pub(super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    session: &mut ServeState,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
) -> Result<Dispatch, ServeEnd> {
    let now = tokio::time::Instant::now();
    if !session.push_window.admit(now) {
        return Ok(reply(CreaturePushDisposition::Rejected));
    }
    let Ok(intent) = decode_creature_push_intent(command.payload) else {
        return Ok(reply(CreaturePushDisposition::Rejected));
    };
    if session.push_due.is_some_and(|due| now < due) {
        return Ok(reply(CreaturePushDisposition::Exhausted));
    }
    let disposition = tokio::time::timeout(
        Duration::from_secs(10),
        authority.push_creature(session.actor, session.admitted.game_session_id, intent),
    )
    .await
    .map_err(|_| ServeEnd::Disconnected)?;
    if disposition == CreaturePushDisposition::Moved {
        session.push_due = Some(tokio::time::Instant::now() + PUSH_DELAY);
    }
    Ok(reply(disposition))
}
