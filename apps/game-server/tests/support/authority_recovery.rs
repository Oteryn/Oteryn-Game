//! Recovery owns database/process orchestration; authority_matrix remains the
//! single independent source and invariant/operator registry.
use super::authority_matrix::*;
use super::durability::{
    AdmissionReconnectJournal, AdmissionReconnectJournalV2, DurabilityError, MigrationExecutor,
};
use super::{postgres, postgres_e2e_is_configured};
use oteryn_game_server::foundation::*;
use sqlx::{Connection, PgConnection};
use std::future::Future;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn pg_test<F, Fut>(name: &str, run: F) -> TestResult<()>
where
    F: FnOnce(String) -> Fut,
    Fut: Future<Output = TestResult<()>>,
{
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let started = Instant::now();
            let db = postgres::IsolatedPostgres::create(name).await?;
            let result = async {
                let url = db.database_url()?;
                MigrationExecutor::connect_migration(&url)
                    .await?
                    .apply_embedded_ledger()
                    .await?;
                run(url).await
            }
            .await;
            db.cleanup().await?;
            println!(
                "authority recovery benchmark {name}: {} ms",
                started.elapsed().as_millis()
            );
            result
        })
}

async fn seed_now(url: &str) -> TestResult<Seed> {
    let mut connection = PgConnection::connect(url).await?;
    let now = sqlx::query_scalar("SELECT FLOOR(EXTRACT(EPOCH FROM clock_timestamp()))::BIGINT")
        .fetch_one(&mut connection)
        .await?;
    connection.close().await?;
    Ok(Seed {
        now,
        ..Seed::fixed()
    })
}

async fn seed_anchor(url: &str, seed: Seed) -> TestResult<()> {
    let mut connection = PgConnection::connect(url).await?;
    sqlx::query("INSERT INTO game_durability_reconnect_sessions (
        game_session_id,account_id,character_id,world_id,runtime_scope_kind,
        runtime_scope_world_id,runtime_scope_channel_id,runtime_scope_instance_id,
        control_loss_epoch,original_grace_deadline,predecessor_generation,
        character_lease_generation,scope_ownership_generation,current_generation,session_state)
        VALUES (encode($1,'hex')::uuid,$2::text::uuid,encode($3,'hex')::uuid,
        encode($4,'hex')::uuid,1,encode($4,'hex')::uuid,encode($5,'hex')::uuid,NULL,3,$6,7,9,9,7,1)")
        .bind(uuid(10).as_slice()).bind(ACCOUNT).bind(uuid(11).as_slice())
        .bind(uuid(12).as_slice()).bind(uuid(13).as_slice()).bind(seed.now+120)
        .execute(&mut connection).await?;
    sqlx::query("INSERT INTO game_durability_control_loss_continuity (
        character_id,control_loss_epoch,account_id,world_id,context_game_session_id,
        original_grace_deadline,protection_entitlement_state,protection_rearm_state)
        VALUES (encode($1,'hex')::uuid,3,$2::text::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,1,1)")
        .bind(uuid(11).as_slice()).bind(ACCOUNT).bind(uuid(12).as_slice())
        .bind(uuid(10).as_slice()).bind(seed.now+120).execute(&mut connection).await?;
    connection.close().await?;
    Ok(())
}

async fn assert_single_attempt(url: &str, replacements: i64, consumed: i64) -> TestResult<()> {
    let mut connection = PgConnection::connect(url).await?;
    let counts: (i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT
        (SELECT COUNT(*) FROM game_durability_session_replacements),
        (SELECT COUNT(*) FROM game_durability_reconnect_attempts),
        (SELECT COUNT(*) FROM game_durability_transport_ref_reservations),
        (SELECT COUNT(*) FROM game_durability_recovery_grant_consumptions),
        (SELECT COUNT(*) FROM game_durability_reconnect_sessions WHERE session_state IN (1,2))",
    )
    .fetch_one(&mut connection)
    .await?;
    assert_eq!(
        counts,
        (replacements, 1, 1, consumed, 1),
        "duplicate effect after replay/race"
    );
    let attempts: i16 = sqlx::query_scalar(
        "SELECT attempt_count FROM game_durability_reconnect_sessions WHERE session_state IN (1,2)",
    )
    .fetch_one(&mut connection)
    .await?;
    assert_eq!(attempts, 1);
    connection.close().await?;
    Ok(())
}

async fn persisted_protection(url: &str) -> TestResult<String> {
    let mut connection = PgConnection::connect(url).await?;
    let value=sqlx::query_scalar("SELECT row_to_json(continuity)::text FROM game_durability_control_loss_continuity AS continuity WHERE character_id=encode($1,'hex')::uuid AND control_loss_epoch=3")
        .bind(uuid(11).as_slice()).fetch_one(&mut connection).await?;
    connection.close().await?;
    Ok(value)
}

fn recovered_prepare_matrix(
    seed: Seed,
    record: &ReconnectDurabilityRecordV1,
    source: &LiveSource,
    v1: Option<ReconnectDurableReconciliationSnapshotV1>,
    v2: ReconnectDurableReconciliationSnapshotV2,
    replacement: Option<TerminalGameSessionReplacementAuthorizationV1>,
) -> TestResult<()> {
    let mut count = 0;
    for boundary in [ConsumerBoundary::CommitV1, ConsumerBoundary::CommitV2] {
        if boundary == ConsumerBoundary::CommitV1 && v1.is_none() {
            assert!(replacement.is_some(), "V1 N/A only for signed replacement");
            continue;
        }
        let project = |live: &LiveSource, expected: bool| -> TestResult<()> {
            if boundary == ConsumerBoundary::CommitV1 {
                let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
                checked(flow.accept_prepare_completion(
                    ReconnectPrepareCompletionV1::for_request(
                        &request,
                        ReconnectPrepareDispositionV1::Ambiguous,
                    ),
                ))?;
                assert_eq!(
                    checked(flow.accept_reconciliation(
                        v1.clone().ok_or("missing V1")?,
                        live.bind(record)?
                    ))?,
                    ReconnectProjectionDecisionV1::AwaitFinalRevalidation
                );
                assert_eq!(
                    flow.phase(),
                    ReconnectDurabilityPhaseV1::AwaitFinalRevalidation
                );
                assert_eq!(
                    flow.authorize_commit(live.bind(record)?, live.time("authorization_at")?)
                        .is_ok(),
                    expected
                );
                assert_eq!(
                    flow.phase(),
                    if expected {
                        ReconnectDurabilityPhaseV1::PendingCommit
                    } else {
                        ReconnectDurabilityPhaseV1::Terminal
                    }
                );
            } else {
                let (mut flow, request) =
                    ReconnectDurabilityFlowV2::begin(record.clone(), replacement.clone());
                let mut budget = v2_budget(seed)?;
                checked(flow.accept_prepare_completion(
                    ReconnectPrepareCompletionV2::for_request(
                        &request,
                        ReconnectPrepareDispositionV2::Ambiguous,
                    ),
                    &mut budget,
                ))?;
                assert_eq!(
                    checked(flow.accept_reconciliation(
                        v2.clone(),
                        live.bind(record)?,
                        &mut budget
                    ))?,
                    ReconnectProjectionDecisionV2::AwaitFinalRevalidation
                );
                assert_eq!(
                    flow.phase(),
                    ReconnectDurabilityPhaseV1::AwaitFinalRevalidation
                );
                assert_eq!(
                    flow.authorize_commit(live.bind(record)?, live.time("authorization_at")?)
                        .is_ok(),
                    expected
                );
                assert_eq!(
                    flow.phase(),
                    if expected {
                        ReconnectDurabilityPhaseV1::PendingCommit
                    } else {
                        ReconnectDurabilityPhaseV1::Terminal
                    }
                );
                assert_eq!(budget.distinct_attempts(), 1);
            }
            Ok(())
        };
        project(source, true)?;
        for (invariant, operator) in registered_cases(boundary) {
            project(&mutated(source, invariant, operator, seed)?, false)?;
            count += 1;
        }
    }
    println!(
        "recovered PREPARE: {count} isolated final revalidation cases; replay itself never installs controller"
    );
    Ok(())
}

#[test]
fn retry_and_lost_commit_response_revalidate_independent_authority() -> TestResult<()> {
    pg_test("authority_retry_lost_response", |url| async move {
        let journal = AdmissionReconnectJournal::connect_runtime(&url).await?;
        let typed = AdmissionReconnectJournalV2::connect_runtime(&url).await?;
        let seed = seed_now(&url).await?;
        let record = prepared_record(seed)?;
        let source = LiveSource::read(seed);
        let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
        assert_eq!(
            journal.prepare(&request).await?,
            ReconnectPrepareDispositionV1::Prepared
        );
        // Persisted PREPARE response is lost; reconnect to the actual journal.
        drop(journal);
        let journal = AdmissionReconnectJournal::connect_runtime(&url).await?;
        let replay = journal.prepare(&request).await?;
        let (_, request2) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
        let replay2 = typed.prepare(&request2).await?;
        run_retry_matrix(seed, &record, &source, None, Some(replay), replay2)?;
        recovered_prepare_matrix(
            seed,
            &record,
            &source,
            Some(journal.reconcile(&request).await?),
            typed.reconcile(&request2).await?,
            None,
        )?;
        assert_eq!(
            checked(
                flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                    &request,
                    ReconnectPrepareDispositionV1::Ambiguous
                ))
            )?,
            ReconnectPrepareActionV1::ReconcileSameAttempt
        );
        assert_eq!(
            checked(
                flow.accept_reconciliation(
                    journal.reconcile(&request).await?,
                    source.bind(&record)?
                )
            )?,
            ReconnectProjectionDecisionV1::AwaitFinalRevalidation
        );
        let commit = checked(flow.authorize_commit(source.bind(&record)?, seed.now + 2))?;
        assert_eq!(
            journal.commit(&commit).await?,
            ReconnectCommitDispositionV1::Committed
        );
        assert_eq!(
            checked(
                flow.accept_commit_completion(ReconnectCommitCompletionV1::for_request(
                    &commit,
                    ReconnectCommitDispositionV1::Ambiguous
                ))
            )?,
            ReconnectCommitActionV1::ReconcileSameAttempt
        );
        assert_eq!(
            flow.phase(),
            ReconnectDurabilityPhaseV1::ReconciliationRequired
        );
        assert_single_attempt(&url, 0, 1).await?;
        let protection_before_replay = persisted_protection(&url).await?;
        // Lost COMMIT response: replay is historical/idempotent, never a controller grant.
        assert_eq!(
            journal.commit(&commit).await?,
            ReconnectCommitDispositionV1::Committed
        );
        assert_single_attempt(&url, 0, 1).await?;
        assert_eq!(
            persisted_protection(&url).await?,
            protection_before_replay,
            "COMMIT replay changed protection activation or expiry"
        );
        drop(journal);
        drop(typed);
        let journal = AdmissionReconnectJournal::connect_runtime(&url).await?;
        let typed = AdmissionReconnectJournalV2::connect_runtime(&url).await?;
        run_loaded_matrix(
            seed,
            &record,
            &source,
            journal.reconcile(&request).await?,
            typed.reconcile(&request2).await?,
        )?;
        Ok(())
    })
}

#[test]
fn signed_replacement_replay_keeps_receipt_and_final_revalidation() -> TestResult<()> {
    pg_test("authority_replacement_replay", |url| async move {
        let journal = AdmissionReconnectJournalV2::connect_runtime(&url).await?;
        let legacy = AdmissionReconnectJournal::connect_runtime(&url).await?;
        let seed = seed_now(&url).await?;
        seed_anchor(&url, seed).await?;
        let record = prepared_record(seed)?;
        let source = LiveSource::read(seed);
        let auth = source.authorize_replacement(&record)?;
        let (_, request) = ReconnectDurabilityFlowV2::begin(record.clone(), Some(auth.clone()));
        assert_eq!(
            journal.prepare(&request).await?,
            ReconnectPrepareDispositionV2::Prepared
        );
        drop(journal);
        let journal = AdmissionReconnectJournalV2::connect_runtime(&url).await?;
        let replay = journal.prepare(&request).await?;
        run_retry_matrix(seed, &record, &source, Some(auth.clone()), None, replay)?;
        recovered_prepare_matrix(
            seed,
            &record,
            &source,
            None,
            journal.reconcile(&request).await?,
            Some(auth),
        )?;
        assert_single_attempt(&url, 1, 0).await?;
        let (_, unsigned) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
        let (_, v1) = ReconnectDurabilityFlowV1::begin(record.clone());
        assert!(matches!(
            journal.prepare(&unsigned).await,
            Err(DurabilityError::InvalidStoredState)
        ));
        assert!(matches!(
            journal.reconcile(&unsigned).await,
            Err(DurabilityError::InvalidStoredState)
        ));
        assert!(matches!(
            legacy.prepare(&v1).await,
            Err(DurabilityError::InvalidStoredState)
        ));
        assert_eq!(
            journal.reconcile(&request).await?.outcome(),
            ReconnectDurableOutcomeV2::Prepared
        );
        assert_single_attempt(&url, 1, 0).await?;
        Ok(())
    })
}

#[test]
fn typed_terminal_reasons_survive_reload_and_every_source_mutation() -> TestResult<()> {
    pg_test("authority_typed_terminals", |url| async move {
        let journal = AdmissionReconnectJournalV2::connect_runtime(&url).await?;
        let seed = seed_now(&url).await?;
        let (_, first) = ReconnectDurabilityFlowV2::begin(prepared_record(seed)?, None);
        assert_eq!(
            journal.prepare(&first).await?,
            ReconnectPrepareDispositionV2::Prepared
        );
        for (seed, initial, reason) in [
            (
                Seed {
                    session: 30,
                    character: 31,
                    ..seed
                },
                ReconnectPrepareDispositionV2::RejectedTransportRefCollision,
                ReconnectDurableTerminalDispositionV1::TransportRefCollision,
            ),
            (
                Seed {
                    attempt: 2,
                    transport: 0x72,
                    ..seed
                },
                ReconnectPrepareDispositionV2::RejectedConcurrentPrepared,
                ReconnectDurableTerminalDispositionV1::ConcurrentPrepared,
            ),
            (
                Seed {
                    attempt: 3,
                    transport: 0x73,
                    epoch: 4,
                    ..seed
                },
                ReconnectPrepareDispositionV2::RejectedStaleAuthority,
                ReconnectDurableTerminalDispositionV1::StaleAuthority,
            ),
        ] {
            let record = prepared_record(seed)?;
            let source = LiveSource::read(seed);
            let (_, request) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
            assert_eq!(journal.prepare(&request).await?, initial);
            let recovered = AdmissionReconnectJournalV2::connect_runtime(&url).await?;
            assert_eq!(
                recovered.prepare(&request).await?,
                ReconnectPrepareDispositionV2::ExistingTerminal {
                    disposition: reason
                }
            );
            let legacy = AdmissionReconnectJournal::connect_runtime(&url).await?;
            let (_, request1) = ReconnectDurabilityFlowV1::begin(record.clone());
            run_terminal_matrix(
                seed,
                &record,
                &source,
                legacy.reconcile(&request1).await?,
                recovered.reconcile(&request).await?,
                reason,
            )?;
        }
        Ok(())
    })
}

async fn wait_for_two_lock_waiters(connection: &mut PgConnection) -> TestResult<()> {
    for _ in 0..1000 {
        let count:i64=sqlx::query_scalar("SELECT COUNT(*) FROM pg_stat_activity WHERE datname=current_database() AND pid<>pg_backend_pid() AND wait_event_type='Lock'")
            .fetch_one(&mut *connection).await?;
        if count >= 2 {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    Err("both replacement participants never reached the held PostgreSQL lock".into())
}

async fn replacement_race(url: String, identical: bool) -> TestResult<()> {
    let journal = AdmissionReconnectJournalV2::connect_runtime(&url).await?;
    let seed = seed_now(&url).await?;
    seed_anchor(&url, seed).await?;
    let other = if identical {
        seed
    } else {
        Seed {
            session: 21,
            transport: 0x72,
            ..seed
        }
    };
    let mut requests = Vec::new();
    for input in [seed, other] {
        let record = prepared_record(input)?;
        let source = LiveSource::read(input);
        let (_, request) = ReconnectDurabilityFlowV2::begin(
            record.clone(),
            Some(source.authorize_replacement(&record)?),
        );
        requests.push(request);
    }
    let mut blocker = PgConnection::connect(&url).await?;
    sqlx::query("BEGIN").execute(&mut blocker).await?;
    sqlx::query("SELECT game_session_id FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(uuid(10).as_slice()).fetch_one(&mut blocker).await?;
    let mut tasks = Vec::new();
    for request in requests.clone() {
        let worker = journal.clone();
        tasks.push(tokio::spawn(async move { worker.prepare(&request).await }));
    }
    let mut observer = PgConnection::connect(&url).await?;
    wait_for_two_lock_waiters(&mut observer).await?;
    sqlx::query("COMMIT").execute(&mut blocker).await?;
    let mut results = Vec::new();
    for task in tasks {
        results.push(task.await?);
    }
    let winner = results
        .iter()
        .position(|x| matches!(x, Ok(ReconnectPrepareDispositionV2::Prepared)))
        .ok_or("no prepared winner")?;
    assert_eq!(
        results
            .iter()
            .filter(|x| matches!(x, Ok(ReconnectPrepareDispositionV2::Prepared)))
            .count(),
        1
    );
    if identical {
        assert!(matches!(
            results[1 - winner],
            Ok(ReconnectPrepareDispositionV2::ExistingPrepared)
        ));
    } else {
        assert!(matches!(
            results[1 - winner],
            Ok(ReconnectPrepareDispositionV2::RejectedStaleAuthority
                | ReconnectPrepareDispositionV2::IdempotencyConflict)
                | Err(DurabilityError::InvalidStoredState)
        ));
        assert!(
            matches!(
                journal.reconcile(&requests[1 - winner]).await,
                Err(DurabilityError::InvalidStoredState)
            ),
            "loser acquired a recovery receipt"
        );
    }
    let winner_seed = if winner == 0 { seed } else { other };
    let record = prepared_record(winner_seed)?;
    let source = LiveSource::read(winner_seed);
    let replay = journal.prepare(&requests[winner]).await?;
    run_retry_matrix(
        winner_seed,
        &record,
        &source,
        Some(source.authorize_replacement(&record)?),
        None,
        replay,
    )?;
    recovered_prepare_matrix(
        winner_seed,
        &record,
        &source,
        None,
        journal.reconcile(&requests[winner]).await?,
        Some(source.authorize_replacement(&record)?),
    )?;
    let (mut winner_flow, winner_request) = ReconnectDurabilityFlowV2::begin(
        record.clone(),
        Some(source.authorize_replacement(&record)?),
    );
    let mut budget = v2_budget(winner_seed)?;
    checked(winner_flow.accept_prepare_completion(
        ReconnectPrepareCompletionV2::for_request(&winner_request, replay),
        &mut budget,
    ))?;
    let commit = checked(winner_flow.authorize_commit(source.bind(&record)?, seed.now + 2))?;
    let legacy = AdmissionReconnectJournal::connect_runtime(&url).await?;
    assert_eq!(
        legacy.commit(&commit).await?,
        ReconnectCommitDispositionV1::Committed
    );
    assert_single_attempt(&url, 1, 1).await?;
    let snapshot = journal.reconcile(&requests[winner]).await?;
    assert!(matches!(
        reconcile_v2(
            &record,
            source.bind(&record)?,
            snapshot.clone(),
            winner_seed
        )?,
        ReconnectProjectionDecisionV2::InstallController { .. }
    ));
    for (invariant, operator) in registered_cases(ConsumerBoundary::ReconcileV2) {
        let changed = mutated(&source, invariant, operator, winner_seed)?;
        assert!(
            reconcile_v2(
                &record,
                changed.bind(&record)?,
                snapshot.clone(),
                winner_seed
            )
            .is_err(),
            "race winner projection {invariant:?}/{operator:?}"
        );
    }
    // Loser/identity substitution cannot authorize COMMIT against the winner's current binding.
    if !identical {
        let loser_record = prepared_record(if winner == 0 { other } else { seed })?;
        assert!(
            prepared_v1(&loser_record)?
                .authorize_commit(source.bind(&loser_record)?, seed.now + 2)
                .is_err()
        );
    }
    println!(
        "synchronized replacement race: identical={identical}, two lock waiters, one winner, one receipt"
    );
    Ok(())
}

#[test]
fn deterministic_replacement_races_repeat_without_duplicate_authority() -> TestResult<()> {
    for _ in 0..3 {
        pg_test("authority_distinct_race", |url| {
            replacement_race(url, false)
        })?;
        pg_test("authority_identical_race", |url| {
            replacement_race(url, true)
        })?;
    }
    Ok(())
}

struct CommittedFixture {
    seed: Seed,
    record: ReconnectDurabilityRecordV1,
    source: LiveSource,
    request: ReconnectPrepareRequestV1,
    request2: ReconnectPrepareRequestV2,
}
async fn committed_fixture(url: &str) -> TestResult<CommittedFixture> {
    let journal = AdmissionReconnectJournal::connect_runtime(url).await?;
    let seed = seed_now(url).await?;
    let record = prepared_record(seed)?;
    let source = LiveSource::read(seed);
    let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
    let disposition = journal.prepare(&request).await?;
    assert_eq!(disposition, ReconnectPrepareDispositionV1::Prepared);
    checked(
        flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
            &request,
            disposition,
        )),
    )?;
    let commit = checked(flow.authorize_commit(source.bind(&record)?, seed.now + 2))?;
    assert_eq!(
        journal.commit(&commit).await?,
        ReconnectCommitDispositionV1::Committed
    );
    let (_, request2) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
    Ok(CommittedFixture {
        seed,
        record,
        source,
        request,
        request2,
    })
}

#[test]
fn later_durable_epoch_keeps_history_but_cannot_restore_old_controller() -> TestResult<()> {
    pg_test("authority_later_epoch", |url| async move {
        let fixture = committed_fixture(&url).await?;
        let later = Seed {
            epoch: 4,
            generation: 8,
            attempt: 2,
            transport: 0x72,
            proof_nonce: 0x56,
            now: fixture.seed.now + 1,
            ..fixture.seed
        };
        let later_record = prepared_record(later)?;
        let later_source = LiveSource::read(later);
        let journal = AdmissionReconnectJournal::connect_runtime(&url).await?;
        let (_, next) = ReconnectDurabilityFlowV1::begin(later_record);
        assert_eq!(
            journal.prepare(&next).await?,
            ReconnectPrepareDispositionV1::Prepared
        );
        drop(journal);
        let journal = AdmissionReconnectJournal::connect_runtime(&url).await?;
        let typed = AdmissionReconnectJournalV2::connect_runtime(&url).await?;
        let v1 = journal.reconcile(&fixture.request).await?;
        let v2 = typed.reconcile(&fixture.request2).await?;
        assert_eq!(
            v1,
            ReconnectDurableReconciliationSnapshotV1::committed(fixture.record.clone())
        );
        assert!(matches!(
            v2.outcome(),
            ReconnectDurableOutcomeV2::Committed { .. }
        ));
        let changed = mutated(
            &fixture.source,
            AuthorityInvariant::ContinuityEpoch,
            MutationOperator::Newer,
            fixture.seed,
        )?;
        for current in [&changed, &later_source] {
            assert!(
                reconcile_v1(&fixture.record, current.bind(&fixture.record)?, v1.clone()).is_err()
            );
            assert!(
                reconcile_v2(
                    &fixture.record,
                    current.bind(&fixture.record)?,
                    v2.clone(),
                    fixture.seed
                )
                .is_err()
            );
        }
        Ok(())
    })
}

async fn restart_child(
    url: &str,
    now: i64,
    expected_revision: i64,
    field: &str,
    parent: u32,
) -> TestResult<()> {
    assert_ne!(std::process::id(), parent);
    let seed = Seed {
        now,
        ..Seed::fixed()
    };
    let record = prepared_record(seed)?; // Expected binding, never live provenance.
    let mut connection = PgConnection::connect(url).await?;
    let (revision, payload): (i64, String) =
        sqlx::query_as("SELECT revision,payload FROM authority_test_resolver WHERE id=1")
            .fetch_one(&mut connection)
            .await?;
    assert_eq!(
        revision, expected_revision,
        "child did not reread current resolver revision"
    );
    let live = LiveSource(serde_json::from_str(&payload)?);
    let baseline = LiveSource::read(seed);
    let changed: Vec<_> = baseline
        .0
        .as_object()
        .ok_or("source object")?
        .keys()
        .filter(|key| baseline.0[*key] != live.0[*key])
        .map(String::as_str)
        .collect();
    assert_eq!(
        changed,
        if field == "positive" {
            Vec::new()
        } else {
            vec![field]
        },
        "restart one-invariant rule"
    );
    assert_eq!(
        live.0.as_object().ok_or("source object")?.len(),
        baseline.0.as_object().ok_or("baseline object")?.len()
    );
    let journal = AdmissionReconnectJournal::connect_runtime(url).await?;
    let typed = AdmissionReconnectJournalV2::connect_runtime(url).await?;
    let (_, request) = ReconnectDurabilityFlowV1::begin(record.clone());
    let (_, request2) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
    let v1 = journal.reconcile(&request).await?;
    let v2 = typed.reconcile(&request2).await?;
    let result1 = reconcile_v1(&record, live.bind(&record)?, v1);
    let result2 = reconcile_v2(&record, live.bind(&record)?, v2, seed);
    if field == "positive" {
        assert_eq!(
            result1?,
            ReconnectProjectionDecisionV1::InstallController {
                generation: checked(ConnectionGeneration::new(8))?,
                transport_ref: transport(seed.transport)?
            }
        );
        assert_eq!(
            result2?,
            ReconnectProjectionDecisionV2::InstallController {
                generation: checked(ConnectionGeneration::new(8))?,
                transport_ref: transport(seed.transport)?
            }
        );
    } else {
        assert!(result1.is_err(), "restart V1 granted {field}");
        assert!(result2.is_err(), "restart V2 granted {field}");
    }
    connection.close().await?;
    Ok(())
}

async fn invoke_child(url: &str, seed: Seed, revision: i64, field: &str) -> TestResult<()> {
    let mut child = Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "authority_recovery::restart_rereads_independent_source",
        ])
        .env("OTERYN_AUTHORITY_RESTART_CHILD", "1")
        .env("OTERYN_AUTHORITY_RESTART_URL", url)
        .env("OTERYN_AUTHORITY_RESTART_NOW", seed.now.to_string())
        .env("OTERYN_AUTHORITY_RESTART_REVISION", revision.to_string())
        .env("OTERYN_AUTHORITY_RESTART_FIELD", field)
        .env(
            "OTERYN_AUTHORITY_RESTART_PARENT",
            std::process::id().to_string(),
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let started = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            let output = child.wait_with_output()?;
            assert!(
                output.status.success(),
                "restart {field}/{revision} failed: {} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return Ok(());
        }
        if started.elapsed() > Duration::from_secs(30) {
            child.kill()?;
            child.wait()?;
            return Err(format!("restart child timed out: {field}/{revision}").into());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[test]
fn restart_rereads_independent_source() -> TestResult<()> {
    if std::env::var_os("OTERYN_AUTHORITY_RESTART_CHILD").is_some() {
        return tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(restart_child(
                &std::env::var("OTERYN_AUTHORITY_RESTART_URL")?,
                std::env::var("OTERYN_AUTHORITY_RESTART_NOW")?.parse()?,
                std::env::var("OTERYN_AUTHORITY_RESTART_REVISION")?.parse()?,
                &std::env::var("OTERYN_AUTHORITY_RESTART_FIELD")?,
                std::env::var("OTERYN_AUTHORITY_RESTART_PARENT")?.parse()?,
            ));
    }
    pg_test("authority_process_restart", |url| async move {
        let mut connection = PgConnection::connect(&url).await?;
        sqlx::query("CREATE TABLE authority_test_resolver (id INTEGER PRIMARY KEY,revision BIGINT NOT NULL,payload TEXT NOT NULL)")
            .execute(&mut connection).await?;
        let fixture = committed_fixture(&url).await?;
        sqlx::query("INSERT INTO authority_test_resolver VALUES (1,0,$1)")
            .bind(fixture.source.0.to_string())
            .execute(&mut connection)
            .await?;
        invoke_child(&url, fixture.seed, 0, "positive").await?;
        let cases = registered_cases(ConsumerBoundary::ReconcileV1);
        assert_eq!(
            cases,
            registered_cases(ConsumerBoundary::ReconcileV2),
            "restart version applicability diverged"
        );
        for (ordinal, (invariant, operator)) in cases.iter().copied().enumerate() {
            let live = mutated(&fixture.source, invariant, operator, fixture.seed)?;
            let revision = i64::try_from(ordinal + 1)?;
            sqlx::query("UPDATE authority_test_resolver SET revision=$1,payload=$2 WHERE id=1")
                .bind(revision)
                .bind(live.0.to_string())
                .execute(&mut connection)
                .await?;
            invoke_child(&url, fixture.seed, revision, invariant.field()).await?;
        }
        // Restore a positive source after every negative child: the durable record never changed.
        let revision = i64::try_from(cases.len() + 1)?;
        sqlx::query("UPDATE authority_test_resolver SET revision=$1,payload=$2 WHERE id=1")
            .bind(revision)
            .bind(fixture.source.0.to_string())
            .execute(&mut connection)
            .await?;
        invoke_child(&url, fixture.seed, revision, "positive").await?;
        assert_single_attempt(&url, 0, 1).await?;
        println!(
            "process restart authority matrix: {} fresh children, {} isolated V1/V2 negatives, two positive controls",
            cases.len() + 2,
            cases.len() * 2
        );
        connection.close().await?;
        Ok(())
    })
}
