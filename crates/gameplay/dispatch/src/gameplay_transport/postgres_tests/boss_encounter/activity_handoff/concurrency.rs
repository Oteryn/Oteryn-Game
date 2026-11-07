//! Administrative storage races before a production handoff writer is enabled.
//! The enclosing scenario keeps a real admitted connection and checks it afterward.
use super::*;

pub(super) async fn prepare_group(h: &Harness, source: &[u8; 16], tag: u8) -> TestResult<[u8; 16]> {
    let handoff = fixture::id(tag);
    let inserted = sqlx::query(
        "INSERT INTO game_activity_handoff_groups \
         (handoff_id,world_id,instance_id,activity_key,template_revision,content_revision, \
          ownership_generation,participant_count,state,prepared_at) \
         SELECT encode($1,'hex')::uuid,s.world_id,encode($3,'hex')::uuid, \
                'oteryn:boss.reservation_fixture','template-1',r.content_revision,1,1,1,0 \
         FROM game_durability_reconnect_sessions s JOIN game_character_roots r \
         ON r.character_id=s.character_id WHERE s.game_session_id=encode($2,'hex')::uuid",
    )
    .bind(handoff.as_slice())
    .bind(source.as_slice())
    .bind(fixture::id(tag + 1).as_slice())
    .execute(&h.pool)
    .await?;
    assert_eq!(inserted.rows_affected(), 1);
    Ok(handoff)
}

pub(super) async fn abort_group(h: &Harness, handoff: &[u8; 16]) -> TestResult {
    let changed = sqlx::query(
        "UPDATE game_activity_handoff_groups SET state=3,decided_at=1 \
         WHERE handoff_id=encode($1,'hex')::uuid AND state=1",
    )
    .bind(handoff.as_slice())
    .execute(&h.pool)
    .await?;
    assert_eq!(changed.rows_affected(), 1);
    Ok(())
}

async fn observed_wait(h: &Harness, waiting: i32, blocking: i32) -> TestResult {
    // Backend identities prove the overlap. A sleep alone is not a race witness.
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let blocked: bool = sqlx::query_scalar("SELECT $2=ANY(pg_blocking_pids($1))")
                .bind(waiting)
                .bind(blocking)
                .fetch_one(&h.pool)
                .await?;
            if blocked {
                return Ok::<(), sqlx::Error>(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|_| "activity preparation did not wait on the expected source transaction")??;
    Ok(())
}

fn assert_source_refusal(error: sqlx::Error, expected: &str) {
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("23514")
    );
    assert!(
        error.to_string().contains(expected),
        "wrong refusal: {error}"
    );
}

pub(super) async fn qualify_source_reservations(
    h: &Harness,
    admitted: AdmittedSession,
) -> TestResult {
    let original = participant_binding(h, admitted).await?;
    let source = admitted.game_session_id.as_bytes();
    // Save a Channel entry independently of the prepared groups. It is removed
    // only if this qualification inserted it; pre-existing placement is retained.
    let inserted_position = sqlx::query(
        "INSERT INTO game_character_last_positions \
         (character_id,world_id,tile_x,tile_y,tile_floor,coordinate_frame_digest, \
          map_revision_digest,server_artifact_digest,character_lease_generation,write_sequence) \
         SELECT character_id,world_id,100,101,7,decode(repeat('11',32),'hex'), \
                decode(repeat('22',32),'hex'),decode(repeat('33',32),'hex'),character_lease_generation,1 \
         FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid \
         ON CONFLICT (character_id) DO NOTHING",
    )
    .bind(source.as_slice())
    .execute(&h.pool)
    .await?
    .rows_affected();
    let first = prepare_group(h, source, 180).await?;
    let second = prepare_group(h, source, 183).await?;
    let mut winner = h.pool.begin().await?;
    sqlx::query("SET LOCAL statement_timeout='15s'")
        .execute(&mut *winner)
        .await?;
    let winner_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *winner)
        .await?;
    sqlx::query(ACTIVITY_PARTICIPANT_INSERT)
        .bind(first.as_slice())
        .bind(source.as_slice())
        .bind(fixture::id(182).as_slice())
        .execute(&mut *winner)
        .await?;
    let mut loser = h.pool.begin().await?;
    sqlx::query("SET LOCAL statement_timeout='15s'")
        .execute(&mut *loser)
        .await?;
    let loser_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *loser)
        .await?;
    let insert = sqlx::query(ACTIVITY_PARTICIPANT_INSERT)
        .bind(second.as_slice())
        .bind(source.as_slice())
        .bind(fixture::id(185).as_slice())
        .execute(&mut *loser);
    let (result, release) = fixture::join_two(insert, async {
        observed_wait(h, loser_pid, winner_pid).await?;
        winner.commit().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    })
    .await;
    release?;
    assert_source_refusal(
        result.expect_err("two concurrent groups reserved the same source"),
        "activity source already has a pending or committed group",
    );
    loser.rollback().await?;
    let counts: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_activity_handoff_participants \
                 WHERE handoff_id=encode($1,'hex')::uuid), \
                (SELECT count(*) FROM game_activity_handoff_participants \
                 WHERE handoff_id=encode($2,'hex')::uuid)",
    )
    .bind(first.as_slice())
    .bind(second.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(counts, (1, 0));
    abort_group(h, &first).await?;
    let retried = sqlx::query(ACTIVITY_PARTICIPANT_INSERT)
        .bind(second.as_slice())
        .bind(source.as_slice())
        .bind(fixture::id(185).as_slice())
        .execute(&h.pool)
        .await?;
    assert_eq!(retried.rows_affected(), 1);
    abort_group(h, &second).await?;

    // The INSERT's source SELECT runs before its trigger takes the source lock.
    // A reconnect can commit while it waits. The old selected generation must
    // not become immutable provenance after that wait, even without a rival group.
    let stale = prepare_group(h, source, 186).await?;
    let mut reconnect = h.pool.begin().await?;
    sqlx::query("SET LOCAL statement_timeout='15s'")
        .execute(&mut *reconnect)
        .await?;
    let reconnect_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *reconnect)
        .await?;
    sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=current_generation+1 WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(source.as_slice()).execute(&mut *reconnect).await?;
    let mut preparing = h.pool.begin().await?;
    sqlx::query("SET LOCAL statement_timeout='15s'")
        .execute(&mut *preparing)
        .await?;
    let preparing_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *preparing)
        .await?;
    let insert = sqlx::query(ACTIVITY_PARTICIPANT_INSERT)
        .bind(stale.as_slice())
        .bind(source.as_slice())
        .bind(fixture::id(188).as_slice())
        .execute(&mut *preparing);
    let (result, release) = fixture::join_two(insert, async {
        observed_wait(h, preparing_pid, reconnect_pid).await?;
        reconnect.commit().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    })
    .await;
    release?;
    assert_source_refusal(
        result.expect_err("preparation retained a generation selected before reconnect"),
        "activity provenance requires its exact source and fresh destination",
    );
    preparing.rollback().await?;
    // Restore only this administrative probe; no production reconnect is claimed.
    sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=current_generation-1 WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(source.as_slice()).execute(&h.pool).await?;
    let retried = sqlx::query(ACTIVITY_PARTICIPANT_INSERT)
        .bind(stale.as_slice())
        .bind(source.as_slice())
        .bind(fixture::id(188).as_slice())
        .execute(&h.pool)
        .await?;
    assert_eq!(retried.rows_affected(), 1);
    abort_group(h, &stale).await?;
    if inserted_position == 1 {
        sqlx::query("DELETE FROM game_character_last_positions WHERE character_id=(SELECT character_id FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid)")
            .bind(source.as_slice()).execute(&h.pool).await?;
    }
    assert_eq!(participant_binding(h, admitted).await?, original);
    eprintln!(
        "Activity provenance: observed concurrent source reservation, retained abort/retry and reconnect race qualified; no Instance admission"
    );
    Ok(())
}
