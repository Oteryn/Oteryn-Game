//! Real source return STEP and existing bounded retirement, no direct state edits.
use super::super::fixture::Harness;
use super::*;
use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
pub(super) async fn return_and_retire(
    source: &ComposedFreshAdmission<'_, '_, '_>,
    instance: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: AdmittedSession,
    peer: AdmittedSession,
    body: [u8; 16],
    picked: [u8; 16],
) -> TestResult {
    super::pickup::walk(
        instance,
        peer,
        MovementLocalPosition {
            x: 33043,
            y: 32986,
            floor: -8,
        },
        false,
    )
    .await?;
    let peer_return = source
        .activity_runtimes
        .pending_switch(peer)
        .map_err(|_| "actual peer return barrier")?
        .ok_or("second actual source participant must return")?;
    assert!(matches!(
        peer_return
            .destination
            .item_fence
            .ok_or("peer return fence")?
            .runtime_scope,
        RuntimeScopeRefV1::Channel { .. }
    ));
    let returned_peer = source
        .resolve_runtime_view(peer_return.destination)
        .await
        .map_err(|_| "resolve actual returned peer")?;
    let returned_peer = returned_peer.as_ref().unwrap_or(source);
    super::pickup::walk(
        returned_peer,
        peer_return.destination,
        MovementLocalPosition {
            x: 33038,
            y: 32964,
            floor: 8,
        },
        false,
    )
    .await?;
    let old_scope = admitted
        .item_fence
        .ok_or("actual Instance source fence")?
        .runtime_scope;
    super::pickup::walk(
        instance,
        admitted,
        MovementLocalPosition {
            x: 33043,
            y: 32986,
            floor: -8,
        },
        false,
    )
    .await?;
    let switch = source
        .activity_runtimes
        .pending_switch(admitted)
        .map_err(|_| "actual source return barrier")?
        .ok_or("real source return STEP must produce its durable/native Channel successor")?;
    let next = switch.destination;
    let fence = next.item_fence.ok_or("actual returned item fence")?;
    assert!(matches!(
        fence.runtime_scope,
        RuntimeScopeRefV1::Channel { .. }
    ));
    assert_ne!(next.game_session_id, admitted.game_session_id);
    let returned = source
        .resolve_runtime_view(next)
        .await
        .map_err(|_| "resolve original Channel")?;
    let returned = returned.as_ref().unwrap_or(source);
    let at = returned
        .runtime
        .lock()
        .await
        .read_actor_position(next.runtime_actor.ok_or("actual returned actor")?)
        .map_err(debug)?
        .position();
    assert_eq!(
        (at.x, at.y, at.floor),
        (33040, 32962, 8),
        "the actual original source TP destination"
    );
    let reasons: Vec<(i16, Option<String>)> =
        sqlx::query_as("SELECT reason,command_id::text FROM game_activity_runtime_returns")
            .fetch_all(&h.pool)
            .await?;
    assert_eq!(
        reasons,
        vec![(10, None), (10, None)],
        "both physical source exits have server occurrences and no fabricated client commands"
    );
    for _ in 0..64 {
        source
            .activity_lifecycle_pass()
            .await
            .map_err(|_| "actual bounded retirement pass")?;
        if source.activity_runtimes.get(old_scope, 1).is_none() {
            break;
        }
    }
    assert!(
        source.activity_runtimes.get(old_scope, 1).is_none(),
        "native Instance retires only after its real return and Ground conservation"
    );
    let body_state:(i16,i64)=sqlx::query_as("SELECT lifecycle,quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(body.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        body_state,
        (
            i16::try_from(crate::durability::item_transfer_audit::ITEM_LIFECYCLE_RETIRED)?,
            0
        )
    );
    let custody:(i16,i64)=sqlx::query_as("SELECT i.lifecycle,count(e.item_instance_id) FROM game_item_instances i LEFT JOIN game_item_container_entries e USING(item_instance_id,world_id) WHERE i.item_instance_id=encode($1,'hex')::uuid GROUP BY i.lifecycle")
        .bind(picked.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        custody,
        (1, 1),
        "the actual picked reward UUID survives room retirement"
    );
    let counts:(i64,i64)=sqlx::query_as("SELECT(SELECT count(*) FROM game_item_mint_receipts),(SELECT count(*) FROM game_item_transfer_receipts)").fetch_one(&h.pool).await?;
    source
        .activity_lifecycle_pass()
        .await
        .map_err(|_| "retirement replay")?;
    assert_eq!(sqlx::query_as::<_,(i64,i64)>("SELECT(SELECT count(*) FROM game_item_mint_receipts),(SELECT count(*) FROM game_item_transfer_receipts)").fetch_one(&h.pool).await?,counts,"retirement replay creates neither another body nor another reward");
    Ok(())
}
