use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Dispatch {
    match authority
        .cyclopedia_character_command(*admitted, command.command_type, command.payload)
        .await
    {
        Ok(bytes) => Dispatch::Reply(CommandStatus::Accepted, bytes),
        Err(()) => Dispatch::Reply(CommandStatus::Rejected, Vec::new()),
    }
}
