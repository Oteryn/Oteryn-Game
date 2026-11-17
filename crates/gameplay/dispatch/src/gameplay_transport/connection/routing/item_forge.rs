use super::super::{AdmittedSession, FreshAdmissionAuthority, SessionItemView};
use super::Dispatch;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus, item_forge::Request};
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
    item_view: Option<&SessionItemView>,
) -> Option<Dispatch> {
    if command.command_type == oteryn_protocol_oteryn::item_forge::COMMAND_TYPE_FORGE_DUST_QUERY {
        if !command.payload.is_empty() {
            return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
        }
        let bytes = authority
            .item_forge_balance(*admitted, command.command_id)
            .await?;
        return Some(Dispatch::Reply(
            if bytes.is_empty() {
                CommandStatus::Rejected
            } else {
                CommandStatus::Accepted
            },
            bytes,
        ));
    }
    let Some(request) = Request::decode(command.payload) else {
        return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
    };
    let bytes = authority
        .item_forge(*admitted, command.command_id, request, item_view)
        .await?;
    Some(Dispatch::Reply(
        if bytes.is_empty() {
            CommandStatus::Rejected
        } else {
            CommandStatus::Accepted
        },
        bytes,
    ))
}
