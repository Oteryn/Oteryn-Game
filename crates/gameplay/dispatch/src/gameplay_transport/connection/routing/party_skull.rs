use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::party_skull::{CAPABILITY_PARTY_SKULL_V1, encode_party_icons};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: AdmittedSession,
    payload: &[u8],
    visibility: Option<&crate::gameplay_transport::world_spatial::SessionVisibility>,
) -> Dispatch {
    if !payload.is_empty()
        || !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_PARTY_SKULL_V1)
    {
        return Dispatch::Unregistered;
    }
    let Some(mut icons) = authority.observe_party_icons(admitted).await else {
        return Dispatch::Unregistered;
    };
    icons.retain(|icon| visibility.is_some_and(|view| view.shows(icon.entity)));
    icons.sort_by_key(|icon| icon.entity);
    match encode_party_icons(&icons) {
        Ok(bytes) => Dispatch::Reply(crate::foundation::CommandStatus::Accepted, bytes),
        Err(_) => Dispatch::Unregistered,
    }
}
