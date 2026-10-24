#[test]
fn runtime_terminal_replacement_rejects_consumed_fenced_continuity_before_mutation() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) =
            migrated_database("replacement_consumed_fenced_continuity").await?;
        let now = unix_now().map_err(|_| "invalid clock")?;
        seed_current_actor_anchor(&database_url, 10, 10, now).await?;

        let mut setup = PgConnection::connect(&database_url).await?;
        sqlx::query(
            "WITH activation AS (SELECT clock_timestamp() AS activated_at) \
                 UPDATE game_durability_control_loss_continuity AS continuity \
                 SET protection_entitlement_state = 2, protection_fenced_generation = 42, \
                     protection_activated_at = activation.activated_at, \
                     protection_expires_at = activation.activated_at + INTERVAL '4 seconds', \
                     protection_rearm_state = 2, \
                     protection_rearm_deadline = activation.activated_at + INTERVAL '8 seconds' \
                 FROM activation \
                 WHERE continuity.character_id = encode($1, 'hex')::uuid \
                   AND continuity.control_loss_epoch = 3",
        )
        .bind(uuid_v7(11).as_slice())
        .execute(&mut setup)
        .await?;
        setup.close().await?;

        let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
        let candidate = record_with_protection(
            record(20, 11, 1, 0xaf, 3, 7, 10, now).map_err(|_| "candidate record")?,
            ProtectionEntitlementV1::fenced(42).map_err(|_| "fenced protection")?,
        )
        .map_err(|_| "candidate protection")?;
        let request = v2_request(candidate, 10, 10).map_err(|_| "authorization")?;
        let prepare_result = journal.prepare_v2(&request).await;

        let mut observer = PgConnection::connect(&database_url).await?;
        let predecessor_state: i16 = sqlx::query_scalar(
            "SELECT session_state FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
        )
        .bind(uuid_v7(10).as_slice())
        .fetch_one(&mut observer)
        .await?;
        let candidate_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
        )
        .bind(uuid_v7(20).as_slice())
        .fetch_one(&mut observer)
        .await?;
        let receipt_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM game_durability_session_replacements \
                 WHERE predecessor_game_session_id = encode($1, 'hex')::uuid \
                   AND candidate_game_session_id = encode($2, 'hex')::uuid",
        )
        .bind(uuid_v7(10).as_slice())
        .bind(uuid_v7(20).as_slice())
        .fetch_one(&mut observer)
        .await?;
        observer.close().await?;
        drop(journal);
        database.cleanup().await?;

        assert!(matches!(
            prepare_result,
            Err(DurabilityError::InvalidStoredState)
        ));
        assert_eq!(predecessor_state, 1);
        assert_eq!(candidate_count, 0);
        assert_eq!(receipt_count, 0);
        Ok(())
    })
}

#[test]
fn runtime_reconciliation_requires_exact_replacement_receipt_binding() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) = migrated_database("reconcile_receipt_binding").await?;
        let now = unix_now().map_err(|_| "invalid clock")?;
        seed_current_actor_anchor(&database_url, 10, 9, now).await?;
        let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
        let candidate = record(20, 11, 1, 0xa1, 3, 7, 10, now).map_err(|_| "candidate record")?;
        let request = v2_request(candidate, 10, 10).map_err(|_| "authorization")?;

        assert_eq!(
            journal.prepare_v2(&request).await?,
            ReconnectPrepareDispositionV2::Prepared
        );
        drop(journal);

        let recovered = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
        assert_eq!(
            recovered.reconcile_v2(&request).await?,
            ReconnectDurableReconciliationSnapshotV2::new(
                request.record().clone(),
                ReconnectDurableOutcomeV2::Prepared,
            )
        );

        let conflicting = v2_request(request.record().clone(), 30, 10)
            .map_err(|_| "conflicting authorization")?;
        assert!(matches!(
            recovered.reconcile_v2(&conflicting).await,
            Err(DurabilityError::InvalidStoredState)
        ));

        database.cleanup().await?;
        Ok(())
    })
}

#[test]
fn runtime_terminal_replacement_rejects_scope_ahead_without_candidate_mutation() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) = migrated_database("scope_ahead").await?;
        let now = unix_now().map_err(|_| "invalid clock")?;
        seed_current_actor_anchor(&database_url, 10, 11, now).await?;
        let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
        let candidate = record(20, 11, 1, 0xa2, 3, 7, 10, now).map_err(|_| "candidate record")?;
        let request = v2_request(candidate, 10, 10).map_err(|_| "authorization")?;
        let result = journal.prepare_v2(&request).await;
        assert!(failed_closed_prepare(&result));
        let mut connection = PgConnection::connect(&database_url).await?;
        let candidate_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
        )
        .bind(uuid_v7(20).as_slice())
        .fetch_one(&mut connection)
        .await?;
        assert_eq!(candidate_count, 0);
        let receipt_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM game_durability_session_replacements")
                .fetch_one(&mut connection)
                .await?;
        assert_eq!(receipt_count, 0);
        connection.close().await?;
        database.cleanup().await?;
        Ok(())
    })
}

#[test]
fn runtime_terminal_replacement_rejects_mismatched_predecessor_without_candidate_mutation()
-> TestResult {
    run_postgres_test(async {
        let (database, database_url) = migrated_database("mismatched_predecessor").await?;
        let now = unix_now().map_err(|_| "invalid clock")?;
        seed_current_actor_anchor(&database_url, 30, 10, now).await?;
        let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
        let candidate = record(20, 11, 1, 0xa3, 3, 7, 10, now).map_err(|_| "candidate record")?;
        let request = v2_request(candidate, 10, 10).map_err(|_| "authorization")?;
        let result = journal.prepare_v2(&request).await;
        assert!(failed_closed_prepare(&result));
        let mut connection = PgConnection::connect(&database_url).await?;
        let candidate_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
        )
        .bind(uuid_v7(20).as_slice())
        .fetch_one(&mut connection)
        .await?;
        assert_eq!(candidate_count, 0);
        connection.close().await?;
        database.cleanup().await?;
        Ok(())
    })
}

#[test]
fn runtime_terminal_replacement_fences_prepared_predecessor_and_late_commit() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) = migrated_database("late_commit_fence").await?;
        let now = unix_now().map_err(|_| "invalid clock")?;
        let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
        let predecessor_record =
            record(10, 11, 1, 0xa4, 3, 7, 10, now).map_err(|_| "predecessor record")?;
        let (mut predecessor_flow, predecessor_prepare) =
            ReconnectDurabilityFlowV1::begin(predecessor_record);
        assert_eq!(
            journal.prepare(&predecessor_prepare).await?,
            ReconnectPrepareDispositionV1::Prepared
        );
        predecessor_flow
            .accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                &predecessor_prepare,
                ReconnectPrepareDispositionV1::Prepared,
            ))
            .map_err(|_| "predecessor completion")?;
        let current = exact_current_authority(predecessor_prepare.record(), now)
            .map_err(|_| "predecessor current authority")?;
        let predecessor_commit = predecessor_flow
            .authorize_commit(current, now)
            .map_err(|_| "predecessor commit authorization")?;

        let candidate = record(20, 11, 2, 0xa5, 3, 7, 10, now).map_err(|_| "candidate record")?;
        let request = v2_request(candidate, 10, 10).map_err(|_| "authorization")?;
        assert_eq!(
            journal.prepare_v2(&request).await?,
            ReconnectPrepareDispositionV2::Prepared
        );

        let mut connection = PgConnection::connect(&database_url).await?;
        let predecessor_attempt_state: i16 = sqlx::query_scalar(
            "SELECT state FROM game_durability_reconnect_attempts \
                 WHERE game_session_id = encode($1, 'hex')::uuid AND reconnect_attempt_ref = $2",
        )
        .bind(uuid_v7(10).as_slice())
        .bind(1_u64.to_be_bytes().as_slice())
        .fetch_one(&mut connection)
        .await?;
        assert_eq!(predecessor_attempt_state, 4);
        let predecessor_state: i16 = sqlx::query_scalar(
            "SELECT session_state FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
        )
        .bind(uuid_v7(10).as_slice())
        .fetch_one(&mut connection)
        .await?;
        assert_eq!(predecessor_state, 3);
        let prepared_ref: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT prepared_attempt_ref FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
        )
        .bind(uuid_v7(10).as_slice())
        .fetch_one(&mut connection)
        .await?;
        assert!(prepared_ref.is_none());
        connection.close().await?;
        let late_commit = journal.commit(&predecessor_commit).await;
        assert!(matches!(
            late_commit,
            Ok(ReconnectCommitDispositionV1::RejectedStaleAuthority)
                | Ok(ReconnectCommitDispositionV1::ExistingTerminal)
                | Err(DurabilityError::InvalidStoredState)
        ));
        database.cleanup().await?;
        Ok(())
    })
}

#[test]
fn runtime_terminal_replacement_mid_transaction_failure_rolls_back_everything() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) = migrated_database("replacement_rollback").await?;
        let now = unix_now().map_err(|_| "invalid clock")?;
        seed_current_actor_anchor(&database_url, 10, 9, now).await?;
        let mut connection = PgConnection::connect(&database_url).await?;
        connection
            .execute(sqlx::query(
                "CREATE FUNCTION fail_after_replacement_receipt() RETURNS trigger \
                     LANGUAGE plpgsql AS $$ \
                     BEGIN \
                       IF EXISTS (SELECT 1 FROM game_durability_session_replacements) THEN \
                         RAISE EXCEPTION 'forced replacement rollback'; \
                       END IF; \
                       RETURN NEW; \
                     END $$",
            ))
            .await?;
        connection
            .execute(sqlx::query(
                "CREATE TRIGGER fail_candidate_attempt_after_replacement \
                     BEFORE INSERT ON game_durability_reconnect_attempts \
                     FOR EACH ROW EXECUTE FUNCTION fail_after_replacement_receipt()",
            ))
            .await?;
        connection.close().await?;

        let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
        let candidate = record(20, 11, 1, 0xa6, 3, 7, 10, now).map_err(|_| "candidate record")?;
        let request = v2_request(candidate, 10, 10).map_err(|_| "authorization")?;
        assert!(matches!(
            journal.prepare_v2(&request).await,
            Err(DurabilityError::Database(_))
        ));

        let mut connection = PgConnection::connect(&database_url).await?;
        let predecessor: (String, i16) = sqlx::query_as(
            "SELECT scope_ownership_generation::text, session_state \
                 FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
        )
        .bind(uuid_v7(10).as_slice())
        .fetch_one(&mut connection)
        .await?;
        assert_eq!(predecessor, ("9".to_owned(), 1));
        let receipt_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM game_durability_session_replacements")
                .fetch_one(&mut connection)
                .await?;
        assert_eq!(receipt_count, 0);
        let candidate_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
        )
        .bind(uuid_v7(20).as_slice())
        .fetch_one(&mut connection)
        .await?;
        assert_eq!(candidate_count, 0);
        connection.close().await?;
        database.cleanup().await?;
        Ok(())
    })
}

#[test]
fn runtime_collision_replay_and_restart_reconciliation_preserve_collision_reason() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) = migrated_database("typed_collision_restart").await?;
        let now = unix_now().map_err(|_| "invalid clock")?;
        let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
        let first = record(50, 51, 1, 0xb1, 3, 7, 10, now).map_err(|_| "first record")?;
        let (_first_flow, first_request) = ReconnectDurabilityFlowV1::begin(first);
        assert_eq!(
            journal.prepare(&first_request).await?,
            ReconnectPrepareDispositionV1::Prepared
        );

        let colliding = record(60, 61, 1, 0xb1, 3, 7, 10, now).map_err(|_| "collision record")?;
        let (_flow, collision_request) = ReconnectDurabilityFlowV2::begin(colliding, None);
        assert_eq!(
            journal.prepare_v2(&collision_request).await?,
            ReconnectPrepareDispositionV2::RejectedTransportRefCollision
        );
        drop(journal);

        let recovered = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
        assert_eq!(
            recovered.prepare_v2(&collision_request).await?,
            ReconnectPrepareDispositionV2::ExistingTerminal {
                disposition: ReconnectDurableTerminalDispositionV1::TransportRefCollision,
            }
        );
        assert_eq!(
            recovered.reconcile_v2(&collision_request).await?.outcome(),
            ReconnectDurableOutcomeV2::Terminal {
                disposition: ReconnectDurableTerminalDispositionV1::TransportRefCollision,
            }
        );
        database.cleanup().await?;
        Ok(())
    })
}
