// MP01 fixture: named visibility and party invites when the shared holder is busy.
use crate::bestiary_postgres_harness::{Harness, TestResult, debug};
use crate::durability::DurabilityRoot;
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmissionRefusal, AdmittedSession, FreshAdmissionAuthority},
    party_command::SessionPartyView,
};
use oteryn_protocol_oteryn::{party::*, world_spatial_entities::EntityRef};
use std::{future::Future, task::Poll};

pub(super) async fn check(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut [AdmittedSession],
    targets: &[EntityRef],
    h: &Harness,
) -> TestResult {
    let observer = admitted[0].runtime_actor.ok_or("party observer")?;
    let held = owner.root.try_issue_semantic_pass()?;
    let projection = owner.observe_named_visible_entities_result(observer).await;
    assert!(
        matches!(projection, Err(AdmissionRefusal::Unavailable)),
        "named visibility erased the current holder contention: {projection:?}"
    );
    drop(held);
    let caught_up = owner
        .observe_named_visible_entities_result(observer)
        .await
        .map_err(debug)?;
    assert!(
        caught_up.entities.iter().any(|entity| {
            entity.identity == targets[1].identity
                && entity.generation == targets[1].generation
                && entity.display_name.is_some()
        }),
        "the next projection must publish the already-admitted peer's name"
    );

    // A periodic map cache miss must yield the spell lane on a checkout miss.
    // The party command needs that lane before it can take its durability turn.
    let held = owner.root.try_issue_semantic_pass()?;
    let scope = owner.spell_lane.runtime_scope();
    let projection = DurabilityRoot::with_background_holder(async {
        let _permit = owner.spell_lane_permit().await.ok_or("map spell lane")?;
        let observed = crate::gameplay_transport::loop_move::projection_read(
            owner.root,
            async || owner.root.read_item_loop_ground_for_scope(scope).await,
            crate::gameplay_transport::root_holder_busy,
        )
        .await;
        assert!(observed.is_none());
        TestResult::Ok(())
    });
    tokio::time::timeout(crate::gameplay_transport::COMMAND_HOLDER_WINDOW, projection).await??;
    held.run(|_, _| Box::pin(async { Ok(()) })).await?;

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
    let result = super::party_membership_fixtures::roundtrip(
        owner,
        &mut admitted[1],
        &[
            super::party_membership_fixtures::Step::Query,
            super::party_membership_fixtures::Step::Accept,
            super::party_membership_fixtures::Step::Query,
        ],
    )
    .await?;
    assert_eq!(result[0].invitations.len(), 1);
    assert_eq!(result[1].disposition, PartyDisposition::Ok);
    assert_eq!(result[2].members.len(), 2);
    let members: i64 = sqlx::query_scalar("SELECT count(*) FROM game_party_members")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(members, 2);
    Ok(())
}
