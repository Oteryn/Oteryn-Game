use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::ClientCommandView;
use oteryn_protocol_oteryn::cyclopedia_tracker::{decode_request, encode_snapshot};

/// A malformed payload or a refused change is `Rejected` with no payload; an accepted command
/// answers with the refreshed tracker. An unencodable refresh is a server fault.
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    let Some(request) = decode_request(command.payload) else {
        return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
    };
    match authority
        .cyclopedia_tracker(*admitted, command.command_type, request)
        .await
    {
        Some(snapshot) => Some(Dispatch::Reply(
            CommandStatus::Accepted,
            encode_snapshot(&snapshot)?,
        )),
        None => Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new())),
    }
}
