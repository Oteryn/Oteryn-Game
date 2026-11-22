use super::super::{AdmittedSession, FreshAdmissionAuthority, SessionVisibility};
use super::Dispatch;
use oteryn_protocol_oteryn::{
    CommandStatus,
    zones_nologout::{CAPABILITY_PLAYER_STATUS_V1, decode_player_status_query},
};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: AdmittedSession,
    payload: &[u8],
    visibility: Option<&SessionVisibility>,
) -> Dispatch {
    if !admitted.continuity.selected_capabilities.contains(CAPABILITY_PLAYER_STATUS_V1) {
        return Dispatch::Unregistered;
    }
    let Some(target) = decode_player_status_query(payload) else {
        return Dispatch::Unregistered;
    };
    if !visibility.is_some_and(|visibility| visibility.shows(target)) {
        return Dispatch::Unregistered;
    }
    match authority.player_status(admitted, target).await {
        Some(status) => Dispatch::Reply(CommandStatus::Accepted, status.encode().to_vec()),
        None => Dispatch::Unregistered,
    }
}
