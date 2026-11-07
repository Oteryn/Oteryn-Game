//! Two independent actual Characters share the source room without a party gate.
use super::*;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
pub(super) async fn join(
    source: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &super::super::fixture::Harness,
    original: AdmittedSession,
    first: AdmittedSession,
) -> TestResult<AdmittedSession> {
    let instance = source
        .resolve_runtime_view(first)
        .await
        .map_err(|_| "resolve first actual source participant")?
        .ok_or("actual source Instance")?;
    super::pickup::walk(
        &instance,
        first,
        MovementLocalPosition {
            x: 33042,
            y: 32993,
            floor: -8,
        },
        true,
    )
    .await?;
    let boss_before = {
        let runtime = instance.runtime.lock().await;
        runtime
            .positioned_actor_census()
            .map_err(debug)?
            .into_iter()
            .find_map(|(actor, _, session)| {
                session
                    .is_none()
                    .then(|| {
                        runtime
                            .companion_snapshot(actor)
                            .ok()
                            .filter(|snapshot| {
                                snapshot.state.policy.definition_key
                                    == "oteryn:creature.rakesh_moonfang"
                            })
                            .map(|snapshot| (actor, snapshot.health, snapshot.maximum_health))
                    })
                    .flatten()
            })
            .ok_or("actual live shared Source boss before second entry")?
    };
    drop(instance);
    let deadline: i64 = sqlx::query_scalar("SELECT deadline FROM game_boss_room_activities")
        .fetch_one(&h.pool)
        .await?;
    let mut peer = super::super::scenarios::peer_at_for_scope_transfer(
        h,
        source,
        original,
        MovementLocalPosition {
            x: 33039,
            y: 32964,
            floor: 8,
        },
    )
    .await?;
    assert!(
        source.progression_binding(peer.game_session_id).is_some(),
        "peer factory must bind its genuine active progression policy"
    );
    super::assert_current_progression(
        h,
        peer.item_fence
            .ok_or("actual peer Character fence")?
            .character_id
            .as_bytes(),
    )
    .await?;
    let wire = oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1;
    peer.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            source.offered_capabilities(),
            &[4, 6, wire],
        )
        .ok_or("actual offered peer capabilities")?;
    source
        .activity_runtimes
        .remember(peer)
        .map_err(|_| "retain actual peer source")?;
    let actor = peer.runtime_actor.ok_or("actual peer actor")?;
    // The negative uses a truthful recovered initial pose at the source tile;
    // it is a durable assignment-fence proof, not a travel claim.
    super::stale_join::reject(source, h, peer, first).await?;
    let (west, _) = source
        .ordinary_paced_step(
            actor,
            peer.game_session_id,
            crate::gameplay_transport::world_spatial::StepDirection::West,
            None,
            None,
        )
        .await;
    assert_eq!(
        west.disposition,
        crate::gameplay_transport::world_spatial::StepDisposition::Moved
    );
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    let (step, _) = source
        .ordinary_paced_step(
            actor,
            peer.game_session_id,
            crate::gameplay_transport::world_spatial::StepDirection::East,
            None,
            None,
        )
        .await;
    assert_eq!(
        step.disposition,
        crate::gameplay_transport::world_spatial::StepDisposition::Moved
    );
    let joined = source
        .activity_runtimes
        .pending_switch(peer)
        .map_err(|_| "peer source switch")?
        .ok_or("second actual STEP must join existing shared source room")?
        .destination;
    assert_eq!(
        joined.item_fence.ok_or("peer destination")?.runtime_scope,
        first.item_fence.ok_or("first destination")?.runtime_scope
    );
    let shared = source
        .resolve_runtime_view(joined)
        .await
        .map_err(|_| "actual joined native room")?
        .ok_or("actual joined Source owner")?;
    let runtime = shared.runtime.lock().await;
    let after = runtime.companion_snapshot(boss_before.0).map_err(debug)?;
    assert_eq!(
        (after.health, after.maximum_health),
        (boss_before.1, boss_before.2),
        "joining preserves the original live boss identity and health"
    );
    assert_eq!(
        runtime
            .positioned_actor_census()
            .map_err(debug)?
            .into_iter()
            .filter(|(actor, _, session)| session.is_none()
                && runtime
                    .companion_snapshot(*actor)
                    .is_ok_and(|snapshot| snapshot.state.policy.definition_key
                        == "oteryn:creature.rakesh_moonfang"))
            .count(),
        1,
        "one original actual Source boss, not a duplicate/reset room"
    );
    drop(runtime);
    drop(shared);
    let counts:(i64,i64,i64,i64)=sqlx::query_as("SELECT(SELECT count(*) FROM game_activity_handoff_groups),(SELECT count(*) FROM game_activity_handoff_participants),(SELECT count(*) FROM game_summer_portal_joins),(SELECT count(*) FROM game_party_members)")
        .fetch_one(&h.pool).await?;
    assert_eq!(
        counts,
        (1, 2, 1, 0),
        "two independent Characters, one room, no invented party requirement"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT deadline FROM game_boss_room_activities")
            .fetch_one(&h.pool)
            .await?,
        deadline,
        "a later source entry cannot reset the fight lifetime"
    );
    Ok(joined)
}
