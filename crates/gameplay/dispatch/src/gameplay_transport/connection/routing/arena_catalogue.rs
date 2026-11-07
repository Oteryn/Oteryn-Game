use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Dispatch {
    match authority
        .arena_catalogue_command(*admitted, command.command_id, command.payload)
        .await
    {
        Some(bytes) => Dispatch::Reply(CommandStatus::Accepted, bytes),
        None => Dispatch::Unregistered,
    }
}
