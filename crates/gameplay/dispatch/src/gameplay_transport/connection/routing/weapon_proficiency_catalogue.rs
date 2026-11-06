use super::super::{AdmittedSession, ExactActorRef, FreshAdmissionAuthority, UseCommand};
use super::Dispatch;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    let Ok(offset) = <[u8; 4]>::try_from(command.payload) else {
        return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
    };
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: command.command_id,
        item_fence: admitted.item_fence,
    };
    let Some(page) = authority
        .weapon_proficiency_catalogue(actor, command, u32::from_be_bytes(offset))
        .await
    else {
        return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
    };
    Some(Dispatch::Reply(CommandStatus::Accepted, page.encode()?))
}
