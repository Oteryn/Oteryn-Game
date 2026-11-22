use super::super::super as transport;
use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::{
    ClientCommandView,
    quick_loot::{self as wire, decode_quick_loot_request, encode_quick_loot_result},
};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    let request = match decode_quick_loot_request(command.payload) {
        Ok(value) => value,
        Err(_) => return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new())),
    };
    if request.action.command_type() != command.command_type {
        return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
    }
    let use_command = transport::connection::UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: command.command_id,
        item_fence: admitted.item_fence,
    };
    if command.command_type != wire::COMMAND_TYPE_QUICK_LOOT {
        let (status, payload) = authority
            .quick_loot_destinations(*admitted, use_command, request)
            .await?;
        return Some(Dispatch::Reply(status, payload));
    }
    let (status, result) = authority
        .quick_loot(*admitted, use_command, request)
        .await?;
    Some(Dispatch::Reply(
        status,
        encode_quick_loot_result(&result).ok()?,
    ))
}
