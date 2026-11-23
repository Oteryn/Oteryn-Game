use super::super::{AdmittedSession, FreshAdmissionAuthority, SessionItemView};
use super::Dispatch;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::{
    ClientCommandView,
    quick_loot::{decode_quick_loot_request, encode_quick_loot_result},
};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
    item_view: Option<&SessionItemView>,
) -> Option<Dispatch> {
    let request = match decode_quick_loot_request(command.payload) {
        Ok(value) => value,
        Err(_) => return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new())),
    };
    let wire = crate::gameplay_transport::session_ingress::QuickLootWire::new(
        command.command_type, command.payload,
    )?;
    let (status, result) = authority
        .quick_loot(*admitted, command.command_id, request, wire, item_view)
        .await?;
    Some(Dispatch::Reply(
        status,
        encode_quick_loot_result(&result).ok()?,
    ))
}
