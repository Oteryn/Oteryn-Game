use super::super::{AdmittedSession, ExactActorRef, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus, depot_search as wire};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    let Ok(request) = wire::decode_request(command.payload) else {
        return Some(Dispatch::Unregistered);
    };
    let reply = authority
        .depot_search_completion(actor, *admitted, command.command_id, request)
        .await?;
    Some(match wire::encode_reply(&reply) {
        Ok(bytes) => Dispatch::Reply(CommandStatus::Accepted, bytes),
        Err(_) => Dispatch::Unregistered,
    })
}
