use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus, hireling::Request};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    let request = Request::decode(command.payload)?;
    let active = authority
        .hireling(*admitted, command.command_id, request)
        .await?;
    Some(Dispatch::Reply(
        CommandStatus::Accepted,
        vec![u8::from(active)],
    ))
}
