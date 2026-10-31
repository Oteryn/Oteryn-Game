use super::super::{AdmittedSession, ExactActorRef, FreshAdmissionAuthority, UseCommand};
use super::Dispatch;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::{
    ClientCommandView,
    weapon_proficiency::{SelectPerkReply, SelectPerkRequest, SelectPerkResult},
};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    let reply = match SelectPerkRequest::decode(command.payload) {
        Some(request) => {
            let command = UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command.command_id,
                item_fence: admitted.item_fence,
            };
            authority
                .weapon_proficiency_select(actor, command, request)
                .await?
        }
        None => SelectPerkReply {
            result: SelectPerkResult::Rejected,
            selections: Vec::new(),
        },
    };
    Some(Dispatch::Reply(CommandStatus::Accepted, reply.encode()?))
}
