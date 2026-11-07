#[test]
fn runtime_terminal_replacement_forward_syncs_scope_and_replays_exact_receipt() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) = migrated_database("forward_sync_receipt").await?;
        let now = unix_now().map_err(|_| "invalid clock")?;
        seed_current_actor_anchor(&database_url, 10, 9, now).await?;
        let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
        let candidate = record(20, 11, 1, 0xa1, 3, 7, 10, now).map_err(|_| "candidate record")?;
        let request = v2_request(candidate, 10, 10).map_err(|_| "authorization")?;

        assert_eq!(
            journal.prepare_v2(&request).await?,
            ReconnectPrepareDispositionV2::Prepared
        );
        let mut connection = PgConnection::connect(&database_url).await?;
        let predecessor: (String, i16) = sqlx::query_as(
            "SELECT scope_ownership_generation::text, session_state \
                 FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
        )
        .bind(uuid_v7(10).as_slice())
        .fetch_one(&mut connection)
        .await?;
        assert_eq!(predecessor, ("10".to_owned(), 3));
        let receipt_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM game_durability_session_replacements \
                 WHERE character_id = encode($1, 'hex')::uuid \
                   AND predecessor_game_session_id = encode($2, 'hex')::uuid \
                   AND candidate_game_session_id = encode($3, 'hex')::uuid",
        )
        .bind(uuid_v7(11).as_slice())
        .bind(uuid_v7(10).as_slice())
        .bind(uuid_v7(20).as_slice())
        .fetch_one(&mut connection)
        .await?;
        assert_eq!(receipt_count, 1);
        connection.close().await?;

        drop(journal);
        let recovered = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
        assert_eq!(
            recovered.prepare_v2(&request).await?,
            ReconnectPrepareDispositionV2::ExistingPrepared
        );
        let conflicting = v2_request(request.record().clone(), 30, 10)
            .map_err(|_| "conflicting authorization")?;
        let conflict = recovered.prepare_v2(&conflicting).await;
        assert!(failed_closed_prepare(&conflict));
        database.cleanup().await?;
        Ok(())
    })
}

#[test]
fn replacement_created_session_can_reconnect_in_later_epoch_without_reusing_replacement_authorization()
-> TestResult {
    run_postgres_test(async {
        let (database, database_url) =
            migrated_database("replacement_candidate_later_epoch").await?;
        let now = unix_now().map_err(|_| "invalid clock")?;
        seed_current_actor_anchor(&database_url, 10, 10, now).await?;
        let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;

        let candidate = record(20, 11, 1, 0xa2, 3, 7, 10, now).map_err(|_| "candidate record")?;
        let authorization =
            authorization_for(10, &candidate, 10).map_err(|_| "replacement authorization")?;
        let (mut replacement_flow, replacement_request) =
            ReconnectDurabilityFlowV2::begin(candidate.clone(), Some(authorization));
        assert_eq!(
            journal.prepare_v2(&replacement_request).await?,
            ReconnectPrepareDispositionV2::Prepared
        );

        let mut budget = ReconnectAttemptBudgetV1::new(candidate.continuity().control_loss_epoch());
        budget
            .reserve(
                candidate.identity().reconnect_attempt_ref(),
                candidate.connection().transport_ref(),
            )
            .map_err(|_| "replacement attempt reservation")?;
        replacement_flow
            .accept_prepare_completion(
                ReconnectPrepareCompletionV2::for_request(
                    &replacement_request,
                    ReconnectPrepareDispositionV2::Prepared,
                ),
                &mut budget,
            )
            .map_err(|_| "replacement prepare completion")?;
        let replacement_commit = replacement_flow
            .authorize_commit(
                exact_current_authority(&candidate, now)
                    .map_err(|_| "current replacement authority")?,
                now,
            )
            .map_err(|_| "replacement commit authorization")?;
        assert_eq!(
            journal.commit(&replacement_commit).await?,
            ReconnectCommitDispositionV1::Committed
        );

        let mut observer = PgConnection::connect(&database_url).await?;
        let receipt_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM game_durability_session_replacements \
                 WHERE predecessor_game_session_id = encode($1, 'hex')::uuid \
                   AND candidate_game_session_id = encode($2, 'hex')::uuid",
        )
        .bind(uuid_v7(10).as_slice())
        .bind(uuid_v7(20).as_slice())
        .fetch_one(&mut observer)
        .await?;
        assert_eq!(receipt_count, 1);
        observer.close().await?;

        let later_epoch =
            record(20, 11, 2, 0xa3, 4, 8, 10, now).map_err(|_| "later epoch record")?;
        let (_, later_request) = ReconnectDurabilityFlowV1::begin(later_epoch.clone());
        assert_eq!(
            journal.prepare(&later_request).await?,
            ReconnectPrepareDispositionV1::Prepared
        );

        let replay = journal.prepare(&later_request).await;
        let later_v2_request = ReconnectDurabilityFlowV2::begin(later_epoch, None).1;
        let reconciliation = journal.reconcile_v2(&later_v2_request).await;
        let replay_ok = matches!(replay, Ok(ReconnectPrepareDispositionV1::ExistingPrepared));
        let reconciliation_ok = matches!(
            reconciliation,
            Ok(ref snapshot) if snapshot.outcome() == ReconnectDurableOutcomeV2::Prepared
        );
        assert!(
            replay_ok && reconciliation_ok,
            "later ordinary reconnect must replay and reconcile without historical replacement authorization; replay_ok={replay_ok}, reconciliation_ok={reconciliation_ok}"
        );

        drop(journal);
        database.cleanup().await?;
        Ok(())
    })
}

#[test]
fn replaced_collision_replay_retains_terminal_and_idempotency_priority() -> TestResult {
    run_postgres_test(async {
        let (database, url) = migrated_database("replaced_collision_history").await?;
        let now = unix_now().map_err(|_| "clock")?;
        let journal = AdmissionReconnectJournal::connect_runtime(&url).await?;
        let reserved = record(50, 51, 1, 0xa2, 3, 7, 10, now).map_err(|_| "reservation")?;
        assert_eq!(
            journal
                .prepare(&ReconnectDurabilityFlowV1::begin(reserved).1)
                .await?,
            ReconnectPrepareDispositionV1::Prepared
        );
        let original = record(20, 11, 1, 0xa2, 3, 7, 10, now).map_err(|_| "original")?;
        let original_request = ReconnectDurabilityFlowV1::begin(original.clone()).1;
        assert_eq!(
            journal.prepare(&original_request).await?,
            ReconnectPrepareDispositionV1::RejectedTransportRefCollision
        );
        let successor = record(30, 11, 2, 0xa3, 3, 7, 10, now).map_err(|_| "successor")?;
        let successor_request =
            v2_request(successor, 20, 10).map_err(|_| "replacement authority")?;
        assert_eq!(
            journal.prepare_v2(&successor_request).await?,
            ReconnectPrepareDispositionV2::Prepared
        );
        assert_eq!(
            journal.prepare(&original_request).await?,
            ReconnectPrepareDispositionV1::ExistingTerminal
        );
        let typed_original = ReconnectDurabilityFlowV2::begin(original, None).1;
        assert_eq!(
            journal.prepare_v2(&typed_original).await?,
            ReconnectPrepareDispositionV2::ExistingTerminal {
                disposition: ReconnectDurableTerminalDispositionV1::TransportRefCollision,
            }
        );
        let changed = record(20, 11, 1, 0xa4, 3, 7, 10, now).map_err(|_| "changed replay")?;
        assert_eq!(
            journal
                .prepare(&ReconnectDurabilityFlowV1::begin(changed).1)
                .await?,
            ReconnectPrepareDispositionV1::IdempotencyConflict
        );
        database.cleanup().await?;
        Ok(())
    })
}

#[test]
fn replacement_created_session_can_replay_fresh_same_fence_attempt_after_collision() -> TestResult {
    run_postgres_test(async {
        let (database, database_url) =
            migrated_database("replacement_candidate_same_fence_retry").await?;
        let now = unix_now().map_err(|_| "invalid clock")?;
        seed_current_actor_anchor(&database_url, 10, 10, now).await?;
        let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;

        let reserved =
            record(50, 51, 1, 0xa2, 3, 7, 10, now).map_err(|_| "transport reservation record")?;
        assert_eq!(
            journal
                .prepare(&ReconnectDurabilityFlowV1::begin(reserved).1)
                .await?,
            ReconnectPrepareDispositionV1::Prepared
        );

        let original = record(20, 11, 1, 0xa2, 3, 7, 10, now).map_err(|_| "candidate record")?;
        let authorization =
            authorization_for(10, &original, 10).map_err(|_| "replacement authorization")?;
        let original_request = ReconnectDurabilityFlowV2::begin(original, Some(authorization)).1;
        assert_eq!(
            journal.prepare_v2(&original_request).await?,
            ReconnectPrepareDispositionV2::RejectedTransportRefCollision
        );

        let fresh = record(20, 11, 2, 0xa3, 3, 7, 10, now).map_err(|_| "fresh attempt")?;
        let fresh_legacy_request = ReconnectDurabilityFlowV1::begin(fresh.clone()).1;
        assert_eq!(
            journal.prepare(&fresh_legacy_request).await?,
            ReconnectPrepareDispositionV1::Prepared
        );
        assert_eq!(
            journal.prepare(&fresh_legacy_request).await?,
            ReconnectPrepareDispositionV1::ExistingPrepared
        );

        let fresh_v2_request = ReconnectDurabilityFlowV2::begin(fresh, None).1;
        assert_eq!(
            journal.reconcile_v2(&fresh_v2_request).await?.outcome(),
            ReconnectDurableOutcomeV2::Prepared
        );

        drop(journal);
        database.cleanup().await?;
        Ok(())
    })
}
