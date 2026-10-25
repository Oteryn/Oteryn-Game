#[test]
fn runtime_restart_reconciliation_preserves_concurrent_and_stale_reasons() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) = migrated_database("typed_terminal_restart").await?;
        let now = unix_now().map_err(|_| "invalid clock")?;
        let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;

        let first = record(70, 71, 1, 0xc1, 3, 7, 10, now).map_err(|_| "first record")?;
        let (_first_flow, first_request) = ReconnectDurabilityFlowV1::begin(first);
        assert_eq!(
            journal.prepare(&first_request).await?,
            ReconnectPrepareDispositionV1::Prepared
        );
        let concurrent = record(70, 71, 2, 0xc2, 3, 7, 10, now).map_err(|_| "concurrent record")?;
        let (_flow, concurrent_request) = ReconnectDurabilityFlowV2::begin(concurrent, None);
        assert_eq!(
            journal.prepare_v2(&concurrent_request).await?,
            ReconnectPrepareDispositionV2::RejectedConcurrentPrepared
        );

        let stale = record(70, 71, 3, 0xc3, 4, 7, 10, now).map_err(|_| "stale record")?;
        let (_flow, stale_request) = ReconnectDurabilityFlowV2::begin(stale, None);
        assert_eq!(
            journal.prepare_v2(&stale_request).await?,
            ReconnectPrepareDispositionV2::RejectedStaleAuthority
        );
        drop(journal);

        let recovered = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
        assert_eq!(
            recovered.reconcile_v2(&concurrent_request).await?.outcome(),
            ReconnectDurableOutcomeV2::Terminal {
                disposition: ReconnectDurableTerminalDispositionV1::ConcurrentPrepared,
            }
        );
        assert_eq!(
            recovered.reconcile_v2(&stale_request).await?.outcome(),
            ReconnectDurableOutcomeV2::Terminal {
                disposition: ReconnectDurableTerminalDispositionV1::StaleAuthority,
            }
        );
        database.cleanup().await?;
        Ok(())
    })
}

#[test]
fn runtime_concurrent_terminal_replacement_has_exactly_one_candidate_winner() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) = migrated_database("replacement_race").await?;
        let now = unix_now().map_err(|_| "invalid clock")?;
        seed_current_actor_anchor(&database_url, 10, 10, now).await?;
        let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;

        let first_candidate =
            record(20, 11, 1, 0xd1, 3, 7, 10, now).map_err(|_| "first candidate")?;
        let second_candidate =
            record(21, 11, 1, 0xd2, 3, 7, 10, now).map_err(|_| "second candidate")?;
        let first_request = v2_request(first_candidate, 10, 10).map_err(|_| "first auth")?;
        let second_request = v2_request(second_candidate, 10, 10).map_err(|_| "second auth")?;
        let first_journal = journal.clone();
        let second_journal = journal.clone();
        let first_task =
            tokio::spawn(async move { first_journal.prepare_v2(&first_request).await });
        let second_task =
            tokio::spawn(async move { second_journal.prepare_v2(&second_request).await });
        let results = [first_task.await?, second_task.await?];
        let prepared = results
            .iter()
            .filter(|result| matches!(result, Ok(ReconnectPrepareDispositionV2::Prepared)))
            .count();
        let non_authoritative = results
            .iter()
            .filter(|result| {
                matches!(
                    result,
                    Ok(ReconnectPrepareDispositionV2::RejectedStaleAuthority)
                        | Ok(ReconnectPrepareDispositionV2::IdempotencyConflict)
                        | Err(DurabilityError::InvalidStoredState)
                )
            })
            .count();
        assert_eq!(prepared, 1);
        assert_eq!(non_authoritative, 1);

        let mut connection = PgConnection::connect(&database_url).await?;
        let live_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM game_durability_reconnect_sessions \
                 WHERE character_id = encode($1, 'hex')::uuid AND session_state IN (1, 2)",
        )
        .bind(uuid_v7(11).as_slice())
        .fetch_one(&mut connection)
        .await?;
        assert_eq!(live_count, 1);
        connection.close().await?;
        database.cleanup().await?;
        Ok(())
    })
}

#[test]
fn runtime_terminal_replacement_preserves_actor_epoch_attempt_budget() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) =
            migrated_database("replacement_preserves_attempt_budget").await?;
        let now = unix_now().map_err(|_| "invalid clock")?;
        seed_current_actor_anchor(&database_url, 10, 10, now).await?;
        let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;

        for attempt in 1_u64..=7 {
            let transport = u8::try_from(0xc0_u64 + attempt)?;
            let predecessor_record = record(10, 11, attempt, transport, 3, 7, 10, now)
                .map_err(|_| "predecessor record")?;
            let (_, predecessor_request) = ReconnectDurabilityFlowV1::begin(predecessor_record);
            let expected = if attempt == 1 {
                ReconnectPrepareDispositionV1::Prepared
            } else {
                ReconnectPrepareDispositionV1::RejectedConcurrentPrepared
            };
            assert_eq!(journal.prepare(&predecessor_request).await?, expected);
        }

        let candidate = record(20, 11, 8, 0xd8, 3, 7, 10, now).map_err(|_| "candidate record")?;
        let replacement = v2_request(candidate, 10, 10).map_err(|_| "authorization")?;
        assert_eq!(
            journal.prepare_v2(&replacement).await?,
            ReconnectPrepareDispositionV2::Prepared
        );

        let mut observer = PgConnection::connect(&database_url).await?;
        let retained_count: i16 = sqlx::query_scalar(
            "SELECT attempt_count FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
        )
        .bind(uuid_v7(20).as_slice())
        .fetch_one(&mut observer)
        .await?;
        assert_eq!(retained_count, 8);
        observer.close().await?;

        let ninth_record = record(20, 11, 9, 0xd9, 3, 7, 10, now).map_err(|_| "ninth record")?;
        let (_, ninth_request) = ReconnectDurabilityFlowV1::begin(ninth_record);
        assert_eq!(
            journal.prepare(&ninth_request).await?,
            ReconnectPrepareDispositionV1::AttemptCapacityExceeded
        );

        drop(journal);
        database.cleanup().await?;
        Ok(())
    })
}

#[test]
fn runtime_delayed_predecessor_prepare_cannot_exceed_actor_epoch_attempt_budget() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) =
            migrated_database("replacement_delayed_predecessor_attempt_budget").await?;
        let now = unix_now().map_err(|_| "invalid clock")?;
        seed_current_actor_anchor(&database_url, 10, 10, now).await?;
        let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;

        for attempt in 1_u64..=7 {
            let transport = u8::try_from(0xc0_u64 + attempt)?;
            let predecessor_record = record(10, 11, attempt, transport, 3, 7, 10, now)
                .map_err(|_| "predecessor record")?;
            let (_, predecessor_request) = ReconnectDurabilityFlowV1::begin(predecessor_record);
            let expected = if attempt == 1 {
                ReconnectPrepareDispositionV1::Prepared
            } else {
                ReconnectPrepareDispositionV1::RejectedConcurrentPrepared
            };
            assert_eq!(journal.prepare(&predecessor_request).await?, expected);
        }

        let candidate = record(20, 11, 8, 0xd8, 3, 7, 10, now).map_err(|_| "candidate record")?;
        let replacement = v2_request(candidate, 10, 10).map_err(|_| "authorization")?;
        assert_eq!(
            journal.prepare_v2(&replacement).await?,
            ReconnectPrepareDispositionV2::Prepared
        );

        let delayed_predecessor =
            record(10, 11, 9, 0xd9, 3, 7, 10, now).map_err(|_| "delayed predecessor")?;
        let (_, delayed_request) = ReconnectDurabilityFlowV1::begin(delayed_predecessor);
        assert_eq!(
            journal.prepare(&delayed_request).await?,
            ReconnectPrepareDispositionV1::AttemptCapacityExceeded
        );

        let mut observer = PgConnection::connect(&database_url).await?;
        let retained: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM game_durability_reconnect_attempts \
                 WHERE character_id = encode($1, 'hex')::uuid \
                   AND control_loss_epoch = $2::text::numeric(20, 0)",
        )
        .bind(uuid_v7(11).as_slice())
        .bind("3")
        .fetch_one(&mut observer)
        .await?;
        assert_eq!(retained, 8);
        observer.close().await?;

        drop(journal);
        database.cleanup().await?;
        Ok(())
    })
}

#[test]
fn runtime_terminal_replacement_rejects_exhausted_same_epoch_before_mutation() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) =
            migrated_database("replacement_rejects_exhausted_attempt_budget").await?;
        let now = unix_now().map_err(|_| "invalid clock")?;
        seed_current_actor_anchor(&database_url, 10, 10, now).await?;
        let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;

        for attempt in 1_u64..=8 {
            let transport = u8::try_from(0xc0_u64 + attempt)?;
            let predecessor_record = record(10, 11, attempt, transport, 3, 7, 10, now)
                .map_err(|_| "predecessor record")?;
            let (_, predecessor_request) = ReconnectDurabilityFlowV1::begin(predecessor_record);
            let expected = if attempt == 1 {
                ReconnectPrepareDispositionV1::Prepared
            } else {
                ReconnectPrepareDispositionV1::RejectedConcurrentPrepared
            };
            assert_eq!(journal.prepare(&predecessor_request).await?, expected);
        }

        let candidate = record(20, 11, 9, 0xd9, 3, 7, 10, now).map_err(|_| "candidate record")?;
        let replacement = v2_request(candidate, 10, 10).map_err(|_| "authorization")?;
        assert_eq!(
            journal.prepare_v2(&replacement).await?,
            ReconnectPrepareDispositionV2::AttemptCapacityExceeded
        );

        let mut observer = PgConnection::connect(&database_url).await?;
        let predecessor_state: (i16, i16) = sqlx::query_as(
            "SELECT session_state, attempt_count FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
        )
        .bind(uuid_v7(10).as_slice())
        .fetch_one(&mut observer)
        .await?;
        assert_eq!(predecessor_state, (1, 8));
        let candidate_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
        )
        .bind(uuid_v7(20).as_slice())
        .fetch_one(&mut observer)
        .await?;
        assert_eq!(candidate_count, 0);
        let receipt_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM game_durability_session_replacements \
                 WHERE predecessor_game_session_id = encode($1, 'hex')::uuid \
                   AND candidate_game_session_id = encode($2, 'hex')::uuid",
        )
        .bind(uuid_v7(10).as_slice())
        .bind(uuid_v7(20).as_slice())
        .fetch_one(&mut observer)
        .await?;
        assert_eq!(receipt_count, 0);
        observer.close().await?;

        drop(journal);
        database.cleanup().await?;
        Ok(())
    })
}

#[test]
fn runtime_concurrent_identical_terminal_replacement_replays_exact_receipt() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) = migrated_database("replacement_exact_replay_race").await?;
        let now = unix_now().map_err(|_| "invalid clock")?;
        seed_current_actor_anchor(&database_url, 10, 10, now).await?;
        let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
        let candidate = record(20, 11, 1, 0xe1, 3, 7, 10, now).map_err(|_| "candidate record")?;
        let request = v2_request(candidate, 10, 10).map_err(|_| "authorization")?;
        let mut blocker = PgConnection::connect(&database_url).await?;
        blocker.execute("BEGIN").await?;
        sqlx::query(
            "SELECT game_session_id FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid FOR UPDATE",
        )
        .bind(uuid_v7(10).as_slice())
        .fetch_one(&mut blocker)
        .await?;
        let first_journal = journal.clone();
        let first_request = request.clone();
        let first = tokio::spawn(async move { first_journal.prepare_v2(&first_request).await });
        let second_journal = journal.clone();
        let second_request = request.clone();
        let second = tokio::spawn(async move { second_journal.prepare_v2(&second_request).await });
        let mut observer = PgConnection::connect(&database_url).await?;
        wait_for_lock_waiters(&mut observer, 2).await?;
        blocker.execute("COMMIT").await?;
        let results = [first.await?, second.await?];
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, Ok(ReconnectPrepareDispositionV2::Prepared)))
                .count(),
            1
        );
        assert_eq!(
            results
                .iter()
                .filter(|result| {
                    matches!(result, Ok(ReconnectPrepareDispositionV2::ExistingPrepared))
                })
                .count(),
            1
        );
        observer.close().await?;
        blocker.close().await?;
        drop(journal);
        database.cleanup().await?;
        Ok(())
    })
}
#[test]
fn runtime_v2_reconciliation_racing_commit_uses_one_authoritative_snapshot() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) = migrated_database("reconcile_commit_snapshot").await?;
        let now = unix_now().map_err(|_| "invalid clock")?;
        let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
        let record = record(80, 81, 1, 0xe2, 3, 7, 10, now).map_err(|_| "record")?;
        let (mut flow, prepare_request) = ReconnectDurabilityFlowV1::begin(record.clone());
        assert_eq!(
            journal.prepare(&prepare_request).await?,
            ReconnectPrepareDispositionV1::Prepared
        );
        flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
            &prepare_request,
            ReconnectPrepareDispositionV1::Prepared,
        ))
        .map_err(|_| "prepare completion")?;
        let current = exact_current_authority(&record, now).map_err(|_| "current authority")?;
        let commit_request = flow
            .authorize_commit(current, now)
            .map_err(|_| "commit authorization")?;
        let reconcile_request = ReconnectDurabilityFlowV2::begin(record.clone(), None).1;
        const LOCK_KEY: i64 = 25_220_260_829;
        let mut blocker = PgConnection::connect(&database_url).await?;
        sqlx::query("SELECT pg_advisory_lock($1)")
            .bind(LOCK_KEY)
            .execute(&mut blocker)
            .await?;
        let mut setup = PgConnection::connect(&database_url).await?;
        setup
            .execute(sqlx::query(
                "CREATE FUNCTION block_reconnect_commit_update() RETURNS trigger \
                     LANGUAGE plpgsql AS $$ \
                     BEGIN \
                       PERFORM pg_advisory_xact_lock(25220260829); \
                       RETURN NEW; \
                     END $$",
            ))
            .await?;
        setup
                .execute(sqlx::query(
                    "CREATE TRIGGER block_reconnect_commit_update \
                     BEFORE UPDATE OF current_generation ON game_durability_reconnect_sessions \
                     FOR EACH ROW WHEN (NEW.current_generation IS DISTINCT FROM OLD.current_generation) \
                     EXECUTE FUNCTION block_reconnect_commit_update()",
                ))
                .await?;
        setup.close().await?;
        let commit_journal = journal.clone();
        let commit = tokio::spawn(async move { commit_journal.commit(&commit_request).await });
        let mut observer = PgConnection::connect(&database_url).await?;
        wait_for_lock_waiters(&mut observer, 1).await?;
        let reconcile_journal = journal.clone();
        let reconcile =
            tokio::spawn(async move { reconcile_journal.reconcile_v2(&reconcile_request).await });
        wait_for_lock_waiters(&mut observer, 2).await?;
        sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(LOCK_KEY)
            .execute(&mut blocker)
            .await?;
        assert_eq!(commit.await??, ReconnectCommitDispositionV1::Committed);
        assert_eq!(
            reconcile.await??.outcome(),
            ReconnectDurableOutcomeV2::Committed {
                current_generation: record.connection().candidate(),
                current_transport_ref: record.connection().transport_ref(),
            }
        );
        observer.close().await?;
        blocker.close().await?;
        drop(journal);
        database.cleanup().await?;
        Ok(())
    })
}
#[test]
fn runtime_v2_terminal_reconciliation_rejects_corrupt_attempt_mirrors() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) = migrated_database("terminal_attempt_mirror_corrupt").await?;
        let now = unix_now().map_err(|_| "invalid clock")?;
        let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
        let first = record(50, 51, 1, 0xe3, 3, 7, 10, now).map_err(|_| "first")?;
        let first_request = ReconnectDurabilityFlowV1::begin(first).1;
        assert_eq!(
            journal.prepare(&first_request).await?,
            ReconnectPrepareDispositionV1::Prepared
        );
        let terminal = record(60, 61, 1, 0xe3, 3, 7, 10, now).map_err(|_| "terminal")?;
        let request = ReconnectDurabilityFlowV2::begin(terminal, None).1;
        assert_eq!(
            journal.prepare_v2(&request).await?,
            ReconnectPrepareDispositionV2::RejectedTransportRefCollision
        );
        let mut connection = PgConnection::connect(&database_url).await?;
        sqlx::query(
            "UPDATE game_durability_reconnect_attempts \
                 SET control_loss_epoch = control_loss_epoch + 1 \
                 WHERE game_session_id = encode($1, 'hex')::uuid AND reconnect_attempt_ref = $2",
        )
        .bind(uuid_v7(60).as_slice())
        .bind(1_u64.to_be_bytes().as_slice())
        .execute(&mut connection)
        .await?;
        connection.close().await?;
        assert!(matches!(
            journal.reconcile_v2(&request).await,
            Err(DurabilityError::InvalidStoredState)
        ));
        drop(journal);
        database.cleanup().await?;
        Ok(())
    })
}
#[test]
fn runtime_v2_terminal_reconciliation_rejects_corrupt_session_binding() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) =
            migrated_database("terminal_session_binding_corrupt").await?;
        let now = unix_now().map_err(|_| "invalid clock")?;
        let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
        let first = record(50, 51, 1, 0xe4, 3, 7, 10, now).map_err(|_| "first")?;
        let first_request = ReconnectDurabilityFlowV1::begin(first).1;
        assert_eq!(
            journal.prepare(&first_request).await?,
            ReconnectPrepareDispositionV1::Prepared
        );
        let terminal = record(60, 61, 1, 0xe4, 3, 7, 10, now).map_err(|_| "terminal")?;
        let request = ReconnectDurabilityFlowV2::begin(terminal, None).1;
        assert_eq!(
            journal.prepare_v2(&request).await?,
            ReconnectPrepareDispositionV2::RejectedTransportRefCollision
        );
        let mut connection = PgConnection::connect(&database_url).await?;
        sqlx::query(
            "UPDATE game_durability_reconnect_sessions \
                 SET account_id = '123e4567-e89b-12d3-a456-426614174001'::uuid \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
        )
        .bind(uuid_v7(60).as_slice())
        .execute(&mut connection)
        .await?;
        connection.close().await?;
        assert!(matches!(
            journal.reconcile_v2(&request).await,
            Err(DurabilityError::InvalidStoredState)
        ));
        drop(journal);
        database.cleanup().await?;
        Ok(())
    })
}
