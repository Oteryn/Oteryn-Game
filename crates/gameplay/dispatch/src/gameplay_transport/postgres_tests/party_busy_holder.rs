//! MP01: holder contention must not turn a fresh party invite into Unavailable.
use crate::bestiary_postgres_harness::{Harness, TestResult, debug};
use crate::gameplay_transport::{
    ComposedFreshAdmission, connection::AdmittedSession, party_command::SessionPartyView,
};
use oteryn_protocol_oteryn::{party::*, world_spatial_entities::EntityRef};
use std::{future::Future, task::Poll};

#[test]
fn nbmp0_party_invite_retries_busy_holder_through_real_caller() -> TestResult {
    super::party_membership::run_busy_holder()
}

pub(super) async fn check(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut [AdmittedSession],
    targets: &[EntityRef],
    h: &Harness,
) -> TestResult {
    let mut view = SessionPartyView::default();
    let payload = encode_party_intent(PartyIntent::Invite(targets[1])).map_err(debug)?;
    let held = owner.root.try_issue_semantic_pass()?;
    let mut invite = std::pin::pin!(owner.party_wire_command(
        admitted[0],
        700,
        COMMAND_TYPE_PARTY_INTENT,
        &payload,
        &mut view
    ));
    let first = std::future::poll_fn(|cx| Poll::Ready(invite.as_mut().poll(cx))).await;
    match first {
        Poll::Ready(reply) => {
            let result = decode_party_result(&reply.ok_or("invite reply")?).map_err(debug)?;
            return Err(format!("busy holder was not retried: {:?}", result.disposition).into());
        }
        Poll::Pending => drop(held),
    }
    let reply = tokio::time::timeout(std::time::Duration::from_secs(2), invite)
        .await?
        .ok_or("invite reply")?;
    assert_eq!(
        decode_party_result(&reply).map_err(debug)?.disposition,
        PartyDisposition::Ok
    );
    let result = super::party_membership::roundtrip(
        owner,
        &mut admitted[1],
        &[
            super::party_membership::Step::Query,
            super::party_membership::Step::Accept,
        ],
    )
    .await?;
    assert_eq!(result[0].invitations.len(), 1);
    assert_eq!(result[1].disposition, PartyDisposition::Ok);
    let members: i64 = sqlx::query_scalar("SELECT count(*) FROM game_party_members")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(members, 2);
    Ok(())
}
