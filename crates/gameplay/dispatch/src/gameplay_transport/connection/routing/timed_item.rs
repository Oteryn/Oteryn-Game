//! Timed item command body; predicate and connection control flow stay in `connection.rs`.

use super::super::super as transport;
use super::super::{
    AdmittedSession, ExactActorRef, FreshAdmissionAuthority, SessionItemView, UseCommand,
};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    command: ClientCommandView<'_>,
    item_view: &mut Option<SessionItemView>,
) -> Option<Dispatch> {
    Some({
        let (payload, deltas) = transport::timed_item_wire::dispatch(
            authority,
            actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command.command_id,
                item_fence: admitted.item_fence,
            },
            command.payload,
            item_view.as_mut(),
        )
        .await?;
        if let Some(view) = item_view.as_ref() {
            admitted.continuity.item_view = view.continuity();
        }
        Dispatch::Timed(payload, deltas)
    })
}
