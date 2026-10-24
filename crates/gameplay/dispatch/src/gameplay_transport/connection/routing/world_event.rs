//! World event command body; predicate and connection control flow stay in `connection.rs`.

use super::super::super as transport;
use super::super::{AdmittedSession, FreshAdmissionAuthority, UseCommand};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    Some({
        match transport::world_event::dispatch(
            authority,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command.command_id,
                item_fence: admitted.item_fence,
            },
            command.payload,
        )
        .await
        {
            Some(reply) => Dispatch::WorldEvent(reply),
            None => return None,
        }
    })
}
