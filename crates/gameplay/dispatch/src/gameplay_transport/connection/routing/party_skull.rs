use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::durability::DurabilityError;
use oteryn_protocol_oteryn::party_skull::{CAPABILITY_PARTY_SKULL_V1, encode_party_icons};

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
        return match authority.observe_pvp_status(admitted).await {
            Ok(status) => status.encode().map_or(Dispatch::Unregistered, |bytes| {
                Dispatch::Reply(crate::foundation::CommandStatus::Accepted, bytes)
            }),
            Err(error) => refused(error),
        };
    }
    let mut icons = match authority.observe_party_icons(admitted).await {
        Ok(icons) => icons,
        Err(error) => return refused(error),
    };
    icons.retain(|icon| visibility.is_some_and(|view| view.shows(icon.entity)));
    icons.sort_by_key(|icon| icon.entity);
    match encode_party_icons(&icons) {
        Ok(bytes) => Dispatch::Reply(crate::foundation::CommandStatus::Accepted, bytes),
        Err(_) => Dispatch::Unregistered,
    }
}

fn refused(error: DurabilityError) -> Dispatch {
    if matches!(error, DurabilityError::RootUnavailable | DurabilityError::RootPassDeadlineExceeded) {
        Dispatch::Reply(
            crate::foundation::CommandStatus::Rejected,
            oteryn_protocol_oteryn::COMMAND_BUSY_REFUSAL.to_vec(),
        )
    } else {
        Dispatch::Unregistered
    }
}

#[cfg(test)]
#[path = "party_skull/tests.rs"]
mod tests;
