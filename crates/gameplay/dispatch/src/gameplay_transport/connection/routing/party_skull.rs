use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::durability::DurabilityError;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::party_skull::{CAPABILITY_PARTY_SKULL_V1, PartyIconQueryRefusal, encode_party_icons};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: AdmittedSession,
    payload: &[u8],
    visibility: Option<&crate::gameplay_transport::world_spatial::SessionVisibility>,
) -> Dispatch {
    if !matches!(payload, [] | [8, 1])
        || !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_PARTY_SKULL_V1)
    {
        return Dispatch::Unregistered;
    }
    if payload == [8, 1] {
        return match authority
            .observe_pvp_status(admitted)
            .await
            .and_then(|status| status.encode().ok())
        {
            Some(bytes) => Dispatch::Reply(crate::foundation::CommandStatus::Accepted, bytes),
            None => Dispatch::Unregistered,
        };
    }
    let mut icons = match authority.query_party_icons(admitted).await {
        Ok(icons) => icons,
        Err(DurabilityError::RootUnavailable | DurabilityError::RootPassDeadlineExceeded) => {
            return refusal(PartyIconQueryRefusal::Busy);
        }
        Err(_) => return refusal(PartyIconQueryRefusal::Unavailable),
    };
    icons.retain(|icon| visibility.is_some_and(|view| view.shows(icon.entity)));
    icons.sort_by_key(|icon| icon.entity);
    match encode_party_icons(&icons) {
        Ok(bytes) => Dispatch::Reply(CommandStatus::Accepted, bytes),
        Err(_) => refusal(PartyIconQueryRefusal::Unavailable),
    }
}

fn refusal(reason: PartyIconQueryRefusal) -> Dispatch {
    Dispatch::Reply(CommandStatus::Rejected, reason.encode().to_vec())
}

#[cfg(test)]
#[path = "party_skull/tests.rs"]
mod tests;
