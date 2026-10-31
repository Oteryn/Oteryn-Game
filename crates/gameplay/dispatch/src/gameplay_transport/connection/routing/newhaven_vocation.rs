use super::super::{AdmittedSession, ExactActorRef, FreshAdmissionAuthority, UseCommand};
use super::Dispatch;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::{
    ClientCommandView,
    newhaven_vocation::{NewhavenChoiceResult, NewhavenVocationRequest},
};
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    let reply = if let Some(request) = NewhavenVocationRequest::decode(command.payload) {
        authority
            .newhaven_vocation_choice(
                actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command.command_id,
                    item_fence: admitted.item_fence,
                },
                request,
            )
            .await?
    } else {
        NewhavenChoiceResult::Rejected
    };
    Some(Dispatch::Reply(CommandStatus::Accepted, reply.encode()))
}
