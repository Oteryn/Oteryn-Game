//! A fresh origin owner discovers and completes an interrupted Instance death.
use super::*;
use crate::durability::character_death::PlayerDeathOccurrence;
use crate::foundation::GameSessionState;
use crate::gameplay_transport::activity_departed::{
    DEPARTED_RETURN_COMMITTED_TEST_BARRIER, DepartedReturnCommittedTestBarrier,
};
use crate::gameplay_transport::fresh_evidence::FreshEvidenceSource;
use std::future::{Future, poll_fn};
use std::task::Poll;

type ReturnFact = (Vec<u8>, i16, Vec<u8>, i32, i32, i16, Option<Vec<u8>>);

pub(super) async fn recover(
    h: &Harness,
    evidence: &FreshEvidenceSource,
    source: CurrentCharacterItemFence,
    occurrence: PlayerDeathOccurrence,
    origin: RuntimeScopeRefV1,
    temple: LastPositionCell,
    revision: CharacterRevision,
) -> TestResult {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    let deps = Dependencies::new()?;
    // This constructor's original Channel session actually ended on entry.
    // Remove that exact terminal slot; no Instance actor, directory entry or
    // transfer/recovery token from the departed process exists in this owner.
    assert_eq!(
        store
            .current_session_at(deps.admitted.game_session_id)
            .await?
            .0
            .session_state(),
        GameSessionState::Terminal
    );
    deps.runtime
        .lock()
        .await
        .remove_terminal_session(deps.admitted.game_session_id, deps.actor)
        .map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    owner.evidence = evidence;
    assert_eq!(owner.runtime.lock().await.binding().runtime_scope(), origin);
    assert!(
        owner
            .activity_runtimes
            .activity_owners()
            .map_err(|_| "empty activity directory")?
            .is_empty()
    );
    assert!(
        owner
            .runtime
            .lock()
            .await
            .activity_participant_sessions()
            .map_err(debug)?
            .is_empty()
    );

    let pending_before = pending(h).await?;
    let death_before: String = sqlx::query_scalar("SELECT to_jsonb(d)::text FROM game_character_death_receipts d WHERE death_occurrence_id=encode($1,'hex')::uuid")
        .bind(occurrence.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    let first_drop: String = sqlx::query_scalar("SELECT to_jsonb(r)::text FROM game_player_death_item_records r WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(75).as_slice()).fetch_one(&h.pool).await?;
    assert!(!items_complete(h, occurrence).await?);
    assert_eq!(counts(h).await?, (1, 0, 0, 1));
    assert_eq!(
        h.root
            .read_item_loop_ground_for_scope(source.runtime_scope)
            .await?
            .len(),
        1
    );
    let partial = snapshot(h).await?;
    owner
        .activity_lifecycle_pass()
        .await
        .map_err(|_| "live assignment lifecycle pass")?;
    assert_eq!(
        snapshot(h).await?,
        partial,
        "an empty registry does not prove the actual Instance owner lost"
    );

    // The existing control writer revokes only this real Instance. From this
    // point every recovery action below is the production origin lifecycle;
    // this case never obtains or invokes a low-level departed-death proof.
    revoke_instance(h, source.runtime_scope).await?;
    let barrier = std::sync::Arc::new(DepartedReturnCommittedTestBarrier::default());
    let mut pass = std::pin::pin!(
        DEPARTED_RETURN_COMMITTED_TEST_BARRIER
            .scope(barrier.clone(), owner.activity_lifecycle_pass(),)
    );
    let mut reached = std::pin::pin!(barrier.entered.notified());
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        poll_fn(|context| {
            if reached.as_mut().poll(context).is_ready() {
                return Poll::Ready(Ok(()));
            }
            if let Poll::Ready(result) = pass.as_mut().poll(context) {
                return Poll::Ready(Err(format!(
                    "partial-death lifecycle ended before return COMMIT: {result:?}"
                )));
            }
            Poll::Pending
        }),
    )
    .await
    .map_err(|_| "partial-death return COMMIT barrier timeout")??;

    // The observation boundary is after real return COMMIT and before the
    // lifecycle reaches Ground retirement. Both original selected losses must
    // already be complete; the pending temple obligation is still untouched.
    assert!(items_complete(h, occurrence).await?);
    assert_eq!(counts(h).await?, (2, 1, 0, 2));
    assert_eq!(pending(h).await?, pending_before);
    assert_eq!(sqlx::query_scalar::<_, String>("SELECT to_jsonb(d)::text FROM game_character_death_receipts d WHERE death_occurrence_id=encode($1,'hex')::uuid")
        .bind(occurrence.as_bytes().as_slice()).fetch_one(&h.pool).await?, death_before);
    assert_eq!(sqlx::query_scalar::<_, String>("SELECT to_jsonb(r)::text FROM game_player_death_item_records r WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(75).as_slice()).fetch_one(&h.pool).await?, first_drop, "the already committed first loss is replayed, not reissued");
    let selected: Vec<Vec<u8>> = sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_player_death_item_records ORDER BY item_instance_id")
        .fetch_all(&h.pool).await?;
    assert_eq!(selected, vec![id(70).to_vec(), id(75).to_vec()]);
    assert_eq!(
        h.root
            .read_item_loop_ground_for_scope(source.runtime_scope)
            .await?
            .len(),
        2
    );
    assert!(
        h.root
            .read_item_loop_ground_for_scope(origin)
            .await?
            .is_empty()
    );
    let child: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(parent_item_instance_id) FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        child.as_slice(),
        id(70).as_slice(),
        "the selected death bag still contains its original child before retirement"
    );
    let returned: ReturnFact = sqlx::query_as("SELECT uuid_send(destination_game_session_id),reason,uuid_send(death_occurrence_id),return_x,return_y,return_floor,destination_transport_ref FROM game_activity_runtime_returns WHERE source_game_session_id=encode($1,'hex')::uuid")
        .bind(source.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    let destination = GameSessionId::decode(&returned.0).map_err(debug)?;
    assert_eq!(
        returned.1, 5,
        "the retained return is Death, not an ordinary exit or generic owner loss"
    );
    assert_eq!(returned.2.as_slice(), occurrence.as_bytes().as_slice());
    assert_eq!(
        (returned.3, returned.4, returned.5),
        (temple.x, temple.y, temple.floor)
    );
    assert!(
        returned.6.is_none(),
        "process recovery does not create an authenticated transport"
    );
    let offline = store.current_session_at(destination).await?.0;
    assert_eq!(offline.session_state(), GameSessionState::Reconnectable);
    assert_eq!(offline.current_runtime_scope(), origin);
    assert_eq!(
        offline.current_character_lease().generation(),
        source.character_lease_generation + 1
    );
    assert!(offline.current_transport().is_none());
    assert!(
        owner
            .runtime
            .lock()
            .await
            .activity_participant_sessions()
            .map_err(debug)?
            .is_empty()
    );
    assert!(
        owner
            .activity_runtimes
            .activity_owners()
            .map_err(|_| "no restored Instance owner")?
            .is_empty()
    );

    barrier.proceed.notify_one();
    tokio::time::timeout(std::time::Duration::from_secs(30), pass)
        .await
        .map_err(|_| "partial-death lifecycle completion timeout")?
        .map_err(|_| "partial-death lifecycle completion")?;
    // Two Ground roots (one with a child) need bounded existing owner steps.
    // Subsequent passes also independently discover and release the offline
    // successor. No native actor or client credential is restored.
    for _ in 0..3 {
        owner
            .activity_lifecycle_pass()
            .await
            .map_err(|_| "partial-death terminal/retirement pass")?;
    }
    assert_eq!(
        store
            .current_session_at(destination)
            .await?
            .0
            .session_state(),
        GameSessionState::Terminal
    );
    assert_eq!(
        store
            .current_session_at(source.game_session_id)
            .await?
            .0
            .session_state(),
        GameSessionState::Terminal
    );
    assert_eq!(counts(h).await?, (2, 1, 3, 5));
    assert!(
        h.root
            .read_item_loop_ground_for_scope(source.runtime_scope)
            .await?
            .is_empty()
    );
    assert!(
        h.root
            .read_item_loop_ground_for_scope(origin)
            .await?
            .is_empty()
    );
    let live: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE item_instance_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid) AND lifecycle=1")
        .bind(id(70).as_slice()).bind(id(71).as_slice()).bind(id(75).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(live, 0);
    assert_eq!(pending(h).await?, pending_before);
    assert!(
        owner
            .runtime
            .lock()
            .await
            .activity_participant_sessions()
            .map_err(debug)?
            .is_empty()
    );
    assert!(
        owner
            .activity_runtimes
            .activity_owners()
            .map_err(|_| "empty retired directory")?
            .is_empty()
    );

    // A separate, genuinely authenticated fresh Store admission may now own
    // the Character. It inherits the exact pending death; only the ordinary
    // accepted Channel temple-placement owner may later consume that row.
    let fresh =
        super::super::activity_departed::fresh_after_release(h, source.character_id, origin)
            .await?;
    assert_eq!(fresh.session_state(), GameSessionState::Active);
    assert_eq!(fresh.current_runtime_scope(), origin);
    assert_eq!(
        fresh.current_character_lease().generation(),
        source.character_lease_generation + 2
    );
    let fresh_fence = item_fence_of(fresh).ok_or("actual fresh Channel fence")?;
    let obligation = h
        .root
        .pending_respawn(&authority, &h.node, gameplay(fresh_fence, revision))
        .await
        .map_err(debug)?
        .ok_or("ordinary fresh admission retains pending respawn")?;
    assert_eq!(obligation.occurrence, occurrence);
    assert_eq!(obligation.respawn_position, cell_bytes(temple));
    assert_eq!(pending(h).await?, pending_before);
    let final_state = snapshot(h).await?;
    owner
        .activity_lifecycle_pass()
        .await
        .map_err(|_| "partial-death replay pass")?;
    assert_eq!(
        snapshot(h).await?,
        final_state,
        "later lifecycle cannot duplicate losses, returns, retirement or consume pending respawn"
    );
    assert_eq!(
        store
            .current_session_at(fresh.current_game_session_id())
            .await?
            .0,
        fresh
    );
    eprintln!("ACTIVITY_PLAYER_DEATH_DEPARTED_PROCESS POSTGRES_CASE_COMPLETED");
    Ok(())
}

async fn items_complete(h: &Harness, occurrence: PlayerDeathOccurrence) -> TestResult<bool> {
    Ok(
        sqlx::query_scalar("SELECT game_player_death_items_complete(encode($1,'hex')::uuid)")
            .bind(occurrence.as_bytes().as_slice())
            .fetch_one(&h.pool)
            .await?,
    )
}

async fn pending(h: &Harness) -> TestResult<String> {
    Ok(
        sqlx::query_scalar("SELECT to_jsonb(p)::text FROM game_character_pending_respawns p")
            .fetch_one(&h.pool)
            .await?,
    )
}

async fn counts(h: &Harness) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_player_death_item_records),(SELECT count(*) FROM game_activity_runtime_returns),(SELECT count(*) FROM game_item_decay_retire_receipts),(SELECT count(*) FROM game_item_audit_outbox)")
        .fetch_one(&h.pool).await?)
}

async fn snapshot(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar(r#"SELECT jsonb_build_object(
        'death',(SELECT jsonb_agg(to_jsonb(r) ORDER BY death_occurrence_id) FROM game_character_death_receipts r),
        'pending',(SELECT jsonb_agg(to_jsonb(p) ORDER BY character_id) FROM game_character_pending_respawns p),
        'loss',(SELECT jsonb_agg(to_jsonb(r) ORDER BY item_instance_id) FROM game_player_death_item_records r),
        'returns',(SELECT jsonb_agg(to_jsonb(r) ORDER BY return_id) FROM game_activity_runtime_returns r),
        'ground',(SELECT jsonb_agg(to_jsonb(g) ORDER BY item_instance_id) FROM game_item_ground_locations g),
        'items',(SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i),
        'entries',(SELECT jsonb_agg(to_jsonb(e) ORDER BY item_instance_id) FROM game_item_container_entries e),
        'retirement',(SELECT jsonb_agg(to_jsonb(r) ORDER BY item_instance_id) FROM game_item_decay_retire_receipts r),
        'audits',(SELECT jsonb_agg(to_jsonb(a) ORDER BY event_id) FROM game_item_audit_outbox a)
    )::text"#).fetch_one(&h.pool).await?)
}
