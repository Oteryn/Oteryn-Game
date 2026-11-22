use super::super::super as transport;
use super::super::{AdmittedSession, FreshAdmissionAuthority, SessionItemView};
use super::Dispatch;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::{
    ClientCommandView,
    quick_loot::{self as wire, encode_quick_loot_result},
};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
    items: Option<&SessionItemView>,
) -> Option<Dispatch> {
    let use_command = transport::connection::UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: command.command_id,
        item_fence: admitted.item_fence,
    };
    if command.command_type != wire::COMMAND_TYPE_QUICK_LOOT {
        if command.payload.is_empty() || command.payload.len() > wire::MAX_QUICK_LOOT_REQUEST_BYTES
        {
            return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
        }
        let request = wire::decode_quick_loot_view_request(command.payload, |handle| match items?
            .resolve(handle)?
        {
            transport::item_view::ItemKey::Instance(id) => Some(id),
            _ => None,
        })
        .ok()
        .filter(|request| request.action.command_type() == command.command_type);
        let (status, payload) = authority
            .quick_loot_destinations(
                *admitted,
                use_command,
                (command.command_type, command.payload),
                request,
            )
            .await?;
        return Some(Dispatch::Reply(status, payload));
    }
    let Ok(request) = wire::decode_quick_loot_request(command.payload) else {
        return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
    };
    if request.action.command_type() != command.command_type {
        return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
    }
    let (status, result) = authority
        .quick_loot(*admitted, use_command, request)
        .await?;
    Some(Dispatch::Reply(
        status,
        encode_quick_loot_result(&result).ok()?,
    ))
}
