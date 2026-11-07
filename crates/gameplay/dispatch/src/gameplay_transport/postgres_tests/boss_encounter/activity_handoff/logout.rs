//! Prepared storage is administrative setup; cancellation runs through real logout.
use super::*;
use crate::durability::fresh_admission::{ExpiredLossReleaseV1, FreshAdmissionStore};
use crate::foundation::{AuthenticatedTransportRefV1, GameSessionState};
use oteryn_protocol_oteryn::logout::{COMMAND_TYPE_LOGOUT_INTENT, LogoutResult, decode_logout_result};

async fn group(h: &Harness, handoff: &[u8; 16]) -> TestResult<(i16, Option<i64>)> {
    Ok(sqlx::query_as(
        "SELECT state,decided_at FROM game_activity_handoff_groups WHERE handoff_id=encode($1,'hex')::uuid",
    )
    .bind(handoff.as_slice())
    .fetch_one(&h.pool)
    .await?)
}

async fn membership(h: &Harness, handoff: &[u8; 16]) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT md5(to_jsonb(p)::text) FROM game_activity_handoff_participants p WHERE handoff_id=encode($1,'hex')::uuid LIMIT 1",
    )
    .bind(handoff.as_slice())
    .fetch_one(&h.pool)
    .await?)
}

#[test]
fn boss_activity_handoff_postgres_real_dispatch_logout_aborts_preparation() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "activity logout E2E panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "boss_activity_logout", true, false).await?;
        prepare_current_session(&h).await?;
        eprintln!("Canonical PostgreSQL 18.6: activity source logout compensation");
        let deps = Dependencies::new()?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        owner.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
        super::super::super::track(&owner, &deps)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(owner.offered_capabilities(), &[21])
            .ok_or("logout capability not offered")?;
        let source = admitted.game_session_id.as_bytes();
        sqlx::query("INSERT INTO game_character_last_positions \
            (character_id,world_id,tile_x,tile_y,tile_floor,coordinate_frame_digest, \
             map_revision_digest,server_artifact_digest,character_lease_generation,write_sequence) \
            SELECT character_id,world_id,100,101,7,decode(repeat('11',32),'hex'), \
                   decode(repeat('22',32),'hex'),decode(repeat('33',32),'hex'),character_lease_generation,1 \
            FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(source.as_slice()).execute(&h.pool).await?;
        // Keep an older ABORTED decision and an unrelated empty PREPARED group.
        // The current group declares two participants but only one has prepared:
        // source logout must cancel the group before the admission barrier.
        let old = concurrency::prepare_group(&h, source, 170).await?;
        sqlx::query(ACTIVITY_PARTICIPANT_INSERT).bind(old.as_slice()).bind(source.as_slice())
            .bind(fixture::id(172).as_slice()).execute(&h.pool).await?;
        concurrency::abort_group(&h, &old).await?;
        let unrelated = concurrency::prepare_group(&h, source, 173).await?;
        let pending = fixture::id(176);
        sqlx::query("INSERT INTO game_activity_handoff_groups \
            (handoff_id,world_id,instance_id,activity_key,template_revision,content_revision, \
             ownership_generation,participant_count,state,prepared_at) \
            SELECT encode($2,'hex')::uuid,world_id,encode($3,'hex')::uuid,activity_key, \
                   template_revision,content_revision,ownership_generation,2,1,0 \
            FROM game_activity_handoff_groups WHERE handoff_id=encode($1,'hex')::uuid")
            .bind(unrelated.as_slice()).bind(pending.as_slice()).bind(fixture::id(177).as_slice())
            .execute(&h.pool).await?;
        sqlx::query(ACTIVITY_PARTICIPANT_INSERT).bind(pending.as_slice()).bind(source.as_slice())
            .bind(fixture::id(178).as_slice()).execute(&h.pool).await?;
        let old_state = group(&h, &old).await?;
        let old_member = membership(&h, &old).await?;
        let pending_member = membership(&h, &pending).await?;
        let store = FreshAdmissionStore::from_root(h.root.clone());
        let transport = admitted.controller.ok_or("missing logout controller")?.transport;
        let account: String = sqlx::query_scalar("SELECT account_id::text FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(source.as_slice()).fetch_one(&h.pool).await?;
        let wrong_transport = AuthenticatedTransportRefV1::decode(&fixture::id(179)).map_err(debug)?;
        assert!(matches!(store.release_logout_session(admitted.game_session_id, &account, wrong_transport).await?, ExpiredLossReleaseV1::NotApplicable));
        assert_eq!(group(&h, &pending).await?, (1, None));

        // A competing group decision must not strand half a terminal release.
        // Hold its exact row on a separate backend. NOWAIT must refuse within
        // the native deadline, retaining session, lease, presence and history.
        let original = participant_binding(&h, admitted).await?;
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_admission_lifecycle_receipts")
            .fetch_one(&h.pool).await?;
        let mut deciding = h.pool.begin().await?;
        sqlx::query("SELECT 1 FROM game_activity_handoff_groups WHERE handoff_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(pending.as_slice()).fetch_one(&mut *deciding).await?;
        assert!(tokio::time::timeout(std::time::Duration::from_secs(15),
            store.release_logout_session(admitted.game_session_id, &account, transport)).await?.is_err());
        assert_eq!(participant_binding(&h, admitted).await?, original);
        assert_eq!(membership(&h, &pending).await?, pending_member);
        assert_eq!(group(&h, &pending).await?, (1, None));
        let after_receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_admission_lifecycle_receipts")
            .fetch_one(&h.pool).await?;
        assert_eq!(after_receipts, receipts);
        deciding.rollback().await?;
        let elevated: bool = sqlx::query_scalar("SELECT \
            has_table_privilege('oteryn_game_runtime','game_activity_handoff_groups','UPDATE') OR \
            has_table_privilege('oteryn_game_runtime','game_activity_handoff_participants','UPDATE') OR \
            has_table_privilege('oteryn_game_control','game_activity_handoff_groups','UPDATE') OR \
            has_function_privilege('oteryn_game_runtime','game_activity_handoff_source_ended()','EXECUTE') OR \
            has_function_privilege('oteryn_game_control','game_activity_handoff_source_ended()','EXECUTE')")
            .fetch_one(&h.pool).await?;
        assert!(!elevated, "terminal cancellation expanded direct activity write authority");

        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let driving = async {
            loop {
                let frame = read_frame(&mut client).await?;
                if decode_wire_envelope(&frame).map_err(debug)?.message_type() == MessageType::SnapshotCommit { break; }
            }
            write_frame(&mut client, &encode_client_command(admitted.continuity.connection_generation, &ClientCommandValue {
                command_id: admitted.continuity.next_command_id,
                command_type: COMMAND_TYPE_LOGOUT_INTENT,
                payload: &[],
            }).map_err(debug)?).await?;
            loop {
                let frame = read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.command_id, admitted.continuity.next_command_id);
                    assert_eq!(result.status, CommandStatus::Accepted);
                    assert_eq!(decode_logout_result(result.payload).map_err(debug)?, LogoutResult::Accepted);
                    break;
                }
            }
            client.shutdown().await?;
            Ok::<(), Box<dyn std::error::Error>>(())
        };
        let (ended, result) = tokio::time::timeout(std::time::Duration::from_secs(30), fixture::join_two(
            serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS), driving,
        )).await?;
        result?;
        assert!(matches!(ended, ConnectionEnd::AdmittedThenLoggedOut(_)), "{ended:?}");
        assert_eq!(store.current_session(admitted.game_session_id).await?.session_state(), GameSessionState::Terminal);
        let outcome = group(&h, &pending).await?;
        assert_eq!(outcome.0, 3, "terminal source left its prepared group reserved");
        assert!(outcome.1.is_some_and(|at| at > 0));
        assert_eq!(membership(&h, &pending).await?, pending_member);
        assert_eq!(group(&h, &old).await?, old_state);
        assert_eq!(membership(&h, &old).await?, old_member);
        assert_eq!(group(&h, &unrelated).await?, (1, None));
        // Lost logout acknowledgement/retry must retain the same abort decision.
        assert!(matches!(store.release_logout_session(admitted.game_session_id, &account, transport).await?, ExpiredLossReleaseV1::Terminal));
        assert_eq!(group(&h, &pending).await?, outcome);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
