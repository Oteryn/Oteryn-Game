//! Appearance command body; predicate and connection control flow stay in `connection.rs`.

use super::super::{
    AdmittedSession, ExactActorRef, FreshAdmissionAuthority, UseCommand, appearance_change,
};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Dispatch {
    let request = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: command.command_id,
        item_fence: admitted.item_fence,
    };
    if matches!(
        command.command_type,
        oteryn_protocol_oteryn::appearance::familiar::QUERY
            | oteryn_protocol_oteryn::appearance::familiar::CHANGE
    ) {
        let payload = authority
            .familiar_appearance(actor, request, command.command_type, command.payload)
            .await;
        return match payload {
            Some(payload) => Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload),
            None => Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new()),
        };
    }
    Dispatch::Appearance(
        appearance_change::dispatch(authority, actor, request, command.payload).await,
    )
}
