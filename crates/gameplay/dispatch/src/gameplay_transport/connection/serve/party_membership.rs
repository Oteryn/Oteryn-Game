use super::*;

use std::time::Duration;

pub(super) const REFRESH: Duration = Duration::from_secs(1);
// A refused drain can coincide with a peer's successful drain on the shared
// holder. Offset this connection's next turn so the same two polls can separate.
const RETRY: Duration = Duration::from_millis(100);

pub(super) async fn refresh<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd> {
    use oteryn_protocol_oteryn::party::{
        DELTA_TYPE_PARTY_MEMBERSHIP_NOTICE_V1, PartyMembershipNotice,
        encode_party_membership_notice,
    };
    let Some(notices) = crate::durability::DurabilityRoot::with_background_holder(
        authority.take_party_membership_notices(session.admitted),
    )
    .await
    else {
        if let Some(refresh) = session.party_notice_refresh.as_mut() {
            refresh.reset_after(RETRY);
        }
        return Ok(());
    };
    let write = async {
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
    };
    match tokio::time::timeout(session.policy.interval, write).await {
        Ok(Some(())) => Ok(()),
        Ok(None) => Err(ServeEnd::Disconnected),
        Err(_) => Err(ServeEnd::ControlLost("party_notice_write_timeout")),
    }
}
