use super::super::{AdmittedSession, ExactActorRef, FreshAdmissionAuthority, UseCommand};
use super::Dispatch;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::{ClientCommandView, weapon_proficiency::ModificationRequest};

/// Commands 379, 380 and 392-395; the command type selects the operation. A payload that does
/// not decode for its type is rejected without a result.
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    let Some(request) = ModificationRequest::decode(command.command_type, command.payload) else {
        return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
    };
    let use_command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: command.command_id,
        item_fence: admitted.item_fence,
    };
    let reply = authority
        .weapon_proficiency_modify(actor, use_command, request)
        .await?;
    Some(Dispatch::Reply(CommandStatus::Accepted, reply.encode()))
}
