use super::*;

pub(super) async fn refresh<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Option<()> {
    use oteryn_protocol_oteryn::party::{
        CAPABILITY_PARTY_MEMBERSHIP_NOTICES_V1, DELTA_TYPE_PARTY_MEMBERSHIP_NOTICE_V1,
        PartyMembershipNotice, encode_party_membership_notice,
    };
    if !session
        .selected_capabilities
        .contains(CAPABILITY_PARTY_MEMBERSHIP_NOTICES_V1)
    {
        return Some(());
    }
    let notices = crate::durability::DurabilityRoot::with_background_holder(
        authority.take_party_membership_notices(session.admitted),
    )
    .await;
    for notice in notices {
        let Some(member_handle) = crate::gameplay_transport::party_command::party_member_handle(
            session.admitted,
            notice.character,
        ) else {
            continue;
        };
        let payload = encode_party_membership_notice(PartyMembershipNotice {
            member_handle,
            change: notice.change,
        })
        .ok()?;
        zones_nologout_transport::emit_delta(
            stream,
            &mut session.admitted,
            &mut session.sequence,
            DELTA_TYPE_PARTY_MEMBERSHIP_NOTICE_V1,
            &payload,
        )
        .await?;
    }
    Some(())
}
