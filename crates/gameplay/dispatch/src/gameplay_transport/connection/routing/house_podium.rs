use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::gameplay_transport::house_podium::Request;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    let request = Request::decode(command.command_type, command.payload)?;
    let reply = authority.house_podium(*admitted, request).await?;
    Some(Dispatch::Reply(CommandStatus::Accepted, reply))
}
