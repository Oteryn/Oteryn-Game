use super::super::{AdmittedSession, ExactActorRef, FreshAdmissionAuthority};
use super::Dispatch;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    let Ok(bytes) = <[u8; 4]>::try_from(command.payload) else {
        return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
    };
    match authority
        .weapon_proficiency_browse(actor, admitted.game_session_id, u32::from_be_bytes(bytes))
        .await
    {
        Some(page) => Some(Dispatch::Reply(CommandStatus::Accepted, page.encode()?)),
        None => Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new())),
    }
}
