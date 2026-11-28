use super::super::super as transport;
use super::super::{AdmittedSession, FreshAdmissionAuthority, SessionItemView};
use super::Dispatch;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::{
    ClientCommandView,
    quick_loot::{self as wire, QuickLootRefusal},
};

pub(in super::super) fn selected_moved(command_type: u32, payload: &[u8]) -> bool {
    command_type == wire::COMMAND_TYPE_QUICK_LOOT
        && wire::QuickLootClickOutcome::decode(payload) == Ok(wire::QuickLootClickOutcome::Moved)
}

pub(in super::super) fn stale_move(command_type: u32, status: CommandStatus, payload: &[u8]) -> bool {
    matches!(
        command_type,
        wire::COMMAND_TYPE_QUICK_LOOT | wire::COMMAND_TYPE_QUICK_LOOT_DISTRIBUTE
    ) && status == CommandStatus::Rejected
        && wire::decode_quick_loot_refusal(payload) == Ok(QuickLootRefusal::Stale)
}

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
    items: Option<&SessionItemView>,
) -> Option<Dispatch> {
    let invalid = |stage| {
        let reason = QuickLootRefusal::InvalidRequest;
        transport::quick_loot::log_refusal(admitted, command.command_id, stage, &reason);
        Some(Dispatch::Reply(
            CommandStatus::Rejected,
            wire::encode_quick_loot_refusal(reason),
        ))
    };
    let use_command = transport::connection::UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: command.command_id,
        item_fence: admitted.item_fence,
    };
    if command.command_type != wire::COMMAND_TYPE_QUICK_LOOT {
        if command.payload.is_empty() || command.payload.len() > wire::MAX_QUICK_LOOT_REQUEST_BYTES
        {
            return invalid("request_size");
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
        return invalid("request_decode");
    };
    if request.action.command_type() != command.command_type {
        return invalid("action_command_type");
    }
    if let Some(handle) = request.selected_item {
        use transport::item_view::ItemKey;
        let selection = items.and_then(|view| {
            let key @ ItemKey::Instance(entry) = view.resolve(handle)? else {
                return None;
            };
            let ItemKey::Entity(corpse_identity) = view.open_corpse_of(&key)? else {
                return None;
            };
            Some(transport::quick_loot::OpenLootSelection { corpse_identity, entry })
        });
        let (status, payload) = authority
            .quick_loot_selected(*admitted, use_command, request, selection)
            .await?;
        return Some(Dispatch::Reply(status, payload));
    }
    let (status, payload) = authority
        .quick_loot_wire(*admitted, use_command, request)
        .await?;
    Some(Dispatch::Reply(status, payload))
}
