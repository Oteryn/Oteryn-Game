//! Bed command body; predicate and connection control flow stay in `connection.rs`.

use super::super::super as transport;
use super::super::{
    AdmittedSession, ExactActorRef, FreshAdmissionAuthority, SessionItemView, UseCommand,
    decode_bed_sleep_intent,
};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
    item_view: &Option<SessionItemView>,
) -> Option<Dispatch> {
    Some({
        match transport::bed_sleep::dispatch(
            authority,
            actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command.command_id,
                item_fence: admitted.item_fence,
            },
            decode_bed_sleep_intent(command.payload),
            item_view.as_ref(),
        )
        .await
        {
            Ok(outcome) => Dispatch::Use(outcome),
            // An unacknowledged commit may already have ended the session.
            // Send no refusal; the terminal/loss lifecycle reconciles it.
            Err(_) => return None,
        }
    })
}
