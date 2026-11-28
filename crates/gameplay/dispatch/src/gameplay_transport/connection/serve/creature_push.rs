//! PUSH-ROUTE-1: one push per command, paced by the connection's push delay.
use super::*;
use oteryn_protocol_oteryn::creature_push::{
    CreaturePushDisposition, CreaturePushIntent, decode_creature_push_intent,
};
use oteryn_protocol_oteryn::world_interaction::{
    COMMAND_TYPE_PUSH_INTENT, PushIntent, decode_push_intent, encode_push_result,
};
use std::time::Duration;

/// Canary configmanager.cpp:337 `PUSH_DELAY`: 1000 ms between committed pushes (`PUSH-RL-03`).
const PUSH_DELAY: Duration = Duration::from_millis(1000);

enum PushRequest {
    Legacy(PushIntent),
    Creature(CreaturePushIntent),
}

fn reply(legacy: bool, disposition: CreaturePushDisposition) -> Result<Dispatch, ServeEnd> {
    let status = if disposition == CreaturePushDisposition::Rejected {
        CommandStatus::Rejected
    } else {
        CommandStatus::Accepted
    };
    let payload = if legacy {
        let outcome = match disposition {
            CreaturePushDisposition::Moved => UseDisposition::Committed,
            CreaturePushDisposition::NotMovable => UseDisposition::NotMovable,
            CreaturePushDisposition::Rejected => UseDisposition::Rejected,
            CreaturePushDisposition::NotPossible => UseDisposition::NotPossible,
            CreaturePushDisposition::NotEnoughRoom => UseDisposition::NoRoom,
            CreaturePushDisposition::DestinationOutOfReach => UseDisposition::TooFar,
            CreaturePushDisposition::Exhausted => UseDisposition::Exhausted,
        };
        encode_push_result(outcome, None).map_err(|_| ServeEnd::Disconnected)?
    } else {
        vec![8, disposition as u8]
    };
    Ok(Dispatch::Reply(status, payload))
}

pub(super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    session: &mut ServeState,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
) -> Result<Dispatch, ServeEnd> {
    let legacy = command.command_type == COMMAND_TYPE_PUSH_INTENT;
    let now = tokio::time::Instant::now();
    if !session.push_window.admit(now) {
        return reply(legacy, CreaturePushDisposition::Rejected);
    }
    let request = if legacy {
        decode_push_intent(command.payload)
            .ok()
            .map(PushRequest::Legacy)
    } else {
        decode_creature_push_intent(command.payload)
            .ok()
            .map(PushRequest::Creature)
    };
    let Some(request) = request else {
        return reply(legacy, CreaturePushDisposition::Rejected);
    };
    if session.push_due.is_some_and(|due| now < due) {
        return reply(legacy, CreaturePushDisposition::Exhausted);
    }
    let actor = session.actor;
    let game_session = session.admitted.game_session_id;
    let disposition = tokio::time::timeout(Duration::from_secs(10), async {
        match request {
            PushRequest::Legacy(intent) => {
                authority
                    .push_creature_on_target_floor(actor, game_session, intent)
                    .await
            }
            PushRequest::Creature(intent) => {
                authority.push_creature(actor, game_session, intent).await
            }
        }
    })
    .await
    .map_err(|_| ServeEnd::Disconnected)?;
    if disposition == CreaturePushDisposition::Moved {
        session.push_due = Some(tokio::time::Instant::now() + PUSH_DELAY);
    }
    reply(legacy, disposition)
}
