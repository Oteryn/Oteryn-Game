use super::super::super::forge_preview::Outcome;
use super::super::{AdmittedSession, FreshAdmissionAuthority, SessionItemView};
use super::Dispatch;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::{ClientCommandView, forge_preview::Request};
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
    views: &Option<SessionItemView>,
) -> Option<Dispatch> {
    let Some(request) = Request::decode(command.payload) else {
        return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
    };
    match authority
        .forge_preview(*admitted, command.command_id, request, views.as_ref())
        .await
    {
        Outcome::Payload(bytes) => Some(Dispatch::Reply(CommandStatus::Accepted, bytes)),
        Outcome::Rejected => Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new())),
        Outcome::Unavailable => None,
    }
}
