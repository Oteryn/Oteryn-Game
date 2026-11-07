// Migration-0265 storage qualification on an already admitted real connection.
// Administrative preparations cannot admit an Instance or substitute for a writer.
use super::*;

#[path = "activity_handoff/concurrency.rs"]
mod concurrency;

#[path = "activity_handoff/logout.rs"]
mod logout;

const ACTIVITY_PARTICIPANT_INSERT: &str = "\
    INSERT INTO game_activity_handoff_participants \
    (handoff_id,world_id,character_id,account_id,origin_channel_id, \
     source_game_session_id,destination_game_session_id, \
     source_connection_generation,source_lease_generation,source_ownership_generation, \
     profile_revision,ruleset_revision,content_revision,starter_template_revision, \
     entry_x,entry_y,entry_floor,coordinate_frame_digest,map_revision_digest,server_artifact_digest) \
    SELECT encode($1,'hex')::uuid,s.world_id,s.character_id,s.account_id,s.runtime_scope_channel_id, \
           s.game_session_id,encode($3,'hex')::uuid, \
           s.current_generation,s.character_lease_generation,s.scope_ownership_generation, \
           r.profile_revision,r.ruleset_revision,r.content_revision,r.starter_template_revision, \
           p.tile_x,p.tile_y,p.tile_floor,p.coordinate_frame_digest,p.map_revision_digest,p.server_artifact_digest \
    FROM game_durability_reconnect_sessions s \
    JOIN game_character_roots r ON r.character_id=s.character_id \
    JOIN game_character_last_positions p ON p.character_id=s.character_id \
    WHERE s.game_session_id=encode($2,'hex')::uuid";

async fn activity_provenance_rejection(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    statement: &str,
    expected: &str,
    handoff: &[u8; 16],
    source: &[u8; 16],
    destination: &[u8; 16],
) -> TestResult {
    sqlx::query("SAVEPOINT activity_provenance_probe")
        .execute(&mut **tx)
        .await?;
    // The SQL and its substitutions are fixed test literals; identities are binds.
    let error = sqlx::query(sqlx::AssertSqlSafe(statement))
        .bind(handoff.as_slice())
        .bind(source.as_slice())
        .bind(destination.as_slice())
        .execute(&mut **tx)
        .await
        .expect_err("unproven activity provenance accepted");
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("23514")
    );
    assert!(
        error.to_string().contains(expected),
        "wrong provenance refusal: {error}"
    );
    sqlx::query("ROLLBACK TO SAVEPOINT activity_provenance_probe")
        .execute(&mut **tx)
        .await?;
    sqlx::query("RELEASE SAVEPOINT activity_provenance_probe")
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(super) async fn qualify_activity_provenance(
    h: &Harness,
    admitted: AdmittedSession,
    activity: crate::foundation::RuntimeScopeRefV1,
) -> TestResult {
    let crate::foundation::RuntimeScopeRefV1::Instance { instance_id, .. } = activity else {
        return Err("provenance requires activity identity".into());
    };
    let original = participant_binding(h, admitted).await?;
    let handoff = fixture::id(92);
    let destination = fixture::id(93);
    let source = admitted.game_session_id.as_bytes();
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL statement_timeout='5s'")
        .execute(&mut *tx)
        .await?;
    // The fixture's saved entry is a Channel cell, not an admitted arena position.
    sqlx::query("INSERT INTO game_character_last_positions \
        (character_id,world_id,tile_x,tile_y,tile_floor,coordinate_frame_digest, \
         map_revision_digest,server_artifact_digest,character_lease_generation,write_sequence) \
        SELECT character_id,world_id,100,101,7,decode(repeat('11',32),'hex'), \
               decode(repeat('22',32),'hex'),decode(repeat('33',32),'hex'),character_lease_generation,1 \
        FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid \
        ON CONFLICT (character_id) DO NOTHING")
        .bind(source.as_slice()).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO game_activity_handoff_groups \
        (handoff_id,world_id,instance_id,activity_key,template_revision,content_revision, \
         ownership_generation,participant_count,state,prepared_at) \
        SELECT encode($1,'hex')::uuid,s.world_id,encode($3,'hex')::uuid, \
               'oteryn:boss.provenance_fixture','template-1',r.content_revision,1,2,1,0 \
        FROM game_durability_reconnect_sessions s JOIN game_character_roots r \
        ON r.character_id=s.character_id WHERE s.game_session_id=encode($2,'hex')::uuid",
    )
    .bind(handoff.as_slice())
    .bind(source.as_slice())
    .bind(instance_id.as_slice())
    .execute(&mut *tx)
    .await?;

    // Change one provenance fact at a time, retaining the exact source elsewhere.
    for (old, changed) in [
        ("s.world_id,", "encode($3,'hex')::uuid,"),
        ("s.character_id,", "encode($3,'hex')::uuid,"),
        ("s.account_id,", "encode($3,'hex')::uuid,"),
        ("s.runtime_scope_channel_id,", "encode($3,'hex')::uuid,"),
        ("s.current_generation,", "s.current_generation+1,"),
        (
            "s.character_lease_generation,",
            "s.character_lease_generation+1,",
        ),
        (
            "s.scope_ownership_generation,",
            "s.scope_ownership_generation+1,",
        ),
        ("r.profile_revision,", "'wrong-profile',"),
        ("r.ruleset_revision,", "'wrong-ruleset',"),
        ("r.content_revision,", "'wrong-content',"),
        ("r.starter_template_revision,", "'wrong-starter',"),
        ("p.tile_x,", "p.tile_x+1,"),
        ("p.tile_y,", "p.tile_y+1,"),
        ("p.tile_floor,", "p.tile_floor+1,"),
        (
            "p.coordinate_frame_digest,",
            "decode(repeat('44',32),'hex'),",
        ),
        ("p.map_revision_digest,", "decode(repeat('44',32),'hex'),"),
        (
            "p.server_artifact_digest ",
            "decode(repeat('44',32),'hex') ",
        ),
        ("encode($3,'hex')::uuid,", "s.game_session_id,"),
    ] {
        assert!(ACTIVITY_PARTICIPANT_INSERT.contains(old));
        activity_provenance_rejection(
            &mut tx,
            &ACTIVITY_PARTICIPANT_INSERT.replace(old, changed),
            "activity provenance requires its exact source and fresh destination",
            &handoff,
            source,
            &destination,
        )
        .await?;
    }
    sqlx::query(ACTIVITY_PARTICIPANT_INSERT)
        .bind(handoff.as_slice())
        .bind(source.as_slice())
        .bind(destination.as_slice())
        .execute(&mut *tx)
        .await?;
    for statement in [
        "UPDATE game_activity_handoff_participants SET origin_channel_id=encode($3,'hex')::uuid WHERE handoff_id=encode($1,'hex')::uuid AND source_game_session_id=encode($2,'hex')::uuid",
        "DELETE FROM game_activity_handoff_participants WHERE handoff_id=encode($1,'hex')::uuid AND source_game_session_id=encode($2,'hex')::uuid AND destination_game_session_id=encode($3,'hex')::uuid",
    ] {
        activity_provenance_rejection(
            &mut tx,
            statement,
            "activity participant provenance is immutable",
            &handoff,
            source,
            &destination,
        )
        .await?;
    }
    // Force the deferred group barrier inside the probe savepoint: one source
    // snapshot cannot commit a declared two-person group, even with readiness.
    sqlx::query("SAVEPOINT activity_group_commit")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_activity_handoff_groups SET state=2,decided_at=1 WHERE handoff_id=encode($1,'hex')::uuid")
        .bind(handoff.as_slice()).execute(&mut *tx).await?;
    let error = sqlx::query("SET CONSTRAINTS game_activity_handoff_group_proven IMMEDIATE")
        .execute(&mut *tx)
        .await
        .expect_err("partial activity group committed");
    assert!(
        error
            .to_string()
            .contains("activity commit requires the complete group")
    );
    sqlx::query("ROLLBACK TO SAVEPOINT activity_group_commit")
        .execute(&mut *tx)
        .await?;
    sqlx::query("RELEASE SAVEPOINT activity_group_commit")
        .execute(&mut *tx)
        .await?;
    // A second group cannot reserve the same live source while the first is
    // PREPARED. Aborting the first retains its history and permits a fresh retry.
    let retry = fixture::id(94);
    let retry_destination = fixture::id(95);
    sqlx::query(
        "INSERT INTO game_activity_handoff_groups \
        (handoff_id,world_id,instance_id,activity_key,template_revision,content_revision, \
         ownership_generation,participant_count,state,prepared_at) \
        SELECT encode($2,'hex')::uuid,world_id,encode($3,'hex')::uuid,activity_key, \
               template_revision,content_revision,ownership_generation,1,1,0 \
        FROM game_activity_handoff_groups WHERE handoff_id=encode($1,'hex')::uuid",
    )
    .bind(handoff.as_slice())
    .bind(retry.as_slice())
    .bind(fixture::id(96).as_slice())
    .execute(&mut *tx)
    .await?;
    activity_provenance_rejection(
        &mut tx,
        ACTIVITY_PARTICIPANT_INSERT,
        "activity source already has a pending or committed group",
        &retry,
        source,
        &retry_destination,
    )
    .await?;
    // Neither preparation nor aborted provenance may weaken 0247's guard.
    sqlx::query("UPDATE game_activity_handoff_groups SET state=3,decided_at=1 WHERE handoff_id=encode($1,'hex')::uuid")
        .bind(handoff.as_slice()).execute(&mut *tx).await?;
    activity_provenance_rejection(&mut tx,
        "UPDATE game_durability_reconnect_sessions SET runtime_scope_kind=2,runtime_scope_channel_id=NULL,runtime_scope_instance_id=encode($3,'hex')::uuid WHERE game_session_id=encode($2,'hex')::uuid AND EXISTS (SELECT 1 FROM game_activity_handoff_groups WHERE handoff_id=encode($1,'hex')::uuid)",
        "activity session requires a committed activity handoff", &handoff, source, &instance_id).await?;
    activity_provenance_rejection(&mut tx,
        "UPDATE game_activity_handoff_groups SET state=1,decided_at=NULL WHERE handoff_id=encode($1,'hex')::uuid AND world_id=(SELECT world_id FROM game_durability_reconnect_sessions WHERE game_session_id=encode($2,'hex')::uuid) AND instance_id<>encode($3,'hex')::uuid",
        "activity group only prepares then commits or aborts once", &handoff, source, &destination).await?;
    sqlx::query(ACTIVITY_PARTICIPANT_INSERT)
        .bind(retry.as_slice())
        .bind(source.as_slice())
        .bind(retry_destination.as_slice())
        .execute(&mut *tx)
        .await?;
    activity_provenance_rejection(
        &mut tx,
        ACTIVITY_PARTICIPANT_INSERT,
        "activity group membership is full",
        &retry,
        source,
        &destination,
    )
    .await?;
    // A full membership list alone also cannot stand in for the ownership flow.
    sqlx::query("SAVEPOINT activity_complete_group")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_activity_handoff_groups SET state=2,decided_at=1 WHERE handoff_id=encode($1,'hex')::uuid")
        .bind(retry.as_slice()).execute(&mut *tx).await?;
    let error = sqlx::query("SET CONSTRAINTS game_activity_handoff_group_proven IMMEDIATE")
        .execute(&mut *tx)
        .await
        .expect_err("activity group without destinations committed");
    assert!(
        error
            .to_string()
            .contains("activity commit requires every terminal source and bound destination")
    );
    sqlx::query("ROLLBACK TO SAVEPOINT activity_complete_group")
        .execute(&mut *tx)
        .await?;
    sqlx::query("RELEASE SAVEPOINT activity_complete_group")
        .execute(&mut *tx)
        .await?;
    tx.rollback().await?;
    assert_eq!(participant_binding(h, admitted).await?, original);
    let writable: bool = sqlx::query_scalar(
        "SELECT has_table_privilege('oteryn_game_runtime', \
        'game_activity_handoff_groups','INSERT') AND has_table_privilege('oteryn_game_runtime', \
        'game_activity_handoff_participants','INSERT')",
    )
    .fetch_one(&h.pool)
    .await?;
    // W02 coordinator ANSWER A installs the full grouped handoff writer in
    // 0628. INSERT admits only its proven immutable provenance; it still cannot
    // allocate a scope or erase either history. All probes above remain refused.
    assert!(
        writable,
        "qualified activity writer lacks provenance INSERT"
    );
    for table in [
        "game_activity_handoff_groups",
        "game_activity_handoff_participants",
        "game_control_activity_scope_grants",
        "game_runtime_scope_assignments",
    ] {
        let destructive: bool = sqlx::query_scalar(
            "SELECT has_table_privilege('oteryn_game_runtime',$1,'DELETE,TRUNCATE,TRIGGER')",
        )
        .bind(table)
        .fetch_one(&h.pool)
        .await?;
        assert!(
            !destructive,
            "runtime can destroy activity authority/history: {table}"
        );
    }
    let grants: bool = sqlx::query_scalar("SELECT has_table_privilege('oteryn_game_runtime','game_control_activity_scope_grants','INSERT,UPDATE')")
        .fetch_one(&h.pool).await?;
    assert!(!grants, "runtime must not allocate its own activity scopes");
    concurrency::qualify_source_reservations(h, admitted).await?;
    Ok(())
}
