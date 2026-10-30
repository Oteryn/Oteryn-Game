use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::durability::cyclopedia_tracker::TrackerKind;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::{ClientCommandView, cyclopedia_tracker::{decode_request, encode_snapshot}};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(authority: &A, admitted: &AdmittedSession, command: ClientCommandView<'_>) -> Option<Dispatch> {
    let request = match decode_request(command.payload) { Some(value) => value, None => return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new())) };
    let kind = match command.command_type { 383 => TrackerKind::Bestiary, 384 => TrackerKind::Bosstiary, _ => return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new())) };
    match authority.cyclopedia_tracker(*admitted, kind, request).await {
        Some(keys) => Some(Dispatch::Reply(CommandStatus::Accepted, encode_snapshot(&keys)?)),
        None => Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new())),
    }
}
