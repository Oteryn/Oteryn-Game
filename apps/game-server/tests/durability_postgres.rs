// Include the unchanged Foundation source in this test crate so privately sealed
// fixture owners and Durability use one type universe, without a production seal.
extern crate self as oteryn_game_server;
#[path = "../src/foundation/mod.rs"]
pub mod foundation;

#[path = "support/authority_matrix.rs"]
mod authority_matrix;
#[path = "support/authority_recovery.rs"]
mod authority_recovery;
#[path = "../src/durability/mod.rs"]
mod durability;
#[path = "support/postgres.rs"]
mod postgres;

#[test]
fn registered_runtime_shares_custody_and_retains_originals_across_all_handles()
-> Result<(), Box<dyn std::error::Error>> {
    use durability::{AdmissionRuntime, DurabilityCustody};
    use foundation::ReconnectDurabilityFlowV2;
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("registered_runtime").await?;
            let result = async {
                let url = database.database_url()?;
                MigrationExecutor::connect_migration(&url)
                    .await?
                    .apply_embedded_ledger()
                    .await?;
                let pool = sqlx::PgPool::connect(&url).await?;
                let (predecessor, _) = DurabilityCustody::acquire(&pool).await?;
                predecessor
                    .checkpoint(&pool, 1, 1, "retained fresh original")
                    .await?;
                predecessor
                    .checkpoint(&pool, 2, 2, "retained guard original")
                    .await?;
                let runtime = AdmissionRuntime::connect(&url).await?;
                let repeated = AdmissionRuntime::connect(&url).await?;
                let generation: String = sqlx::query_scalar(
                    "SELECT generation::text FROM game_durability_executor_custody WHERE slot = 0",
                )
                .fetch_one(&pool)
                .await?;
                assert_eq!(
                    generation, "2",
                    "same process registration must not take custody twice"
                );
                assert_eq!(runtime.recovered_pending(), repeated.recovered_pending());
                for (slot, expected) in [
                    (0, "retained fresh original"),
                    (1, "retained guard original"),
                ] {
                    assert_eq!(
                        runtime.recovered_pending()[slot]
                            .as_ref()
                            .ok_or("registered runtime lost pending original")?
                            .operation_json,
                        expected
                    );
                }
                assert!(matches!(
                    predecessor.fence(&pool).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                let guards = runtime.guards();
                assert!(guards.load(&[]).await?.is_empty());
                let fresh = repeated.fresh();
                let owner = postgres::fresh::Source::new(postgres_clock(&pool).await?)?;
                let fresh_request = owner.request()?;
                assert!(matches!(
                    fresh.reconcile(fresh_request.operation()).await?,
                    durability::fresh_admission::FreshReconciliation::Absent
                ));
                let record = authority_matrix::prepared_record(authority_matrix::Seed::fixed())?;
                let v1_request = ReconnectDurabilityFlowV1::begin(record.clone()).1;
                let v2_request = ReconnectDurabilityFlowV2::begin(record, None).1;
                let v1 = runtime.reconnect_v1();
                let v2 = repeated.reconnect_v2();
                // A real successor invalidates every previously issued handle.
                let (_successor, retained) = DurabilityCustody::acquire(&pool).await?;
                assert_eq!(&retained, runtime.recovered_pending());
                assert!(matches!(
                    guards.load(&[]).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                assert!(matches!(
                    fresh.reconcile(fresh_request.operation()).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                assert!(matches!(
                    v1.prepare(&v1_request).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                assert!(matches!(
                    v2.prepare(&v2_request).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                // Registration cannot mint replacement capacity after its token is stale.
                let stale = AdmissionRuntime::connect(&url).await?;
                assert!(matches!(
                    stale.guards().load(&[]).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                assert!(matches!(
                    AdmissionRuntime::connect("postgres://different.invalid/other").await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                let attempts: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM game_durability_reconnect_attempts")
                        .fetch_one(&pool)
                        .await?;
                assert_eq!(attempts, 0, "stale handles must retain no command effects");
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn executor_custody_retains_two_original_slots_and_fences_predecessor()
-> Result<(), Box<dyn std::error::Error>> {
    use durability::DurabilityCustody;
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("executor_custody").await?;
            let result = async {
                let url = database.database_url()?;
                MigrationExecutor::connect_migration(&url)
                    .await?
                    .apply_embedded_ledger()
                    .await?;
                let pool = sqlx::PgPool::connect(&url).await?;
                let (first, empty) = DurabilityCustody::acquire(&pool).await?;
                if empty != [None, None] {
                    return Err("new custody did not read both empty slots".into());
                }
                first
                    .checkpoint(&pool, 1, 1, "original fresh operation")
                    .await?;
                first
                    .checkpoint(&pool, 2, 2, "original publication operation")
                    .await?;
                first
                    .checkpoint(&pool, 1, 1, "original fresh operation")
                    .await?;
                if !matches!(
                    first.checkpoint(&pool, 1, 1, "different operation").await,
                    Err(DurabilityError::InvalidStoredState)
                ) || !matches!(
                    first.checkpoint(&pool, 3, 1, "third slot").await,
                    Err(DurabilityError::InvalidStoredState)
                ) {
                    return Err("checkpoint overwrite or third slot admitted".into());
                }
                // Observe the successor actually waiting on the predecessor's
                // shared advisory transaction fence; no timing-only sleep proof.
                let predecessor_pass = first.fence(&pool).await?;
                let release_after_observation = async {
                    let observed = tokio::time::timeout(std::time::Duration::from_secs(2), async {
                        loop {
                            let waiting: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_locks WHERE locktype = 'advisory' AND NOT granted AND database = (SELECT oid FROM pg_database WHERE datname = current_database()))").fetch_one(&pool).await?;
                            if waiting { return Ok::<(), sqlx::Error>(()); }
                            tokio::task::yield_now().await;
                        }
                    }).await;
                    // Always release the predecessor even if observation fails;
                    // join retains the successor future until it settles.
                    predecessor_pass.commit().await?;
                    observed.map_err(|_| "successor did not wait on predecessor custody")??;
                    Ok::<(), Box<dyn std::error::Error>>(())
                };
                let successor_pool = pool.clone();
                let successor = tokio::spawn(async move { DurabilityCustody::acquire(&successor_pool).await });
                let observed = release_after_observation.await;
                // Retain and await the handle even when observation returned an error.
                let takeover = successor.await?;
                observed?;
                let (second, recovered) = takeover?;
                if recovered[0]
                    .as_ref()
                    .map(|p| (p.slot, p.operation_kind, p.operation_json.as_str()))
                    != Some((1, 1, "original fresh operation"))
                    || recovered[1]
                        .as_ref()
                        .map(|p| (p.slot, p.operation_kind, p.operation_json.as_str()))
                        != Some((2, 2, "original publication operation"))
                {
                    return Err("takeover lost or replaced original pending slots".into());
                }
                if !matches!(
                    first
                        .checkpoint(&pool, 1, 1, "original fresh operation")
                        .await,
                    Err(DurabilityError::InvalidStoredState)
                ) {
                    return Err("superseded custody was still able to write".into());
                }
                second
                    .checkpoint(&pool, 2, 2, "original publication operation")
                    .await?;
                // Missing storage is corruption, never a new capacity allowance.
                sqlx::query("DELETE FROM game_durability_executor_custody WHERE slot = 2")
                    .execute(&pool)
                    .await?;
                if !matches!(
                    DurabilityCustody::acquire(&pool).await,
                    Err(DurabilityError::InvalidStoredState)
                ) {
                    return Err("missing durable slot created empty replacement capacity".into());
                }
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn fresh_prepare_cannot_invent_control_loss_or_poison_an_unopened_epoch()
-> Result<(), Box<dyn std::error::Error>> {
    use authority_matrix::{Seed, checked};
    use durability::admission_authority_guards::AdmissionGuardStore;
    use durability::fresh_admission::{FreshAdmissionStore, FreshReconciliation};
    use foundation::admission_authority_publication::AdmissionAuthorityPublicationV1;
    use foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1;
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for use_v2 in [false, true] {
                let database = postgres::IsolatedPostgres::create("fresh_first_loss").await?;
                let result = async {
                    let url = database.database_url()?;
                    MigrationExecutor::connect_migration(&url)
                        .await?
                        .apply_embedded_ledger()
                        .await?;
                    let pool = sqlx::PgPool::connect(&url).await?;
                    let now = postgres_clock(&pool).await?;
                    let owner = postgres::fresh::Source::new(now)?;
                    let guards = AdmissionGuardStore::connect_runtime(&url, 8192).await?;
                    guards
                        .publish(&checked(AdmissionAuthorityPublicationV1::prepare(
                            &owner, now,
                        ))?)
                        .await?;
                    let fresh = FreshAdmissionStore::connect_runtime(&url, 65536, 8192).await?;
                    let fresh_request = owner.request()?;
                    if !matches!(
                        fresh.commit(&fresh_request).await?,
                        FreshAdmissionDurableOutcomeV1::Committed(_)
                    ) {
                        return Err("fresh setup failed".into());
                    }
                    let FreshReconciliation::Committed(initial) =
                        fresh.reconcile(fresh_request.operation()).await?
                    else {
                        return Err("missing fresh setup".into());
                    };
                    let claim_keys: Vec<_> =
                        owner.rows[..2].iter().map(|row| row.key.clone()).collect();
                    let claims = guards
                        .load(&claim_keys)
                        .await?
                        .into_iter()
                        .collect::<Option<Vec<_>>>()
                        .ok_or("missing acquired claims")?;
                    // Raw records are independently authored; even matching facts
                    // cannot authorize loss of a healthy current controller.
                    for (attempt, offset) in [(1, -180), (2, 0)] {
                        let seed = Seed {
                            account: "00000000-0000-4000-8000-000000000001",
                            session: 9,
                            character: 2,
                            generation: 1,
                            epoch: 1,
                            attempt,
                            transport: 10,
                            now: now + offset,
                            ..Seed::fixed()
                        };
                        let template = authority_matrix::prepared_record(seed)?;
                        let reconnect = checked(ReconnectDurabilityRecordV1::new(
                            checked(ReconnectIdentityV1::new(
                                authority_matrix::session(9)?,
                                checked(ReconnectAttemptRef::new(attempt))?,
                                seed.account,
                                authority_matrix::character(2)?,
                                authority_matrix::world(3)?,
                                RuntimeScopeRefV1::channel(
                                    authority_matrix::world(3)?,
                                    authority_matrix::channel(4)?,
                                ),
                            ))?,
                            template.connection(),
                            checked(ReconnectAuthorityFenceV1::new(
                                2,
                                checked(ScopeOwnershipGeneration::new(1))?,
                            ))?,
                            template.continuity(),
                            template.proof().clone(),
                            template.fnd02().clone(),
                            template.compatibility().clone(),
                        ))?;
                        let decision_now = postgres_clock(&pool).await?;
                        let expired = reconnect.continuity().prepared_deadline() < decision_now;
                        if expired != (attempt == 1) {
                            return Err(
                                "fresh PREPARE deadline fixture is not expired/current as intended"
                                    .into(),
                            );
                        }
                        let (_, request) = ReconnectDurabilityFlowV1::begin(reconnect.clone());
                        let journal = AdmissionReconnectJournal::connect_runtime(&url).await?;
                        if use_v2 {
                            let (_, request_v2) =
                                foundation::ReconnectDurabilityFlowV2::begin(reconnect, None);
                            let journal_v2 =
                                durability::AdmissionReconnectJournalV2::connect_runtime(&url)
                                    .await?;
                            if journal_v2.prepare(&request_v2).await?
                                != foundation::ReconnectPrepareDispositionV2::RejectedStaleAuthority
                            {
                                return Err("raw V2 prepare invented owning loss authority".into());
                            }
                        } else if journal.prepare(&request).await?
                            != ReconnectPrepareDispositionV1::RejectedStaleAuthority
                        {
                            return Err("raw V1 prepare invented owning loss authority".into());
                        }
                        let FreshReconciliation::Committed(unchanged) =
                            fresh.reconcile(fresh_request.operation()).await?
                        else {
                            return Err("rejected preparation lost fresh authority".into());
                        };
                        if unchanged != initial
                            || guards.load(&claim_keys).await?
                                != claims.iter().cloned().map(Some).collect::<Vec<_>>()
                        {
                            return Err(
                                "rejected prepare altered healthy session/receipt/claims".into()
                            );
                        }
                        let attempts: i64 = sqlx::query_scalar(
                            "SELECT (SELECT count(*) FROM game_durability_reconnect_attempts) + \
                             (SELECT count(*) FROM game_durability_reconnect_pending_commands) + \
                             (SELECT count(*) FROM game_durability_control_loss_continuity)",
                        )
                        .fetch_one(&pool)
                        .await?;
                        if attempts != 0 {
                            return Err("unopened epoch poisoned by retained attempt".into());
                        }
                    }
                    pool.close().await;
                    Ok::<(), Box<dyn std::error::Error>>(())
                }
                .await;
                database.cleanup().await?;
                result?;
            }
            Ok(())
        })
}

#[test]
fn fresh_failure_at_each_effect_rolls_back_claims_receipt_and_reservation()
-> Result<(), Box<dyn std::error::Error>> {
    use durability::admission_authority_guards::AdmissionGuardStore;
    use durability::fresh_admission::{FreshAdmissionStore, FreshReconciliation};
    use foundation::admission_authority_publication::AdmissionAuthorityPublicationV1;
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        // Fixed test-only SQL identifiers. Each trigger interrupts a different
        // tentative effect after an independently valid bootstrap publication.
        for table in [
            "game_durability_fresh_admission_receipts",
            "game_durability_reconnect_sessions",
            "game_durability_admission_account_guards",
            "game_durability_admission_character_guards",
            "game_durability_admission_guard_history",
            "game_durability_transport_ref_reservations",
        ] {
            let database = postgres::IsolatedPostgres::create("fresh_effect_rollback").await?;
            let result = async {
                let url = database.database_url()?;
                MigrationExecutor::connect_migration(&url).await?.apply_embedded_ledger().await?;
                let pool = sqlx::PgPool::connect(&url).await?;
                let now = postgres_clock(&pool).await?;
                let source = postgres::fresh::Source::new(now)?;
                let guards = AdmissionGuardStore::connect_runtime(&url, 8192).await?;
                let publication = authority_matrix::checked(AdmissionAuthorityPublicationV1::prepare(&source, now))?;
                guards.publish(&publication).await?;
                let request = source.request()?;
                let store = FreshAdmissionStore::connect_runtime(&url, 65536, 8192).await?;
                sqlx::raw_sql("CREATE FUNCTION inject_fresh_effect_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected fresh effect failure' USING ERRCODE = 'P0001'; END $$;").execute(&pool).await?;
                sqlx::query(sqlx::AssertSqlSafe(format!("CREATE TRIGGER injected_fresh_effect BEFORE INSERT OR UPDATE ON {table} FOR EACH ROW EXECUTE FUNCTION inject_fresh_effect_failure()"))).execute(&pool).await?;
                let Err(error) = store.commit(&request).await else { return Err("injected effect did not abort".into()); };
                if !matches!(error, DurabilityError::Database(sqlx::Error::Database(ref error)) if error.code().as_deref() == Some("P0001")) {
                    return Err(format!("unexpected failure at {table}: {error}").into());
                }
                let reconciliation = store.reconcile(request.operation()).await?;
                if reconciliation != FreshReconciliation::Absent {
                    return Err(format!("receipt survived failed effect at {table}: {reconciliation:?}").into());
                }
                let keys: Vec<_> = source.rows.iter().map(|row| row.key.clone()).collect();
                if guards.load(&keys).await? != source.rows.iter().cloned().map(Some).collect::<Vec<_>>() {
                    return Err(format!("partial claims at {table}").into());
                }
                let counts: (i64, i64, i64, i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM game_durability_fresh_admission_receipts), (SELECT COUNT(*) FROM game_durability_reconnect_sessions), (SELECT COUNT(*) FROM game_durability_transport_ref_reservations), (SELECT COUNT(*) FROM game_durability_admission_guard_history)").fetch_one(&pool).await?;
                if counts != (0, 0, 0, 4) {
                    return Err(format!("partial durable effects at {table}: expected (0, 0, 0, 4), got {counts:?}").into());
                }
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }.await;
            database.cleanup().await?;
            result?;
        }
        Ok(())
    })
}

#[test]
fn fresh_commit_persists_complete_operation_and_reconciles_original_decision()
-> Result<(), Box<dyn std::error::Error>> {
    use durability::admission_authority_guards::AdmissionGuardStore;
    use durability::fresh_admission::{FreshAdmissionStore, FreshReconciliation};
    use foundation::GameSessionState;
    use foundation::admission_authority_publication::AdmissionAuthorityPublicationV1;
    use foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1;
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let database = postgres::IsolatedPostgres::create("fresh_commit_original_l").await?;
        let result = async {
            let url = database.database_url()?;
            MigrationExecutor::connect_migration(&url).await?.apply_embedded_ledger().await?;
            let pool = sqlx::PgPool::connect(&url).await?;
            let now = postgres_clock(&pool).await?;
            let source = postgres::fresh::Source::new(now)?;
            let guards = AdmissionGuardStore::connect_runtime(&url, 8192).await?;
            let publication = authority_matrix::checked(AdmissionAuthorityPublicationV1::prepare(&source, now))?;
            let request = source.request()?;
            let store = FreshAdmissionStore::connect_runtime(&url, 65536, 8192).await?;
            let mut partial_source = postgres::fresh::Source::new(now)?;
            partial_source.rows.pop(); // Only independently current signing trust is absent.
            guards.publish(&authority_matrix::checked(AdmissionAuthorityPublicationV1::prepare(&partial_source, now))?).await?;
            assert_eq!(store.commit(&request).await?, FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority);
            assert_eq!(store.reconcile(request.operation()).await?, FreshReconciliation::Absent);
            guards.publish(&publication).await?;
            let FreshAdmissionDurableOutcomeV1::Committed(receipt) = store.commit(&request).await? else { return Err("fresh commit did not commit".into()); };
            assert_eq!(receipt.operation(), request.operation());
            assert!(receipt.decided_at() >= now);
            assert_eq!(store.commit(&request).await?, FreshAdmissionDurableOutcomeV1::ExistingCommitted(receipt.clone()));
            let restarted = FreshAdmissionStore::connect_runtime(&url, 65536, 8192).await?;
            let FreshReconciliation::Committed(snapshot) = restarted.reconcile(request.operation()).await? else { return Err("missing committed receipt".into()); };
            assert_eq!(snapshot.receipt, receipt);
            assert_eq!(snapshot.current_session.session_state(), GameSessionState::Active);
            assert_eq!(snapshot.current_session.current_connection_generation().get(), 1);
            assert!(snapshot.current_session.current_control_loss_epoch().is_none());
            let current = guards.load(&source.rows.iter().map(|row| row.key.clone()).collect::<Vec<_>>()).await?;
            assert_eq!(current[0].as_ref(), Some(&request.operation().transition.successors[0]));
            assert_eq!(current[1].as_ref(), Some(&request.operation().transition.successors[1]));
            let mut changed = request.operation().clone();
            changed.transition.successors[0].source.decision_identity = "different-historical-operation".into();
            assert_eq!(restarted.reconcile(&changed).await?, FreshReconciliation::Conflict);
            let receipts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM game_durability_fresh_admission_receipts").fetch_one(&pool).await?;
            let sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM game_durability_reconnect_sessions").fetch_one(&pool).await?;
            let transports: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM game_durability_transport_ref_reservations WHERE reservation_owner = 2").fetch_one(&pool).await?;
            assert_eq!((receipts, sessions, transports), (1, 1, 1));
            pool.close().await;
            Ok::<(), Box<dyn std::error::Error>>(())
        }.await;
        database.cleanup().await?;
        result
    })
}

#[test]
fn guard_publication_is_atomic_replayable_and_retains_decision_history()
-> Result<(), Box<dyn std::error::Error>> {
    use durability::admission_authority_guards::{
        AdmissionGuardStore, GuardPublicationDisposition,
    };
    use foundation::admission_authority_publication::*;
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("guard_publication").await?;
            let result = async {
                let url = database.database_url()?;
                MigrationExecutor::connect_migration(&url)
                    .await?
                    .apply_embedded_ledger()
                    .await?;
                let pool = sqlx::PgPool::connect(&url).await?;
                let now = postgres_clock(&pool).await?;
                let mut source = postgres::fresh::Source::new(now)?;
                // Explicit test allocation, not a selected production resource default.
                let store = AdmissionGuardStore::connect_runtime(&url, 8192).await?;
                let request = authority_matrix::checked(AdmissionAuthorityPublicationV1::prepare(
                    &source, now,
                ))?;
                assert_eq!(
                    store.publish(&request).await?,
                    GuardPublicationDisposition::Applied
                );
                assert_eq!(
                    store.publish(&request).await?,
                    GuardPublicationDisposition::Existing
                );
                let keys: Vec<_> = source.rows.iter().map(|row| row.key.clone()).collect();
                assert_eq!(
                    store.load(&keys).await?,
                    source.rows.iter().cloned().map(Some).collect::<Vec<_>>()
                );
                for row in &mut source.rows {
                    row.publication_revision = 2;
                    row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
                        expected_publication_revision: 1,
                    };
                    row.source.source_revision = 2;
                    row.source.decision_identity = "next-independent-decision".into();
                    if let AdmissionAuthorityGuardStateV1::Account { security, .. } = &mut row.state
                    {
                        security.provenance.publication_revision = 2;
                    }
                }
                // Independently valid full-u64 source/runtime fences survive SQL
                // NUMERIC storage and exact textual mirror reconstruction.
                source.rows[2].source.source_revision = u64::MAX;
                if let AdmissionAuthorityGuardStateV1::Runtime {
                    ownership_generation,
                    ..
                } = &mut source.rows[2].state
                {
                    *ownership_generation = u64::MAX;
                }
                // Reusing an accepted decision for different effects must reject the
                // entire batch, including otherwise valid changes preceding it.
                source.rows[3].source.decision_identity = "platform-observation-1".into();
                let conflicting = authority_matrix::checked(
                    AdmissionAuthorityPublicationV1::prepare(&source, now),
                )?;
                assert_eq!(
                    store.publish(&conflicting).await?,
                    GuardPublicationDisposition::Conflict
                );
                assert_eq!(
                    store.load(&keys).await?,
                    request
                        .changes()
                        .iter()
                        .cloned()
                        .map(Some)
                        .collect::<Vec<_>>()
                );
                source.rows[3].source.decision_identity = "next-independent-decision".into();
                let successor = authority_matrix::checked(
                    AdmissionAuthorityPublicationV1::prepare(&source, now),
                )?;
                assert_eq!(
                    store.publish(&successor).await?,
                    GuardPublicationDisposition::Applied
                );
                assert_eq!(
                    store.publish(&request).await?,
                    GuardPublicationDisposition::Stale
                );
                let restarted = AdmissionGuardStore::connect_runtime(&url, 8192).await?;
                assert_eq!(
                    restarted.load(&keys).await?,
                    source.rows.iter().cloned().map(Some).collect::<Vec<_>>()
                );
                let history: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM game_durability_admission_guard_history",
                )
                .fetch_one(&pool)
                .await?;
                assert_eq!(history, 8);
                // Observe the exact production SELECT before decoding/mirror
                // validation. Old per-mirror rejection cannot satisfy this test.
                if restarted.projected_guard_presence(&keys[1]).await? != (true, true) {
                    return Err("bounded positive guard projection missing".into());
                }
                let overhead: i64 = sqlx::query_scalar("SELECT (octet_length(to_jsonb(g)::text) - octet_length(source_authority))::bigint FROM game_durability_admission_character_guards g").fetch_one(&pool).await?;
                for (size, presence) in [(131072_i64, (true, true)), (131073, (false, false))] {
                    let padding = size.checked_sub(overhead).and_then(|n| i32::try_from(n).ok()).filter(|n| *n > 0).ok_or("invalid complete-row boundary fixture")?;
                    sqlx::query("UPDATE game_durability_admission_character_guards SET source_authority = repeat('x', $1)").bind(padding).execute(&pool).await?;
                    let actual: i64 = sqlx::query_scalar("SELECT octet_length(to_jsonb(g)::text)::bigint FROM game_durability_admission_character_guards g").fetch_one(&pool).await?;
                    if actual != size || restarted.projected_guard_presence(&keys[1]).await? != presence {
                        return Err(format!("guard SQL complete-row boundary failed: wanted {size}, actual {actual}").into());
                    }
                    if !matches!(restarted.load(&keys).await, Err(DurabilityError::InvalidStoredState)) {
                        return Err("corrupt mirror passed full guard consistency checks".into());
                    }
                }
                sqlx::query("UPDATE game_durability_admission_character_guards SET source_authority = $1").bind(&source.rows[1].source.authority).execute(&pool).await?;
                if restarted.load(&keys).await? != source.rows.iter().cloned().map(Some).collect::<Vec<_>>() {
                    return Err("bounded guard row failed after restoring its exact mirror".into());
                }
                // Isolated administrator corrupts one mirror; decoded history
                // must not override the independently stored eligibility field.
                sqlx::query(
                    "UPDATE game_durability_admission_character_guards SET eligible = NOT eligible",
                )
                .execute(&pool)
                .await?;
                assert!(matches!(
                    restarted.load(&keys).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn fresh_sealed_fixture_prepares_complete_owner_operation() -> Result<(), Box<dyn std::error::Error>>
{
    let source = postgres::fresh::Source::new(100)?;
    let request = source.request()?;
    authority_matrix::checked(request.operation().validate_historical(100))?;
    let rows: Vec<_> = source.rows.iter().cloned().map(Some).collect();
    assert_eq!(
        authority_matrix::checked(request.validate_at_decision(&rows, Some(100)))?,
        request.operation().transition.successors,
    );
    let mut independently_changed_rows = rows;
    independently_changed_rows[0] = None;
    assert!(
        request
            .validate_at_decision(&independently_changed_rows, Some(100))
            .is_err()
    );
    Ok(())
}

#[test]
fn fresh_operation_codec_retains_effects_and_rejects_trailing_or_oversized_storage()
-> Result<(), Box<dyn std::error::Error>> {
    use durability::fresh_admission::{decode_operation, encode_operation, encoded_operation_size};
    let source = postgres::fresh::Source::new(100)?;
    let request = source.request()?;
    // Test allocation budget; no production resource ceiling is selected here.
    let budget = 65_536;
    let encoded = encode_operation(request.operation(), budget)?;
    assert_eq!(
        encoded_operation_size(request.operation(), budget)?,
        encoded.len()
    );
    assert!(encoded_operation_size(request.operation(), encoded.len() - 1).is_err());
    assert_eq!(
        encode_operation(request.operation(), encoded.len())?,
        encoded
    );
    assert_eq!(decode_operation(&encoded, budget)?, *request.operation());
    assert!(decode_operation(&encoded, encoded.len() - 1).is_err());
    assert!(encode_operation(request.operation(), encoded.len() - 1).is_err());
    assert!(decode_operation(&format!("{encoded}x"), budget).is_err());
    let duplicate = encoded.replacen("\"version\":1", "\"version\":1,\"version\":1", 1);
    assert!(decode_operation(&duplicate, budget).is_err());
    let mut different_effect = request.operation().clone();
    different_effect.transition.successors[0]
        .source
        .decision_identity = "another-exact-decision".into();
    let other = encode_operation(&different_effect, budget)?;
    assert_ne!(other, encoded);
    assert_eq!(encode_operation(&different_effect, other.len())?, other);
    assert_eq!(decode_operation(&other, budget)?, different_effect);
    Ok(())
}

#[test]
fn fresh_guard_codec_preserves_full_u64_and_rejects_invalid_binary()
-> Result<(), Box<dyn std::error::Error>> {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use durability::admission_authority_guards::{decode_guard, encode_guard, encoded_guard_size};
    use foundation::admission_authority_publication::AdmissionPublicationPreconditionV1;
    let source = postgres::fresh::Source::new(100)?;
    let budget = 65_536;
    for original in &source.rows {
        let encoded = encode_guard(original, budget)?;
        println!(
            "guard fixture {:?}: {} encoded bytes",
            original.source.purpose,
            encoded.len()
        );
        assert_eq!(encoded_guard_size(original, budget)?, encoded.len());
        assert!(encoded_guard_size(original, encoded.len() - 1).is_err());
        assert_eq!(decode_guard(&encoded, budget)?, *original);
        let mut maximum = original.clone();
        maximum.publication_revision = u64::MAX;
        maximum.source.source_revision = u64::MAX;
        maximum.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: u64::MAX - 1,
        };
        let encoded = encode_guard(&maximum, budget)?;
        assert_eq!(decode_guard(&encoded, budget)?, maximum);
        let envelope: serde_json::Value = serde_json::from_str(&encoded)?;
        let payload = envelope["payload"].as_str().ok_or("payload missing")?;
        let mut bytes = URL_SAFE_NO_PAD.decode(payload)?;
        bytes.push(0);
        let trailing = format!(
            "{{\"version\":1,\"payload\":\"{}\"}}",
            URL_SAFE_NO_PAD.encode(&bytes)
        );
        assert!(decode_guard(&trailing, budget).is_err());
        bytes.truncate(1);
        let truncated = format!(
            "{{\"version\":1,\"payload\":\"{}\"}}",
            URL_SAFE_NO_PAD.encode(&bytes)
        );
        assert!(decode_guard(&truncated, budget).is_err());
    }
    Ok(())
}

#[test]
fn fresh_admission_forward_schema_supports_truthful_atomic_session()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("fresh_forward_schema").await?;
            let result = async {
                let url = database.database_url()?;
                MigrationExecutor::connect_migration(&url)
                    .await?
                    .apply_embedded_ledger()
                    .await?;
                let pool = sqlx::PgPool::connect(&url).await?;
                let receipt_exists: bool = sqlx::query_scalar(
                    "SELECT to_regclass('public.game_durability_fresh_admission_receipts') IS NOT NULL",
                )
                .fetch_one(&pool)
                .await?;
                assert!(receipt_exists, "fresh atomic admission requires its immutable receipt table");
                // A separately restricted runtime role cannot erase a permanent
                // legacy reservation or disable its immutability enforcement.
                sqlx::query("INSERT INTO game_durability_transport_ref_reservations \
                    (transport_ref, game_session_id, reconnect_attempt_ref) \
                    VALUES ($1, encode($2, 'hex')::uuid, $3)")
                    .bind([0x19_u8; 16].as_slice())
                    .bind(authority_matrix::uuid(9).as_slice())
                    .bind([0x19_u8; 8].as_slice())
                    .execute(&pool).await?;
                let role = format!("oteryn_b329_runtime_{}", std::process::id());
                let mut role_test = pool.begin().await?;
                sqlx::query(sqlx::AssertSqlSafe(format!("CREATE ROLE {role} NOLOGIN")))
                    .execute(&mut *role_test).await?;
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "GRANT SELECT, UPDATE, DELETE ON game_durability_transport_ref_reservations TO {role}")))
                    .execute(&mut *role_test).await?;
                sqlx::query(sqlx::AssertSqlSafe(format!("SET LOCAL ROLE {role}")))
                    .execute(&mut *role_test).await?;
                for (statement, expected_code) in [
                    ("DELETE FROM game_durability_transport_ref_reservations", "23514"),
                    ("ALTER TABLE game_durability_transport_ref_reservations DISABLE TRIGGER game_transport_reservation_immutable", "42501"),
                ] {
                    sqlx::query("SAVEPOINT denied_mutation").execute(&mut *role_test).await?;
                    let denied = sqlx::query(statement).execute(&mut *role_test).await;
                    let code = denied.as_ref().err().and_then(sqlx::Error::as_database_error)
                        .and_then(|error| error.code());
                    assert_eq!(code.as_deref(), Some(expected_code), "{statement}: {denied:?}");
                    sqlx::query("ROLLBACK TO SAVEPOINT denied_mutation").execute(&mut *role_test).await?;
                }
                // Transactional role creation and grants disappear even if the test fails.
                role_test.rollback().await?;
                for column in ["control_loss_epoch", "original_grace_deadline", "predecessor_generation"] {
                    let nullable: String = sqlx::query_scalar(
                        "SELECT is_nullable FROM information_schema.columns \
                         WHERE table_schema = 'public' \
                           AND table_name = 'game_durability_reconnect_sessions' \
                           AND column_name = $1",
                    )
                    .bind(column)
                    .fetch_one(&pool)
                    .await?;
                    assert_eq!(nullable, "YES", "fresh ACTIVE cannot fabricate {column}");
                }
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }.await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn independent_authority_matrix_rejects_mutations_after_postgres_reload()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database =
                postgres::IsolatedPostgres::create("independent_authority_matrix").await?;
            let result = async {
                use authority_matrix::{LiveSource, Seed, checked, prepared_record};
                let url = database.database_url()?;
                MigrationExecutor::connect_migration(&url)
                    .await?
                    .apply_embedded_ledger()
                    .await?;
                let seed = Seed {
                    now: unix_now().map_err(foundation_error)?,
                    ..Seed::fixed()
                };
                let record = prepared_record(seed)?;
                let source = LiveSource::read(seed);
                let journal = AdmissionReconnectJournal::connect_runtime(&url).await?;
                let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
                let prepared = journal.prepare(&request).await?;
                assert_eq!(prepared, ReconnectPrepareDispositionV1::Prepared);
                checked(flow.accept_prepare_completion(
                    ReconnectPrepareCompletionV1::for_request(&request, prepared),
                ))?;
                let commit = checked(flow.authorize_commit(source.bind(&record)?, seed.now + 2))?;
                assert_eq!(
                    journal.commit(&commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );
                drop(journal);
                let reloaded = AdmissionReconnectJournal::connect_runtime(&url).await?;
                let v1 = reloaded.reconcile(&request).await?;
                let typed = durability::AdmissionReconnectJournalV2::connect_runtime(&url).await?;
                let (_, request_v2) =
                    oteryn_game_server::foundation::ReconnectDurabilityFlowV2::begin(
                        record.clone(),
                        None,
                    );
                let v2 = typed.reconcile(&request_v2).await?;
                authority_matrix::run_loaded_matrix(seed, &record, &source, v1, v2)?;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

use durability::{
    AdmissionReconnectJournal, DurabilityError, MigrationExecutor, SchemaCompatibility,
};
use oteryn_game_server::foundation::{
    AuthenticatedTransportRefV1, AuthorityEvidenceFenceV1, ChannelId, CharacterId, CommandId,
    ConnectionGeneration, ControlLossEpochRefV1, Fnd02ReconciliationFenceV1, GameSessionId,
    PendingCommandDispositionV1, PendingCommandReconciliationV1, ProtectionEntitlementV1,
    ReconnectAttemptRef, ReconnectAuthorityFenceV1, ReconnectCommitActionV1,
    ReconnectCommitCompletionV1, ReconnectCommitDispositionV1, ReconnectCompatibilityEvidenceV1,
    ReconnectConnectionFenceV1, ReconnectContinuityV1, ReconnectDurabilityErrorV1,
    ReconnectDurabilityFlowV1, ReconnectDurabilityRecordV1,
    ReconnectDurableReconciliationSnapshotV1, ReconnectIdentityV1, ReconnectPrepareActionV1,
    ReconnectPrepareCompletionV1, ReconnectPrepareDispositionV1, ReconnectProjectionDecisionV1,
    ReconnectProofV1, RuntimeScopeRefV1, ScopeOwnershipGeneration, StateDomainRevisionV1, WorldId,
};
use postgres::current_authority_from_record;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type CrossEpochSessionRow = (
    i64,
    i64,
    i64,
    i64,
    Option<Vec<u8>>,
    i16,
    i16,
    Option<Vec<u8>>,
);
type ProtectionContinuityRow = (
    i16,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i64>,
    i16,
    Option<String>,
);

fn foundation_error(error: ReconnectDurabilityErrorV1) -> std::io::Error {
    std::io::Error::other(format!(
        "Foundation V1 record construction failed: {error:?}"
    ))
}

fn unix_now() -> Result<i64, ReconnectDurabilityErrorV1> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?
        .as_secs()
        .try_into()
        .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)
}

async fn postgres_clock(pool: &sqlx::PgPool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT FLOOR(EXTRACT(EPOCH FROM clock_timestamp()))::BIGINT")
        .fetch_one(pool)
        .await
}

fn uuid_v7(raw: u64) -> [u8; 16] {
    let mut value = [0u8; 16];
    value[8..].copy_from_slice(&raw.to_be_bytes());
    value[6] = 0x70;
    value[8] = (value[8] & 0x3f) | 0x80;
    value
}

fn record(
    game_session_raw: u64,
    attempt_raw: u64,
    transport_byte: u8,
    now: i64,
) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
    record_with_prepared_deadline(
        game_session_raw,
        attempt_raw,
        transport_byte,
        now,
        now + 115,
    )
}

fn record_with_prepared_deadline(
    game_session_raw: u64,
    attempt_raw: u64,
    transport_byte: u8,
    now: i64,
    prepared_deadline: i64,
) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
    record_for_epoch(
        game_session_raw,
        attempt_raw,
        transport_byte,
        now,
        prepared_deadline,
        3,
        7,
        8,
        0x55,
    )
}

#[allow(clippy::too_many_arguments)]
fn record_for_epoch(
    game_session_raw: u64,
    attempt_raw: u64,
    transport_byte: u8,
    now: i64,
    prepared_deadline: i64,
    control_loss_epoch: u64,
    predecessor_generation: u64,
    candidate_generation: u64,
    recovery_nonce_byte: u8,
) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
    record_for_epoch_with_protection(
        game_session_raw,
        attempt_raw,
        transport_byte,
        now,
        prepared_deadline,
        control_loss_epoch,
        predecessor_generation,
        candidate_generation,
        recovery_nonce_byte,
        ProtectionEntitlementV1::unused(),
    )
}

fn record_for_actor(
    game_session_raw: u64,
    character_raw: u64,
    attempt_raw: u64,
    transport_byte: u8,
    now: i64,
) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
    record_for_actor_epoch_with_protection(
        game_session_raw,
        character_raw,
        attempt_raw,
        transport_byte,
        now,
        now + 115,
        3,
        7,
        8,
        0x55,
        ProtectionEntitlementV1::unused(),
    )
}

#[allow(clippy::too_many_arguments)]
fn record_for_epoch_with_protection(
    game_session_raw: u64,
    attempt_raw: u64,
    transport_byte: u8,
    now: i64,
    prepared_deadline: i64,
    control_loss_epoch: u64,
    predecessor_generation: u64,
    candidate_generation: u64,
    recovery_nonce_byte: u8,
    protection_entitlement: ProtectionEntitlementV1,
) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
    record_for_actor_epoch_with_protection(
        game_session_raw,
        11,
        attempt_raw,
        transport_byte,
        now,
        prepared_deadline,
        control_loss_epoch,
        predecessor_generation,
        candidate_generation,
        recovery_nonce_byte,
        protection_entitlement,
    )
}

#[allow(clippy::too_many_arguments)]
fn record_for_actor_epoch_with_protection(
    game_session_raw: u64,
    character_raw: u64,
    attempt_raw: u64,
    transport_byte: u8,
    now: i64,
    prepared_deadline: i64,
    control_loss_epoch: u64,
    predecessor_generation: u64,
    candidate_generation: u64,
    recovery_nonce_byte: u8,
    protection_entitlement: ProtectionEntitlementV1,
) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
    let game_session_id = GameSessionId::decode(&uuid_v7(game_session_raw))
        .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let character_id = CharacterId::decode(&uuid_v7(character_raw))
        .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let world_id = WorldId::decode(&uuid_v7(12))
        .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let channel_id = ChannelId::decode(&uuid_v7(13))
        .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let identity = ReconnectIdentityV1::new(
        game_session_id,
        ReconnectAttemptRef::new(attempt_raw)
            .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
        &postgres::fixture_account_for_character(character_raw),
        character_id,
        world_id,
        RuntimeScopeRefV1::channel(world_id, channel_id),
    )?;
    let connection = ReconnectConnectionFenceV1::new(
        ConnectionGeneration::new(predecessor_generation)
            .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
        ConnectionGeneration::new(candidate_generation)
            .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
        AuthenticatedTransportRefV1::decode(&[transport_byte; 16])
            .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
    )?;
    let authority = ReconnectAuthorityFenceV1::new(
        9,
        ScopeOwnershipGeneration::new(10)
            .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
    )?;
    let continuity = ReconnectContinuityV1::new(
        ControlLossEpochRefV1::new(control_loss_epoch)?,
        now + 120,
        prepared_deadline,
        protection_entitlement,
    )?;
    let fnd02 = Fnd02ReconciliationFenceV1::new(
        CommandId::new(3).map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
        vec![
            PendingCommandReconciliationV1::new(
                CommandId::new(1).map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
                PendingCommandDispositionV1::PendingOriginal,
            ),
            PendingCommandReconciliationV1::new(
                CommandId::new(2).map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
                PendingCommandDispositionV1::TerminalOutcomeRetained,
            ),
        ],
        41,
        vec![
            StateDomainRevisionV1::new(1, 4)?,
            StateDomainRevisionV1::new(2, 7)?,
        ],
    )?;
    let platform = AuthorityEvidenceFenceV1::new(
        "platform-security",
        "reconnect",
        "account",
        "sec:17",
        "decision:sec:17",
        now,
    )?;
    let trust = AuthorityEvidenceFenceV1::new(
        "proof-trust",
        "reconnect",
        "recovery-key",
        "trust:21",
        "decision:trust:21",
        now,
    )?;
    let compatibility = ReconnectCompatibilityEvidenceV1::new(
        1,
        1,
        "rules:1",
        "content:2",
        "map:3",
        "world:4",
        12,
        platform,
        trust,
        Some(now + 110),
    )?;
    ReconnectDurabilityRecordV1::new(
        identity,
        connection,
        authority,
        continuity,
        ReconnectProofV1::ReauthenticatedRecovery {
            recovery_grant_nonce: [recovery_nonce_byte; 32],
        },
        fnd02,
        compatibility,
    )
}

fn postgres_e2e_is_configured() -> Result<bool, Box<dyn std::error::Error>> {
    match postgres::postgres_e2e_availability()? {
        postgres::PostgresE2eAvailability::Configured => Ok(true),
        postgres::PostgresE2eAvailability::NotConfigured => {
            eprintln!(
                "PostgreSQL E2E NOT_APPLICABLE: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured; real database assertions run in the dedicated configured harness"
            );
            Ok(false)
        }
    }
}

#[test]
fn isolated_postgres_guard_classifies_absence_and_rejects_unsafe_configuration()
-> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        postgres::classify_e2e_admin_url(None)?,
        postgres::PostgresE2eAvailability::NotConfigured
    );
    assert_eq!(
        postgres::classify_e2e_admin_url(Some(
            "postgresql://oteryn_test_admin:secret@127.0.0.1:5432/postgres"
        ))?,
        postgres::PostgresE2eAvailability::Configured
    );
    assert!(matches!(
        postgres::classify_e2e_admin_url(Some(
            "postgresql://oteryn_test_admin:secret@remote.example/postgres"
        )),
        Err(postgres::IsolatedPostgresError::UnsafeAdminUrl)
    ));
    assert!(
        postgres::validate_admin_url(
            "postgresql://oteryn_test_admin:secret@remote.example/postgres"
        )
        .is_err()
    );
    assert!(
        postgres::validate_admin_url(
            "postgresql://oteryn_test_admin:secret@127.0.0.1:5432/postgres?host=/tmp"
        )
        .is_err()
    );
    assert!(postgres::validate_admin_url(
        "postgresql://oteryn_test_admin:secret@127.0.0.1:5432/postgres?options=-c%20search_path%3Dpublic"
    )
    .is_err());
    Ok(())
}

#[test]
fn fresh_migration_applies_only_the_embedded_game_ledger() -> Result<(), Box<dyn std::error::Error>>
{
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("fresh_migration").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                assert_eq!(executor.inspect().await?, SchemaCompatibility::Compatible);
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn same_prepare_replay_returns_the_existing_durable_disposition()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("same_attempt_replay").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let (_flow, request) = ReconnectDurabilityFlowV1::begin(
                    record(10, 1, 0x11, unix_now().map_err(foundation_error)?)
                        .map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&request).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                assert_eq!(
                    journal.prepare(&request).await?,
                    ReconnectPrepareDispositionV1::ExistingPrepared
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn same_prepare_replay_survives_process_replacement() -> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            if std::env::var_os("OTERYN_DURABILITY_REPLAY_CHILD").is_some() {
                let database_url = std::env::var("OTERYN_DURABILITY_REPLAY_DATABASE_URL")?;
                let record_now = std::env::var("OTERYN_DURABILITY_REPLAY_RECORD_NOW")?.parse()?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let (_flow, request) = ReconnectDurabilityFlowV1::begin(
                    record(11, 2, 0x22, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&request).await?,
                    ReconnectPrepareDispositionV1::ExistingPrepared
                );
                return Ok(());
            }

            let database = postgres::IsolatedPostgres::create("cross_process_replay").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (_flow, request) = ReconnectDurabilityFlowV1::begin(
                    record(11, 2, 0x22, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&request).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );

                let status = Command::new(std::env::current_exe()?)
                    .arg("--exact")
                    .arg("same_prepare_replay_survives_process_replacement")
                    .env("OTERYN_DURABILITY_REPLAY_CHILD", "1")
                    .env("OTERYN_DURABILITY_REPLAY_DATABASE_URL", &database_url)
                    .env(
                        "OTERYN_DURABILITY_REPLAY_RECORD_NOW",
                        record_now.to_string(),
                    )
                    .status()?;
                assert!(
                    status.success(),
                    "fresh process did not replay the durable disposition"
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn transport_ref_collision_is_durable_and_same_attempt_replays_terminal()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("transport_ref_collision").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (_first_flow, first) = ReconnectDurabilityFlowV1::begin(
                    record(20, 1, 0x33, record_now).map_err(foundation_error)?,
                );
                let (_colliding_flow, colliding) = ReconnectDurabilityFlowV1::begin(
                    record_for_actor(21, 121, 1, 0x33, record_now).map_err(foundation_error)?,
                );

                assert_eq!(
                    journal.prepare(&first).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                assert_eq!(
                    journal.prepare(&colliding).await?,
                    ReconnectPrepareDispositionV1::RejectedTransportRefCollision
                );
                assert_eq!(
                    journal.prepare(&colliding).await?,
                    ReconnectPrepareDispositionV1::ExistingTerminal
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn reconnect_account_incumbent_is_a_stale_denial_without_candidate_effects()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("account_incumbent_denial").await?;
            let result = async {
                let url = database.database_url()?;
                MigrationExecutor::connect_migration(&url)
                    .await?
                    .apply_embedded_ledger()
                    .await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&url).await?;
                let pool = sqlx::PgPool::connect(&url).await?;
                let now = postgres_clock(&pool).await?;
                let (_, first) = ReconnectDurabilityFlowV1::begin(
                    record(20, 1, 0x33, now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&first).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                let candidate =
                    record_for_actor(21, 121, 1, 0x34, now).map_err(foundation_error)?;
                let identity = candidate.identity();
                // Independent fixture account intent deliberately targets the already
                // occupied account; current authority comes from the locked DB row.
                let same_account = ReconnectIdentityV1::new(
                    identity.game_session_id(),
                    identity.reconnect_attempt_ref(),
                    "123e4567-e89b-12d3-a456-426614174000",
                    identity.character_id(),
                    identity.world_id(),
                    identity.runtime_scope(),
                )
                .map_err(foundation_error)?;
                let candidate = ReconnectDurabilityRecordV1::new(
                    same_account,
                    candidate.connection(),
                    candidate.authority(),
                    candidate.continuity(),
                    candidate.proof().clone(),
                    candidate.fnd02().clone(),
                    candidate.compatibility().clone(),
                )
                .map_err(foundation_error)?;
                let (_, request) = ReconnectDurabilityFlowV1::begin(candidate);
                assert_eq!(
                    journal.prepare(&request).await?,
                    ReconnectPrepareDispositionV1::RejectedStaleAuthority
                );
                let sessions: i64 =
                    sqlx::query_scalar("SELECT COUNT(*) FROM game_durability_reconnect_sessions")
                        .fetch_one(&pool)
                        .await?;
                let references: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM game_durability_transport_ref_reservations",
                )
                .fetch_one(&pool)
                .await?;
                assert_eq!((sessions, references), (1, 1));
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn one_prepared_attempt_and_eight_attempt_epoch_limits_are_enforced_in_postgres()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("attempt_capacity").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (_first_flow, first) = ReconnectDurabilityFlowV1::begin(
                    record(30, 1, 0x41, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&first).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );

                for attempt in 2_u64..=8 {
                    let transport = u8::try_from(0x40_u64 + attempt)?;
                    let (_flow, request) = ReconnectDurabilityFlowV1::begin(
                        record(30, attempt, transport, record_now).map_err(foundation_error)?,
                    );
                    assert_eq!(
                        journal.prepare(&request).await?,
                        ReconnectPrepareDispositionV1::RejectedConcurrentPrepared
                    );
                }
                let (_ninth_flow, ninth) = ReconnectDurabilityFlowV1::begin(
                    record(30, 9, 0x49, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&ninth).await?,
                    ReconnectPrepareDispositionV1::AttemptCapacityExceeded
                );
                assert_eq!(
                    journal.prepare(&first).await?,
                    ReconnectPrepareDispositionV1::ExistingPrepared
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn same_attempt_with_changed_record_conflicts_without_consuming_the_new_ref()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("idempotency_conflict").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (_first_flow, first) = ReconnectDurabilityFlowV1::begin(
                    record(40, 1, 0x51, record_now).map_err(foundation_error)?,
                );
                let (_changed_flow, changed) = ReconnectDurabilityFlowV1::begin(
                    record(40, 1, 0x52, record_now).map_err(foundation_error)?,
                );
                let (_new_flow, new_attempt) = ReconnectDurabilityFlowV1::begin(
                    record_for_actor(41, 141, 1, 0x52, record_now).map_err(foundation_error)?,
                );

                assert_eq!(
                    journal.prepare(&first).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                assert_eq!(
                    journal.prepare(&changed).await?,
                    ReconnectPrepareDispositionV1::IdempotencyConflict
                );
                assert_eq!(
                    journal.prepare(&new_attempt).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn concurrent_same_attempt_reconciles_to_one_prepared_and_one_existing_prepared()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("same_attempt_race").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let (_flow, request) = ReconnectDurabilityFlowV1::begin(
                    record(50, 1, 0x61, unix_now().map_err(foundation_error)?)
                        .map_err(foundation_error)?,
                );
                let barrier = Arc::new(tokio::sync::Barrier::new(2));
                let first = {
                    let barrier = Arc::clone(&barrier);
                    let journal = journal.clone();
                    let request = request.clone();
                    tokio::spawn(async move {
                        barrier.wait().await;
                        journal.prepare(&request).await
                    })
                };
                let second = {
                    let barrier = Arc::clone(&barrier);
                    let journal = journal.clone();
                    let request = request.clone();
                    tokio::spawn(async move {
                        barrier.wait().await;
                        journal.prepare(&request).await
                    })
                };
                let mut dispositions = [first.await??, second.await??];
                dispositions.sort_unstable_by_key(|disposition| match disposition {
                    ReconnectPrepareDispositionV1::Prepared => 0,
                    ReconnectPrepareDispositionV1::ExistingPrepared => 1,
                    _ => 2,
                });
                assert_eq!(
                    dispositions,
                    [
                        ReconnectPrepareDispositionV1::Prepared,
                        ReconnectPrepareDispositionV1::ExistingPrepared,
                    ]
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn exact_prepared_attempt_commits_once_and_reconciles_after_response_loss()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("commit_reconcile").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (mut flow, prepare) = ReconnectDurabilityFlowV1::begin(
                    record(60, 1, 0x71, record_now).map_err(foundation_error)?,
                );

                assert_eq!(
                    journal.prepare(&prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                assert_eq!(
                    flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                        &prepare,
                        ReconnectPrepareDispositionV1::Prepared,
                    ))
                    .map_err(foundation_error)?,
                    ReconnectPrepareActionV1::AwaitFinalRevalidation
                );
                let current = current_authority_from_record(prepare.record(), record_now)
                    .map_err(foundation_error)?;
                let commit = flow
                    .authorize_commit(current, record_now)
                    .map_err(foundation_error)?;

                assert_eq!(
                    journal.commit(&commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );
                assert_eq!(
                    journal.commit(&commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );
                drop(journal);
                let recovered_journal =
                    AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                assert_eq!(
                    flow.accept_commit_completion(ReconnectCommitCompletionV1::for_request(
                        &commit,
                        ReconnectCommitDispositionV1::Committed,
                    ))
                    .map_err(foundation_error)?,
                    ReconnectCommitActionV1::ReconcileSameAttempt
                );
                assert_eq!(
                    flow.accept_reconciliation(
                        recovered_journal.reconcile(&prepare).await?,
                        current_authority_from_record(prepare.record(), record_now)
                            .map_err(foundation_error)?,
                    )
                    .map_err(foundation_error)?,
                    ReconnectProjectionDecisionV1::InstallController {
                        generation: ConnectionGeneration::new(8).map_err(|_error| {
                            std::io::Error::other("invalid connection generation")
                        })?,
                        transport_ref: AuthenticatedTransportRefV1::decode(&[0x71; 16])
                            .map_err(|_error| std::io::Error::other("invalid transport ref"))?,
                    }
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn expired_prepared_replay_requires_exact_incumbent_binding()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("expired_incumbent_binding").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let pool = sqlx::PgPool::connect(&database_url).await?;
                let record_now = postgres_clock(&pool).await?;
                let prepared_deadline = record_now + 2;
                let (_flow, prepare) = ReconnectDurabilityFlowV1::begin(
                    record_with_prepared_deadline(
                        96,
                        1,
                        0xe6,
                        record_now,
                        prepared_deadline,
                    )
                    .map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                let session_id = prepare
                    .record()
                    .identity()
                    .game_session_id()
                    .as_bytes()
                    .to_vec();
                let attempt_ref = prepare
                    .record()
                    .identity()
                    .reconnect_attempt_ref()
                    .to_be_bytes()
                    .to_vec();
                let conflicting_ref = 2_u64.to_be_bytes().to_vec();
                sqlx::query(
                    "UPDATE game_durability_reconnect_sessions SET prepared_attempt_ref = $2 \
                     WHERE game_session_id = encode($1, 'hex')::uuid",
                )
                .bind(session_id.as_slice())
                .bind(conflicting_ref.as_slice())
                .execute(&pool)
                .await?;
                while postgres_clock(&pool).await? <= prepared_deadline {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                assert!(matches!(
                    journal.prepare(&prepare).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                let attempt_state: i16 = sqlx::query_scalar(
                    "SELECT state FROM game_durability_reconnect_attempts \
                     WHERE game_session_id = encode($1, 'hex')::uuid AND reconnect_attempt_ref = $2",
                )
                .bind(session_id.as_slice())
                .bind(attempt_ref.as_slice())
                .fetch_one(&pool)
                .await?;
                let retained_incumbent: Option<Vec<u8>> = sqlx::query_scalar(
                    "SELECT prepared_attempt_ref FROM game_durability_reconnect_sessions \
                     WHERE game_session_id = encode($1, 'hex')::uuid",
                )
                .bind(session_id.as_slice())
                .fetch_one(&pool)
                .await?;
                assert_eq!(attempt_state, 1, "failed expiry transition must roll back");
                assert_eq!(retained_incumbent.as_deref(), Some(conflicting_ref.as_slice()));
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn new_epoch_rejects_zero_fast_reconnect_generation_in_committed_winner()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("fast_proof_corrupt").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (mut flow, first_prepare) = ReconnectDurabilityFlowV1::begin(
                    record(97, 1, 0xe7, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&first_prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                    &first_prepare,
                    ReconnectPrepareDispositionV1::Prepared,
                ))
                .map_err(foundation_error)?;
                let current = current_authority_from_record(first_prepare.record(), record_now)
                    .map_err(foundation_error)?;
                let commit = flow
                    .authorize_commit(current, record_now)
                    .map_err(foundation_error)?;
                assert_eq!(
                    journal.commit(&commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );
                let pool = sqlx::PgPool::connect(&database_url).await?;
                let session_id = first_prepare
                    .record()
                    .identity()
                    .game_session_id()
                    .as_bytes()
                    .to_vec();
                let attempt_ref = first_prepare
                    .record()
                    .identity()
                    .reconnect_attempt_ref()
                    .to_be_bytes()
                    .to_vec();
                sqlx::query(
                    "UPDATE game_durability_reconnect_attempts \
                     SET record_json = jsonb_set(record_json::jsonb, '{proof}', \
                         '{\"class\":\"fast_reconnect\",\"generation\":0}'::jsonb)::text \
                     WHERE game_session_id = encode($1, 'hex')::uuid AND reconnect_attempt_ref = $2",
                )
                .bind(session_id.as_slice())
                .bind(attempt_ref.as_slice())
                .execute(&pool)
                .await?;
                let (_next_flow, next_prepare) = ReconnectDurabilityFlowV1::begin(
                    record_for_epoch(
                        97,
                        2,
                        0xe8,
                        record_now + 1,
                        record_now + 116,
                        4,
                        8,
                        9,
                        0x69,
                    )
                    .map_err(foundation_error)?,
                );
                assert!(matches!(
                    journal.prepare(&next_prepare).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                let session: (String, String, Option<Vec<u8>>, i16) = sqlx::query_as(
                    "SELECT control_loss_epoch::text, current_generation::text, current_transport_ref, session_state \
                     FROM game_durability_reconnect_sessions \
                     WHERE game_session_id = encode($1, 'hex')::uuid",
                )
                .bind(session_id.as_slice())
                .fetch_one(&pool)
                .await?;
                assert_eq!(session.0, "3");
                assert_eq!(session.1, "8");
                assert_eq!(session.2.as_deref(), Some([0xe7_u8; 16].as_slice()));
                assert_eq!(session.3, 2);
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn committed_prepare_replay_after_process_restart_routes_to_reconciliation()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("committed_prepare_replay").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (mut original_flow, original_prepare) = ReconnectDurabilityFlowV1::begin(
                    record(95, 1, 0xe5, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&original_prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                original_flow
                    .accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                        &original_prepare,
                        ReconnectPrepareDispositionV1::Prepared,
                    ))
                    .map_err(foundation_error)?;
                let current = current_authority_from_record(original_prepare.record(), record_now)
                    .map_err(foundation_error)?;
                let commit = original_flow
                    .authorize_commit(current, record_now)
                    .map_err(foundation_error)?;
                assert_eq!(
                    journal.commit(&commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );
                drop(journal);

                let recovered_journal =
                    AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let (mut recovered_flow, replay_prepare) =
                    ReconnectDurabilityFlowV1::begin(original_prepare.record().clone());
                let replay_disposition = recovered_journal.prepare(&replay_prepare).await?;
                assert_eq!(
                    replay_disposition,
                    ReconnectPrepareDispositionV1::Ambiguous,
                    "a durable COMMITTED winner must route a fresh process into reconciliation"
                );
                assert_eq!(
                    recovered_flow
                        .accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                            &replay_prepare,
                            replay_disposition,
                        ))
                        .map_err(foundation_error)?,
                    ReconnectPrepareActionV1::ReconcileSameAttempt
                );
                assert_eq!(
                    recovered_flow
                        .accept_reconciliation(
                            recovered_journal.reconcile(&replay_prepare).await?,
                            current_authority_from_record(replay_prepare.record(), record_now)
                                .map_err(foundation_error)?,
                        )
                        .map_err(foundation_error)?,
                    ReconnectProjectionDecisionV1::InstallController {
                        generation: ConnectionGeneration::new(8).map_err(|_error| {
                            std::io::Error::other("invalid connection generation")
                        })?,
                        transport_ref: AuthenticatedTransportRefV1::decode(&[0xe5; 16])
                            .map_err(|_error| std::io::Error::other("invalid transport ref"))?,
                    }
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn committed_replay_requires_the_exact_retained_transport_reservation()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database =
                postgres::IsolatedPostgres::create("committed_replay_transport_reservation")
                    .await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (mut flow, prepare) = ReconnectDurabilityFlowV1::begin(
                    record(96, 1, 0xe6, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                    &prepare,
                    ReconnectPrepareDispositionV1::Prepared,
                ))
                .map_err(foundation_error)?;
                let current = current_authority_from_record(prepare.record(), record_now)
                    .map_err(foundation_error)?;
                let commit = flow
                    .authorize_commit(current, record_now)
                    .map_err(foundation_error)?;
                assert_eq!(
                    journal.commit(&commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );

                let pool = sqlx::PgPool::connect(&database_url).await?;
                let mut corruption = postgres::begin_transport_corruption(&pool).await?;
                let corrupted = sqlx::query(
                    "UPDATE game_durability_transport_ref_reservations \
                     SET game_session_id = encode($2, 'hex')::uuid, reconnect_attempt_ref = $3 \
                     WHERE transport_ref = $1",
                )
                .bind(
                    prepare
                        .record()
                        .connection()
                        .transport_ref()
                        .to_bytes()
                        .as_slice(),
                )
                .bind(uuid_v7(0x9a))
                .bind([0xfe_u8; 8].as_slice())
                .execute(&mut *corruption)
                .await?;
                postgres::finish_transport_corruption(corruption).await?;
                assert_eq!(corrupted.rows_affected(), 1);
                assert!(matches!(
                    journal.commit(&commit).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                pool.close().await;
                drop(journal);

                let recovered_journal =
                    AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let (_recovered_flow, replay_prepare) =
                    ReconnectDurabilityFlowV1::begin(prepare.record().clone());
                assert!(matches!(
                    recovered_journal.prepare(&replay_prepare).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                assert!(matches!(
                    recovered_journal.reconcile(&replay_prepare).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn fresh_commit_holds_the_transport_reservation_lock_through_commit()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database =
                postgres::IsolatedPostgres::create("fresh_commit_reservation_lock").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (mut flow, prepare) = ReconnectDurabilityFlowV1::begin(
                    record(106, 1, 0xf4, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                    &prepare,
                    ReconnectPrepareDispositionV1::Prepared,
                ))
                .map_err(foundation_error)?;
                let current = current_authority_from_record(prepare.record(), record_now)
                    .map_err(foundation_error)?;
                let commit = flow
                    .authorize_commit(current, record_now)
                    .map_err(foundation_error)?;

                let pool = sqlx::PgPool::connect(&database_url).await?;
                let transport_ref = prepare
                    .record()
                    .connection()
                    .transport_ref()
                    .to_bytes()
                    .to_vec();
                let mut reservation_lock = pool.begin().await?;
                sqlx::query(
                    "SELECT transport_ref FROM game_durability_transport_ref_reservations \
                     WHERE transport_ref = $1 FOR UPDATE",
                )
                .bind(transport_ref.as_slice())
                .fetch_one(&mut *reservation_lock)
                .await?;

                let blocked_journal = journal.clone();
                let blocked_commit = commit.clone();
                let blocked =
                    tokio::spawn(async move { blocked_journal.commit(&blocked_commit).await });
                tokio::time::sleep(Duration::from_millis(100)).await;
                assert!(
                    !blocked.is_finished(),
                    "fresh COMMIT must wait while another transaction holds the reservation row"
                );
                let consumed_while_blocked: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM game_durability_recovery_grant_consumptions",
                )
                .fetch_one(&pool)
                .await?;
                assert_eq!(consumed_while_blocked, 0);

                reservation_lock.commit().await?;
                assert_eq!(blocked.await??, ReconnectCommitDispositionV1::Committed);
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn fresh_commit_requires_the_exact_retained_transport_reservation()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("fresh_commit_transport_reservation").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (mut flow, prepare) = ReconnectDurabilityFlowV1::begin(
                    record(99, 1, 0xe9, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                    &prepare,
                    ReconnectPrepareDispositionV1::Prepared,
                ))
                .map_err(foundation_error)?;
                let current = current_authority_from_record(prepare.record(), record_now)
                    .map_err(foundation_error)?;
                let commit = flow
                    .authorize_commit(current, record_now)
                    .map_err(foundation_error)?;

                let pool = sqlx::PgPool::connect(&database_url).await?;
                let mut corruption = postgres::begin_transport_corruption(&pool).await?;
                let deleted = sqlx::query(
                    "DELETE FROM game_durability_transport_ref_reservations WHERE transport_ref = $1",
                )
                .bind(
                    prepare
                        .record()
                        .connection()
                        .transport_ref()
                        .to_bytes()
                        .as_slice(),
                )
                .execute(&mut *corruption)
                .await?;
                postgres::finish_transport_corruption(corruption).await?;
                assert_eq!(deleted.rows_affected(), 1);

                assert!(matches!(
                    journal.commit(&commit).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                let session_id = prepare
                    .record()
                    .identity()
                    .game_session_id()
                    .as_bytes()
                    .to_vec();
                let attempt_ref = prepare
                    .record()
                    .identity()
                    .reconnect_attempt_ref()
                    .to_be_bytes()
                    .to_vec();
                let attempt_state: i16 = sqlx::query_scalar(
                    "SELECT state FROM game_durability_reconnect_attempts \
                     WHERE game_session_id = encode($1, 'hex')::uuid AND reconnect_attempt_ref = $2",
                )
                .bind(session_id.as_slice())
                .bind(attempt_ref.as_slice())
                .fetch_one(&pool)
                .await?;
                assert_eq!(attempt_state, 1, "invalid reservation must not commit the attempt");
                let session: (String, Option<Vec<u8>>, i16, Option<Vec<u8>>) = sqlx::query_as(
                    "SELECT current_generation::text, current_transport_ref, session_state, prepared_attempt_ref \
                     FROM game_durability_reconnect_sessions \
                     WHERE game_session_id = encode($1, 'hex')::uuid",
                )
                .bind(session_id.as_slice())
                .fetch_one(&pool)
                .await?;
                assert_eq!(session.0, "7");
                assert!(session.1.is_none());
                assert_eq!(session.2, 1);
                assert_eq!(session.3.as_deref(), Some(attempt_ref.as_slice()));
                let consumed: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM game_durability_recovery_grant_consumptions",
                )
                .fetch_one(&pool)
                .await?;
                assert_eq!(consumed, 0, "invalid reservation must not consume the recovery nonce");
                let character_id = prepare.record().identity().character_id().as_bytes().to_vec();
                let protection: (i16, Option<String>, bool, bool, i16) = sqlx::query_as(
                    "SELECT protection_entitlement_state, protection_fenced_generation::text, \
                            protection_activated_at IS NULL, protection_expires_at IS NULL, \
                            protection_rearm_state \
                     FROM game_durability_control_loss_continuity \
                     WHERE character_id = encode($1, 'hex')::uuid AND control_loss_epoch = 3",
                )
                .bind(character_id.as_slice())
                .fetch_one(&pool)
                .await?;
                assert_eq!(protection, (1, None, true, true, 1));
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn successful_commit_activates_unused_protection_exactly_once()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("protection_activate_once").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (mut flow, prepare) = ReconnectDurabilityFlowV1::begin(
                    record(104, 1, 0xf1, record_now).map_err(foundation_error)?,
                );
                assert_eq!(journal.prepare(&prepare).await?, ReconnectPrepareDispositionV1::Prepared);
                flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                    &prepare,
                    ReconnectPrepareDispositionV1::Prepared,
                ))
                .map_err(foundation_error)?;
                let current = current_authority_from_record(prepare.record(), record_now)
                    .map_err(foundation_error)?;
                let commit = flow.authorize_commit(current, record_now).map_err(foundation_error)?;
                assert_eq!(journal.commit(&commit).await?, ReconnectCommitDispositionV1::Committed);

                let pool = sqlx::PgPool::connect(&database_url).await?;
                let character_id = prepare.record().identity().character_id().as_bytes().to_vec();
                let first: ProtectionContinuityRow =
                    sqlx::query_as(
                        "SELECT protection_entitlement_state, protection_fenced_generation::text, \
                                protection_activated_at::text, protection_expires_at::text, \
                                EXTRACT(EPOCH FROM (protection_expires_at - protection_activated_at))::BIGINT, \
                                protection_rearm_state, protection_rearm_deadline::text \
                         FROM game_durability_control_loss_continuity \
                         WHERE character_id = encode($1, 'hex')::uuid \
                           AND control_loss_epoch = 3",
                    )
                    .bind(character_id.as_slice())
                    .fetch_one(&pool)
                    .await?;
                assert_eq!(first.0, 2);
                assert_eq!(first.1.as_deref(), Some("8"));
                assert!(first.2.is_some());
                assert!(first.3.is_some());
                assert_eq!(first.4, Some(4));
                assert_eq!(first.5, 2);
                assert!(first.6.is_none());

                drop(journal);
                let recovered_journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                assert_eq!(
                    recovered_journal.commit(&commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );
                let replay: ProtectionContinuityRow =
                    sqlx::query_as(
                        "SELECT protection_entitlement_state, protection_fenced_generation::text, \
                                protection_activated_at::text, protection_expires_at::text, \
                                EXTRACT(EPOCH FROM (protection_expires_at - protection_activated_at))::BIGINT, \
                                protection_rearm_state, protection_rearm_deadline::text \
                         FROM game_durability_control_loss_continuity \
                         WHERE character_id = encode($1, 'hex')::uuid \
                           AND control_loss_epoch = 3",
                    )
                    .bind(character_id.as_slice())
                    .fetch_one(&pool)
                    .await?;
                assert_eq!(replay, first, "lost-response replay must not restart the protection window");
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn protection_continuity_rejects_a_distinct_game_session_for_the_same_control_loss()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database =
                postgres::IsolatedPostgres::create("protection_continuity_session_binding").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;

                let (_first_flow, first_prepare) = ReconnectDurabilityFlowV1::begin(
                    record_for_actor(106, 11, 1, 0xf3, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&first_prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );

                let (_second_flow, second_prepare) = ReconnectDurabilityFlowV1::begin(
                    record_for_actor(107, 11, 1, 0xf4, record_now).map_err(foundation_error)?,
                );
                assert!(matches!(
                    journal.prepare(&second_prepare).await,
                    Err(DurabilityError::InvalidStoredState)
                ));

                let pool = sqlx::PgPool::connect(&database_url).await?;
                let character_id = first_prepare
                    .record()
                    .identity()
                    .character_id()
                    .as_bytes()
                    .to_vec();
                let context_game_session_id: Vec<u8> = sqlx::query_scalar(
                    "SELECT uuid_send(context_game_session_id) \
                     FROM game_durability_control_loss_continuity \
                     WHERE character_id = encode($1, 'hex')::uuid \
                       AND control_loss_epoch = 3",
                )
                .bind(character_id.as_slice())
                .fetch_one(&pool)
                .await?;
                assert_eq!(
                    context_game_session_id.as_slice(),
                    first_prepare
                        .record()
                        .identity()
                        .game_session_id()
                        .as_bytes()
                        .as_slice()
                );
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn reconnect_sessions_reject_a_distinct_game_session_for_a_later_control_loss_epoch()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database =
                postgres::IsolatedPostgres::create("reconnect_session_cross_epoch_binding").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let first_now = unix_now().map_err(foundation_error)?;

                let (mut first_flow, first_prepare) = ReconnectDurabilityFlowV1::begin(
                    record_for_actor(108, 11, 1, 0xf5, first_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&first_prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                first_flow
                    .accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                        &first_prepare,
                        ReconnectPrepareDispositionV1::Prepared,
                    ))
                    .map_err(foundation_error)?;
                let first_current =
                    current_authority_from_record(first_prepare.record(), first_now)
                        .map_err(foundation_error)?;
                let first_commit = first_flow
                    .authorize_commit(first_current, first_now)
                    .map_err(foundation_error)?;
                assert_eq!(
                    journal.commit(&first_commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );

                let second_now = unix_now().map_err(foundation_error)?;
                let second_record = record_for_actor_epoch_with_protection(
                    109,
                    11,
                    1,
                    0xf6,
                    second_now,
                    second_now + 115,
                    4,
                    8,
                    9,
                    0x74,
                    ProtectionEntitlementV1::fenced(8).map_err(foundation_error)?,
                )
                .map_err(foundation_error)?;
                let (_second_flow, second_prepare) =
                    ReconnectDurabilityFlowV1::begin(second_record);
                assert!(matches!(
                    journal.prepare(&second_prepare).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn fenced_entitlement_does_not_create_a_second_protection_window_on_later_epoch()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("protection_no_loop_extension").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let first_now = unix_now().map_err(foundation_error)?;
                let (mut first_flow, first_prepare) = ReconnectDurabilityFlowV1::begin(
                    record_for_epoch(105, 1, 0xf2, first_now, first_now + 115, 3, 7, 8, 0x71)
                        .map_err(foundation_error)?,
                );
                assert_eq!(journal.prepare(&first_prepare).await?, ReconnectPrepareDispositionV1::Prepared);
                first_flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                    &first_prepare,
                    ReconnectPrepareDispositionV1::Prepared,
                )).map_err(foundation_error)?;
                let first_current = current_authority_from_record(first_prepare.record(), first_now)
                    .map_err(foundation_error)?;
                let first_commit = first_flow.authorize_commit(first_current, first_now).map_err(foundation_error)?;
                assert_eq!(journal.commit(&first_commit).await?, ReconnectCommitDispositionV1::Committed);

                let second_now = unix_now().map_err(foundation_error)?;
                let fenced = ProtectionEntitlementV1::fenced(8).map_err(foundation_error)?;
                let (mut second_flow, second_prepare) = ReconnectDurabilityFlowV1::begin(
                    record_for_epoch_with_protection(
                        105,
                        2,
                        0xf3,
                        second_now,
                        second_now + 115,
                        4,
                        8,
                        9,
                        0x72,
                        fenced,
                    )
                    .map_err(foundation_error)?,
                );
                assert_eq!(journal.prepare(&second_prepare).await?, ReconnectPrepareDispositionV1::Prepared);
                second_flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                    &second_prepare,
                    ReconnectPrepareDispositionV1::Prepared,
                )).map_err(foundation_error)?;
                let second_current = current_authority_from_record(second_prepare.record(), second_now)
                    .map_err(foundation_error)?;
                let second_commit = second_flow.authorize_commit(second_current, second_now).map_err(foundation_error)?;
                assert_eq!(journal.commit(&second_commit).await?, ReconnectCommitDispositionV1::Committed);

                let pool = sqlx::PgPool::connect(&database_url).await?;
                let character_id = second_prepare.record().identity().character_id().as_bytes().to_vec();
                let rows: Vec<(String, i16, Option<String>, bool, bool, i16)> = sqlx::query_as(
                    "SELECT control_loss_epoch::text, protection_entitlement_state, \
                            protection_fenced_generation::text, \
                            protection_activated_at IS NOT NULL, protection_expires_at IS NOT NULL, \
                            protection_rearm_state \
                     FROM game_durability_control_loss_continuity \
                     WHERE character_id = encode($1, 'hex')::uuid \
                     ORDER BY control_loss_epoch",
                )
                .bind(character_id.as_slice())
                .fetch_all(&pool)
                .await?;
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].0, "3");
                assert_eq!(rows[0].1, 2);
                assert_eq!(rows[0].2.as_deref(), Some("8"));
                assert!(rows[0].3 && rows[0].4);
                assert_eq!(rows[0].5, 2);
                assert_eq!(rows[1], ("4".to_owned(), 2, Some("8".to_owned()), false, false, 2));
                let activations: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM game_durability_control_loss_continuity \
                     WHERE character_id = encode($1, 'hex')::uuid \
                       AND protection_activated_at IS NOT NULL",
                )
                .bind(character_id.as_slice())
                .fetch_one(&pool)
                .await?;
                assert_eq!(activations, 1, "non-rearmed reconnect churn must not mint a second protection window");
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn expired_prepared_replay_retires_incumbent_and_allows_fresh_attempt()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("expired_prepared_replay").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let pool = sqlx::PgPool::connect(&database_url).await?;
                let record_now = postgres_clock(&pool).await?;
                let prepared_deadline = record_now + 2;
                let (_flow, expired_prepare) = ReconnectDurabilityFlowV1::begin(
                    record_with_prepared_deadline(93, 1, 0xe1, record_now, prepared_deadline)
                        .map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&expired_prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                while postgres_clock(&pool).await? <= prepared_deadline {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                assert_eq!(
                    journal.prepare(&expired_prepare).await?,
                    ReconnectPrepareDispositionV1::ExistingTerminal,
                    "same-attempt replay after prepared expiry must retire the incumbent"
                );

                let fresh_deadline = postgres_clock(&pool).await? + 5;
                let (_fresh_flow, fresh_prepare) = ReconnectDurabilityFlowV1::begin(
                    record_with_prepared_deadline(93, 2, 0xe2, record_now, fresh_deadline)
                        .map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&fresh_prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared,
                    "prepared expiry must not consume the remaining original grace"
                );
                let session_id = fresh_prepare
                    .record()
                    .identity()
                    .game_session_id()
                    .as_bytes()
                    .to_vec();
                let prepared_ref: Option<Vec<u8>> = sqlx::query_scalar(
                    "SELECT prepared_attempt_ref FROM game_durability_reconnect_sessions \
                     WHERE game_session_id = encode($1, 'hex')::uuid",
                )
                .bind(session_id.as_slice())
                .fetch_one(&pool)
                .await?;
                assert_eq!(
                    prepared_ref.as_deref(),
                    Some(2_u64.to_be_bytes().as_slice())
                );
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn new_epoch_requires_complete_committed_fnd02_fence() -> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("new_epoch_complete_fnd02").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (mut flow, first_prepare) = ReconnectDurabilityFlowV1::begin(
                    record(97, 1, 0xe7, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&first_prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                    &first_prepare,
                    ReconnectPrepareDispositionV1::Prepared,
                ))
                .map_err(foundation_error)?;
                let current =
                    current_authority_from_record(first_prepare.record(), record_now)
                        .map_err(foundation_error)?;
                let commit = flow
                    .authorize_commit(current, record_now)
                    .map_err(foundation_error)?;
                assert_eq!(
                    journal.commit(&commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );

                let pool = sqlx::PgPool::connect(&database_url).await?;
                let session_id = first_prepare
                    .record()
                    .identity()
                    .game_session_id()
                    .as_bytes()
                    .to_vec();
                let attempt_ref = first_prepare
                    .record()
                    .identity()
                    .reconnect_attempt_ref()
                    .to_be_bytes();
                let changed_next = sqlx::query(
                    "UPDATE game_durability_reconnect_attempts \
                     SET fnd02_next_command_id = 4 \
                     WHERE game_session_id = encode($1, 'hex')::uuid \
                       AND reconnect_attempt_ref = $2",
                )
                .bind(session_id.as_slice())
                .bind(attempt_ref.as_slice())
                .execute(&pool)
                .await?;
                assert_eq!(changed_next.rows_affected(), 1);
                let (_next_flow, next_prepare) = ReconnectDurabilityFlowV1::begin(
                    record_for_epoch(97, 2, 0xe8, record_now + 1, record_now + 116, 4, 8, 9, 0x69)
                        .map_err(foundation_error)?,
                );
                assert!(matches!(
                    journal.prepare(&next_prepare).await,
                    Err(DurabilityError::InvalidStoredState)
                ));

                sqlx::query(
                    "UPDATE game_durability_reconnect_attempts \
                     SET fnd02_next_command_id = 3 \
                     WHERE game_session_id = encode($1, 'hex')::uuid \
                       AND reconnect_attempt_ref = $2",
                )
                .bind(session_id.as_slice())
                .bind(attempt_ref.as_slice())
                .execute(&pool)
                .await?;
                let changed_pending = sqlx::query(
                    "UPDATE game_durability_reconnect_pending_commands \
                     SET disposition = 2 \
                     WHERE game_session_id = encode($1, 'hex')::uuid \
                       AND reconnect_attempt_ref = $2 AND command_id = 1",
                )
                .bind(session_id.as_slice())
                .bind(attempt_ref.as_slice())
                .execute(&pool)
                .await?;
                assert_eq!(changed_pending.rows_affected(), 1);
                let (_pending_flow, pending_prepare) = ReconnectDurabilityFlowV1::begin(
                    record_for_epoch(97, 3, 0xe9, record_now + 1, record_now + 116, 4, 8, 9, 0x6a)
                        .map_err(foundation_error)?,
                );
                assert!(matches!(
                    journal.prepare(&pending_prepare).await,
                    Err(DurabilityError::InvalidStoredState)
                ));

                sqlx::query(
                    "UPDATE game_durability_reconnect_pending_commands \
                     SET disposition = 1 \
                     WHERE game_session_id = encode($1, 'hex')::uuid \
                       AND reconnect_attempt_ref = $2 AND command_id = 1",
                )
                .bind(session_id.as_slice())
                .bind(attempt_ref.as_slice())
                .execute(&pool)
                .await?;
                let removed_server_sequence = sqlx::query(
                    "UPDATE game_durability_reconnect_attempts \
                     SET record_json = (record_json::jsonb #- '{fnd02,server_sequence}')::text \
                     WHERE game_session_id = encode($1, 'hex')::uuid \
                       AND reconnect_attempt_ref = $2",
                )
                .bind(session_id.as_slice())
                .bind(attempt_ref.as_slice())
                .execute(&pool)
                .await?;
                assert_eq!(removed_server_sequence.rows_affected(), 1);
                let (_missing_server_flow, missing_server_prepare) =
                    ReconnectDurabilityFlowV1::begin(
                        record_for_epoch(
                            97,
                            4,
                            0xea,
                            record_now + 1,
                            record_now + 116,
                            4,
                            8,
                            9,
                            0x6b,
                        )
                        .map_err(foundation_error)?,
                    );
                assert!(matches!(
                    journal.prepare(&missing_server_prepare).await,
                    Err(DurabilityError::InvalidStoredState)
                ));

                sqlx::query(
                    "UPDATE game_durability_reconnect_attempts \
                     SET record_json = jsonb_set( \
                         record_json::jsonb, '{fnd02,server_sequence}', '41'::jsonb \
                     )::text \
                     WHERE game_session_id = encode($1, 'hex')::uuid \
                       AND reconnect_attempt_ref = $2",
                )
                .bind(session_id.as_slice())
                .bind(attempt_ref.as_slice())
                .execute(&pool)
                .await?;
                let unordered_domains = sqlx::query(
                    "UPDATE game_durability_reconnect_attempts \
                     SET record_json = jsonb_set( \
                         record_json::jsonb, '{fnd02,domain_revisions}', $3::jsonb \
                     )::text \
                     WHERE game_session_id = encode($1, 'hex')::uuid \
                       AND reconnect_attempt_ref = $2",
                )
                .bind(session_id.as_slice())
                .bind(attempt_ref.as_slice())
                .bind(r#"[{"domain_id":2,"revision":7},{"domain_id":1,"revision":4}]"#)
                .execute(&pool)
                .await?;
                assert_eq!(unordered_domains.rows_affected(), 1);
                let (_unordered_domains_flow, unordered_domains_prepare) =
                    ReconnectDurabilityFlowV1::begin(
                        record_for_epoch(
                            97,
                            5,
                            0xeb,
                            record_now + 1,
                            record_now + 116,
                            4,
                            8,
                            9,
                            0x6c,
                        )
                        .map_err(foundation_error)?,
                    );
                assert!(matches!(
                    journal.prepare(&unordered_domains_prepare).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                let session: (String, String, i16, i16) = sqlx::query_as(
                    "SELECT control_loss_epoch::text, current_generation::text, session_state, attempt_count \
                     FROM game_durability_reconnect_sessions \
                     WHERE game_session_id = encode($1, 'hex')::uuid",
                )
                .bind(session_id.as_slice())
                .fetch_one(&pool)
                .await?;
                assert_eq!(session, ("3".to_owned(), "8".to_owned(), 2, 1));
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn new_epoch_rejects_committed_winner_without_compatibility_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database =
                postgres::IsolatedPostgres::create("new_epoch_compatibility_evidence").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (mut flow, first_prepare) = ReconnectDurabilityFlowV1::begin(
                    record(98, 1, 0xe8, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&first_prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                    &first_prepare,
                    ReconnectPrepareDispositionV1::Prepared,
                ))
                .map_err(foundation_error)?;
                let current = current_authority_from_record(first_prepare.record(), record_now)
                    .map_err(foundation_error)?;
                let commit = flow
                    .authorize_commit(current, record_now)
                    .map_err(foundation_error)?;
                assert_eq!(
                    journal.commit(&commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );

                let pool = sqlx::PgPool::connect(&database_url).await?;
                let session_id = first_prepare
                    .record()
                    .identity()
                    .game_session_id()
                    .as_bytes()
                    .to_vec();
                let attempt_ref = first_prepare
                    .record()
                    .identity()
                    .reconnect_attempt_ref()
                    .to_be_bytes();
                let removed = sqlx::query(
                    "UPDATE game_durability_reconnect_attempts \
                     SET record_json = (record_json::jsonb - 'compatibility')::text \
                     WHERE game_session_id = encode($1, 'hex')::uuid \
                       AND reconnect_attempt_ref = $2",
                )
                .bind(session_id.as_slice())
                .bind(attempt_ref.as_slice())
                .execute(&pool)
                .await?;
                assert_eq!(removed.rows_affected(), 1);

                let (_next_flow, next_prepare) = ReconnectDurabilityFlowV1::begin(
                    record_for_epoch(98, 2, 0xe9, record_now + 1, record_now + 116, 4, 8, 9, 0x6d)
                        .map_err(foundation_error)?,
                );
                assert!(matches!(
                    journal.prepare(&next_prepare).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn new_epoch_requires_a_valid_committed_active_transport_binding()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("new_epoch_active_binding").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (mut flow, first_prepare) = ReconnectDurabilityFlowV1::begin(
                    record(94, 1, 0xe3, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&first_prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                    &first_prepare,
                    ReconnectPrepareDispositionV1::Prepared,
                ))
                .map_err(foundation_error)?;
                let current = current_authority_from_record(first_prepare.record(), record_now)
                    .map_err(foundation_error)?;
                let commit = flow
                    .authorize_commit(current, record_now)
                    .map_err(foundation_error)?;
                assert_eq!(
                    journal.commit(&commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );

                let pool = sqlx::PgPool::connect(&database_url).await?;
                let session_id = first_prepare
                    .record()
                    .identity()
                    .game_session_id()
                    .as_bytes()
                    .to_vec();
                sqlx::query(
                    "UPDATE game_durability_reconnect_sessions SET current_transport_ref = $2 \
                     WHERE game_session_id = encode($1, 'hex')::uuid",
                )
                .bind(session_id.as_slice())
                .bind([0xee_u8; 16].as_slice())
                .execute(&pool)
                .await?;

                let (_second_flow, second_prepare) = ReconnectDurabilityFlowV1::begin(
                    record_for_epoch(
                        94,
                        2,
                        0xe4,
                        record_now + 1,
                        record_now + 116,
                        4,
                        8,
                        9,
                        0x68,
                    )
                    .map_err(foundation_error)?,
                );
                assert!(matches!(
                    journal.prepare(&second_prepare).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                let session: (String, String, Option<Vec<u8>>, i16, i16) = sqlx::query_as(
                    "SELECT control_loss_epoch::text, current_generation::text, current_transport_ref, \
                            session_state, attempt_count \
                     FROM game_durability_reconnect_sessions \
                     WHERE game_session_id = encode($1, 'hex')::uuid",
                )
                .bind(session_id.as_slice())
                .fetch_one(&pool)
                .await?;
                assert_eq!(session.0, "3");
                assert_eq!(session.1, "8");
                assert_eq!(session.2.as_deref(), Some([0xee_u8; 16].as_slice()));
                assert_eq!(session.3, 2);
                assert_eq!(session.4, 1);
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn committed_session_accepts_a_later_non_reused_control_loss_epoch()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("cross_epoch_reconnect").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let first_now = unix_now().map_err(foundation_error)?;
                let (mut first_flow, first_prepare) = ReconnectDurabilityFlowV1::begin(
                    record(63, 1, 0xd1, first_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&first_prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                first_flow
                    .accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                        &first_prepare,
                        ReconnectPrepareDispositionV1::Prepared,
                    ))
                    .map_err(foundation_error)?;
                let first_current =
                    current_authority_from_record(first_prepare.record(), first_now)
                        .map_err(foundation_error)?;
                let first_commit = first_flow
                    .authorize_commit(first_current, first_now)
                    .map_err(foundation_error)?;
                assert_eq!(
                    journal.commit(&first_commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );

                let second_now = first_now + 1;
                let (mut second_flow, second_prepare) = ReconnectDurabilityFlowV1::begin(
                    record_for_epoch(
                        63,
                        2,
                        0xd2,
                        second_now,
                        second_now + 115,
                        4,
                        8,
                        9,
                        0x56,
                    )
                    .map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&second_prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared,
                    "a later authoritative loss epoch must replace the committed transport fence"
                );

                let (_changed_grace_flow, changed_grace_prepare) =
                    ReconnectDurabilityFlowV1::begin(
                        record_for_epoch(
                            63,
                            3,
                            0xd3,
                            second_now + 1,
                            second_now + 116,
                            4,
                            8,
                            9,
                            0x57,
                        )
                        .map_err(foundation_error)?,
                    );
                assert_eq!(
                    journal.prepare(&changed_grace_prepare).await?,
                    ReconnectPrepareDispositionV1::RejectedStaleAuthority,
                    "attempts in one loss epoch cannot restart or extend its original grace"
                );
                assert_eq!(
                    journal.prepare(&changed_grace_prepare).await?,
                    ReconnectPrepareDispositionV1::ExistingTerminal
                );

                second_flow
                    .accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                        &second_prepare,
                        ReconnectPrepareDispositionV1::Prepared,
                    ))
                    .map_err(foundation_error)?;
                let second_current =
                    current_authority_from_record(second_prepare.record(), second_now)
                        .map_err(foundation_error)?;
                let second_commit = second_flow
                    .authorize_commit(second_current, second_now)
                    .map_err(foundation_error)?;
                assert_eq!(
                    journal.commit(&second_commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );
                assert_eq!(
                    journal.commit(&second_commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );
                assert!(matches!(
                    journal.commit(&first_commit).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                assert_eq!(
                    journal.reconcile(&first_prepare).await?,
                    ReconnectDurableReconciliationSnapshotV1::committed(
                        first_prepare.record().clone()
                    ),
                    "a later committed projection must retain historical committed evidence"
                );

                let (_reused_epoch_flow, reused_epoch_prepare) =
                    ReconnectDurabilityFlowV1::begin(
                        record_for_epoch(
                            63,
                            4,
                            0xd4,
                            second_now,
                            second_now + 115,
                            3,
                            9,
                            10,
                            0x58,
                        )
                        .map_err(foundation_error)?,
                    );
                assert_eq!(
                    journal.prepare(&reused_epoch_prepare).await?,
                    ReconnectPrepareDispositionV1::RejectedStaleAuthority,
                    "a previously retained loss epoch cannot be reused"
                );
                assert_eq!(
                    journal.prepare(&reused_epoch_prepare).await?,
                    ReconnectPrepareDispositionV1::ExistingTerminal
                );

                for attempt in 5_u64..=10 {
                    let transport = u8::try_from(0xd0_u64 + attempt)?;
                    let nonce = u8::try_from(0x50_u64 + attempt)?;
                    let (_old_epoch_flow, old_epoch_prepare) = ReconnectDurabilityFlowV1::begin(
                        record_for_epoch(
                            63,
                            attempt,
                            transport,
                            second_now,
                            second_now + 115,
                            3,
                            9,
                            10,
                            nonce,
                        )
                        .map_err(foundation_error)?,
                    );
                    assert_eq!(
                        journal.prepare(&old_epoch_prepare).await?,
                        ReconnectPrepareDispositionV1::RejectedStaleAuthority
                    );
                }
                let (_old_epoch_capacity_flow, old_epoch_capacity) =
                    ReconnectDurabilityFlowV1::begin(
                        record_for_epoch(
                            63,
                            11,
                            0xdb,
                            second_now,
                            second_now + 115,
                            3,
                            9,
                            10,
                            0x5b,
                        )
                        .map_err(foundation_error)?,
                    );
                assert_eq!(
                    journal.prepare(&old_epoch_capacity).await?,
                    ReconnectPrepareDispositionV1::AttemptCapacityExceeded,
                    "closed epochs retain the same eight-attempt hard bound"
                );

                let pool = sqlx::PgPool::connect(&database_url).await?;
                let session_id = second_prepare
                    .record()
                    .identity()
                    .game_session_id()
                    .as_bytes()
                    .to_vec();
                let session: CrossEpochSessionRow = sqlx::query_as(
                    "SELECT control_loss_epoch::BIGINT, original_grace_deadline, predecessor_generation::BIGINT, \
                            current_generation::BIGINT, current_transport_ref, session_state, \
                            attempt_count, prepared_attempt_ref \
                     FROM game_durability_reconnect_sessions \
                     WHERE game_session_id = encode($1, 'hex')::uuid",
                )
                .bind(session_id.as_slice())
                .fetch_one(&pool)
                .await?;
                assert_eq!(session.0, 4);
                assert_eq!(session.1, second_now + 120);
                assert_eq!(session.2, 8);
                assert_eq!(session.3, 9);
                assert_eq!(session.4.as_deref(), Some([0xd2_u8; 16].as_slice()));
                assert_eq!(session.5, 2);
                assert_eq!(session.6, 2);
                assert!(session.7.is_none());

                let attempts_per_epoch: Vec<(i64, i64)> = sqlx::query_as(
                    "SELECT control_loss_epoch::BIGINT, COUNT(*) \
                     FROM game_durability_reconnect_attempts \
                     WHERE game_session_id = encode($1, 'hex')::uuid \
                     GROUP BY control_loss_epoch ORDER BY control_loss_epoch",
                )
                .bind(session_id.as_slice())
                .fetch_all(&pool)
                .await?;
                assert_eq!(attempts_per_epoch, vec![(3, 8), (4, 2)]);

                let grace_by_epoch: Vec<(i64, i64)> = sqlx::query_as(
                    "SELECT control_loss_epoch::BIGINT, \
                            (record_json::jsonb #>> '{continuity,original_grace_deadline}')::BIGINT \
                     FROM game_durability_reconnect_attempts \
                     WHERE game_session_id = encode($1, 'hex')::uuid \
                       AND reconnect_attempt_ref IN ($2, $3) \
                     ORDER BY control_loss_epoch",
                )
                .bind(session_id.as_slice())
                .bind(1_u64.to_be_bytes().as_slice())
                .bind(2_u64.to_be_bytes().as_slice())
                .fetch_all(&pool)
                .await?;
                assert_eq!(
                    grace_by_epoch,
                    vec![(3, first_now + 120), (4, second_now + 120)]
                );
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn stale_commit_terminalizes_the_prepared_attempt_for_reconciliation()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("stale_commit").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (mut flow, prepare) = ReconnectDurabilityFlowV1::begin(
                    record(61, 1, 0x72, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                assert_eq!(
                    flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                        &prepare,
                        ReconnectPrepareDispositionV1::Prepared,
                    ))
                    .map_err(foundation_error)?,
                    ReconnectPrepareActionV1::AwaitFinalRevalidation
                );
                let current =
                    current_authority_from_record(prepare.record(), record_now)
                        .map_err(foundation_error)?;
                let commit = flow
                    .authorize_commit(current, record_now)
                    .map_err(foundation_error)?;
                let pool = sqlx::PgPool::connect(&database_url).await?;
                sqlx::query(
                    "UPDATE game_durability_reconnect_sessions \
                     SET session_state = 2, current_generation = 8, current_transport_ref = $2 \
                     WHERE game_session_id = encode($1, 'hex')::uuid",
                )
                .bind(
                    prepare
                        .record()
                        .identity()
                        .game_session_id()
                        .as_bytes()
                        .as_slice(),
                )
                .bind([0x99_u8; 16].as_slice())
                .execute(&pool)
                .await?;
                assert_eq!(
                    journal.commit(&commit).await?,
                    ReconnectCommitDispositionV1::RejectedStaleAuthority
                );
                assert_eq!(
                    flow.accept_commit_completion(ReconnectCommitCompletionV1::for_request(
                        &commit,
                        ReconnectCommitDispositionV1::RejectedStaleAuthority,
                    ))
                    .map_err(foundation_error)?,
                    ReconnectCommitActionV1::Terminal(
                        ReconnectCommitDispositionV1::RejectedStaleAuthority
                    )
                );
                assert_eq!(
                    journal.reconcile(&prepare).await?,
                    oteryn_game_server::foundation::ReconnectDurableReconciliationSnapshotV1::terminal(
                        prepare.record().clone(),
                    )
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn expired_prepared_reconciliation_terminalizes_incumbent_and_allows_later_attempt()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database =
                postgres::IsolatedPostgres::create("expired_prepared_reconciliation").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let pool = sqlx::PgPool::connect(&database_url).await?;
                let record_now = postgres_clock(&pool).await?;
                let prepared_deadline = record_now + 2;
                let (_flow, expired_prepare) = ReconnectDurabilityFlowV1::begin(
                    record_with_prepared_deadline(94, 1, 0xe3, record_now, prepared_deadline)
                        .map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&expired_prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                while postgres_clock(&pool).await? <= prepared_deadline {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }

                assert_eq!(
                    journal.reconcile(&expired_prepare).await?,
                    oteryn_game_server::foundation::ReconnectDurableReconciliationSnapshotV1::terminal(
                        expired_prepare.record().clone(),
                    ),
                    "reconciliation must terminalize the exact expired PREPARED attempt"
                );
                let session_id = expired_prepare
                    .record()
                    .identity()
                    .game_session_id()
                    .as_bytes()
                    .to_vec();
                let attempt_ref = expired_prepare
                    .record()
                    .identity()
                    .reconnect_attempt_ref()
                    .to_be_bytes();
                let state: i16 = sqlx::query_scalar(
                    "SELECT state FROM game_durability_reconnect_attempts \
                     WHERE game_session_id = encode($1, 'hex')::uuid \
                       AND reconnect_attempt_ref = $2",
                )
                .bind(session_id.as_slice())
                .bind(attempt_ref.as_slice())
                .fetch_one(&pool)
                .await?;
                let prepared_ref: Option<Vec<u8>> = sqlx::query_scalar(
                    "SELECT prepared_attempt_ref FROM game_durability_reconnect_sessions \
                     WHERE game_session_id = encode($1, 'hex')::uuid",
                )
                .bind(session_id.as_slice())
                .fetch_one(&pool)
                .await?;
                assert_eq!(state, 4, "expired PREPARED must be durably terminal");
                assert!(prepared_ref.is_none(), "expired anchor must be cleared");

                let fresh_deadline = postgres_clock(&pool).await? + 5;
                let (_fresh_flow, fresh_prepare) = ReconnectDurabilityFlowV1::begin(
                    record_with_prepared_deadline(94, 2, 0xe4, record_now, fresh_deadline)
                        .map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&fresh_prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared,
                    "the expired incumbent must not falsely block a later distinct attempt"
                );
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn committed_reconciliation_remains_historical_after_later_epoch_opens()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database =
                postgres::IsolatedPostgres::create("historical_committed_replay").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let first_now = unix_now().map_err(foundation_error)?;
                let (mut first_flow, first_prepare) = ReconnectDurabilityFlowV1::begin(
                    record(161, 1, 0xa1, first_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&first_prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                first_flow
                    .accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                        &first_prepare,
                        ReconnectPrepareDispositionV1::Prepared,
                    ))
                    .map_err(foundation_error)?;
                let first_commit = first_flow
                    .authorize_commit(
                        current_authority_from_record(first_prepare.record(), first_now)
                            .map_err(foundation_error)?,
                        first_now,
                    )
                    .map_err(foundation_error)?;
                assert_eq!(
                    journal.commit(&first_commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );

                let second_now = first_now + 1;
                let (_second_flow, second_prepare) = ReconnectDurabilityFlowV1::begin(
                    record_for_epoch(161, 2, 0xa2, second_now, second_now + 115, 4, 8, 9, 0xa3)
                        .map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&second_prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );

                assert_eq!(
                    journal.reconcile(&first_prepare).await?,
                    ReconnectDurableReconciliationSnapshotV1::committed(
                        first_prepare.record().clone()
                    ),
                    "a later loss epoch must not erase historical committed evidence"
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn historical_committed_reconciliation_rejects_corrupt_later_prepared_projection()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let corruptions = [
                "session_generation",
                "canonical_attempt",
                "candidate_generation",
                "proof_zero_nonce",
                "transport_reservation",
                "protection_continuity",
                "fnd02_mirror",
            ];
            for (index, corruption) in corruptions.into_iter().enumerate() {
                let database =
                    postgres::IsolatedPostgres::create(&format!("hist_prep_{index}")).await?;
                let result = async {
                    let database_url = database.database_url()?;
                    let executor = MigrationExecutor::connect_migration(&database_url).await?;
                    executor.apply_embedded_ledger().await?;
                    let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                    let record_now = unix_now().map_err(foundation_error)?;
                    let seed = 170 + index as u64;
                    let (mut first_flow, first_prepare) = ReconnectDurabilityFlowV1::begin(
                        record(seed, 1, 0xb1 + index as u8, record_now)
                            .map_err(foundation_error)?,
                    );
                    assert_eq!(
                        journal.prepare(&first_prepare).await?,
                        ReconnectPrepareDispositionV1::Prepared
                    );
                    first_flow
                        .accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                            &first_prepare,
                            ReconnectPrepareDispositionV1::Prepared,
                        ))
                        .map_err(foundation_error)?;
                    let first_commit = first_flow
                        .authorize_commit(
                            current_authority_from_record(first_prepare.record(), record_now)
                                .map_err(foundation_error)?,
                            record_now,
                        )
                        .map_err(foundation_error)?;
                    assert_eq!(
                        journal.commit(&first_commit).await?,
                        ReconnectCommitDispositionV1::Committed
                    );

                    let second_now = record_now + 1;
                    let (_second_flow, second_prepare) = ReconnectDurabilityFlowV1::begin(
                        record_for_epoch(
                            seed,
                            2,
                            0xc1 + index as u8,
                            second_now,
                            second_now + 115,
                            4,
                            8,
                            9,
                            0xd1 + index as u8,
                        )
                        .map_err(foundation_error)?,
                    );
                    assert_eq!(
                        journal.prepare(&second_prepare).await?,
                        ReconnectPrepareDispositionV1::Prepared
                    );

                    let pool = sqlx::PgPool::connect(&database_url).await?;
                    let session_id = second_prepare
                        .record()
                        .identity()
                        .game_session_id()
                        .as_bytes()
                        .to_vec();
                    let attempt_ref = second_prepare
                        .record()
                        .identity()
                        .reconnect_attempt_ref()
                        .to_be_bytes();
                    let transport_ref = second_prepare
                        .record()
                        .connection()
                        .transport_ref()
                        .to_bytes();
                    let character_id = second_prepare
                        .record()
                        .identity()
                        .character_id()
                        .as_bytes()
                        .to_vec();
                    let epoch = second_prepare
                        .record()
                        .continuity()
                        .control_loss_epoch()
                        .get()
                        .to_string();
                    match corruption {
                        "session_generation" => {
                            sqlx::query(
                                "UPDATE game_durability_reconnect_sessions \
                                 SET current_generation = current_generation + 1 \
                                 WHERE game_session_id = encode($1, 'hex')::uuid",
                            )
                            .bind(session_id.as_slice())
                            .execute(&pool)
                            .await?;
                        }
                        "canonical_attempt" => {
                            sqlx::query(
                                "UPDATE game_durability_reconnect_attempts \
                                 SET record_json = jsonb_set(record_json::jsonb, \
                                     '{connection,transport_ref}', '[1]'::jsonb)::text \
                                 WHERE game_session_id = encode($1, 'hex')::uuid \
                                   AND reconnect_attempt_ref = $2",
                            )
                            .bind(session_id.as_slice())
                            .bind(attempt_ref.as_slice())
                            .execute(&pool)
                            .await?;
                        }
                        "candidate_generation" => {
                            sqlx::query(
                                "UPDATE game_durability_reconnect_attempts \
                                 SET record_json = jsonb_set(record_json::jsonb, \
                                     '{connection,candidate_generation}', '10'::jsonb)::text \
                                 WHERE game_session_id = encode($1, 'hex')::uuid \
                                   AND reconnect_attempt_ref = $2",
                            )
                            .bind(session_id.as_slice())
                            .bind(attempt_ref.as_slice())
                            .execute(&pool)
                            .await?;
                        }
                        "proof_zero_nonce" => {
                            sqlx::query(
                                "UPDATE game_durability_reconnect_attempts \
                                 SET record_json = jsonb_set(record_json::jsonb, \
                                     '{proof,recovery_grant_nonce}', \
                                     '[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,\
                                       0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]'::jsonb)::text \
                                 WHERE game_session_id = encode($1, 'hex')::uuid \
                                   AND reconnect_attempt_ref = $2",
                            )
                            .bind(session_id.as_slice())
                            .bind(attempt_ref.as_slice())
                            .execute(&pool)
                            .await?;
                        }
                        "transport_reservation" => {
                            let mut corruption =
                                postgres::begin_transport_corruption(&pool).await?;
                            sqlx::query(
                                "DELETE FROM game_durability_transport_ref_reservations \
                                 WHERE transport_ref = $1",
                            )
                            .bind(transport_ref.as_slice())
                            .execute(&mut *corruption)
                            .await?;
                            postgres::finish_transport_corruption(corruption).await?;
                        }
                        "protection_continuity" => {
                            sqlx::query(
                                "DELETE FROM game_durability_control_loss_continuity \
                                 WHERE character_id = encode($1, 'hex')::uuid \
                                   AND control_loss_epoch = $2::text::numeric(20, 0)",
                            )
                            .bind(character_id.as_slice())
                            .bind(&epoch)
                            .execute(&pool)
                            .await?;
                        }
                        "fnd02_mirror" => {
                            sqlx::query(
                                "UPDATE game_durability_reconnect_attempts \
                                 SET fnd02_next_command_id = fnd02_next_command_id + 1 \
                                 WHERE game_session_id = encode($1, 'hex')::uuid \
                                   AND reconnect_attempt_ref = $2",
                            )
                            .bind(session_id.as_slice())
                            .bind(attempt_ref.as_slice())
                            .execute(&pool)
                            .await?;
                        }
                        _ => unreachable!(),
                    }

                    assert!(
                        matches!(
                            journal.reconcile(&first_prepare).await,
                            Err(DurabilityError::InvalidStoredState)
                        ),
                        "historical reconciliation must reject corrupt later PREPARED {corruption}"
                    );
                    pool.close().await;
                    Ok::<(), Box<dyn std::error::Error>>(())
                }
                .await;
                database.cleanup().await?;
                result?;
            }
            Ok(())
        })
}

#[test]
fn committed_replay_fails_closed_when_session_state_is_inconsistent()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("corrupt_commit_state").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (mut flow, prepare) = ReconnectDurabilityFlowV1::begin(
                    record(62, 1, 0x73, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                    &prepare,
                    ReconnectPrepareDispositionV1::Prepared,
                ))
                .map_err(foundation_error)?;
                let current = current_authority_from_record(prepare.record(), record_now)
                    .map_err(foundation_error)?;
                let commit = flow
                    .authorize_commit(current, record_now)
                    .map_err(foundation_error)?;
                assert_eq!(
                    journal.commit(&commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );
                let pool = sqlx::PgPool::connect(&database_url).await?;
                sqlx::query(
                    "UPDATE game_durability_reconnect_sessions SET session_state = 1 \
                     WHERE game_session_id = encode($1, 'hex')::uuid",
                )
                .bind(
                    prepare
                        .record()
                        .identity()
                        .game_session_id()
                        .as_bytes()
                        .as_slice(),
                )
                .execute(&pool)
                .await?;
                assert!(matches!(
                    journal.commit(&commit).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn commit_row_lock_wait_cannot_outlive_authorization_deadline()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("commit_deadline_lock").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let lock_pool = sqlx::PgPool::connect(&database_url).await?;
                let record_now = postgres_clock(&lock_pool).await?;
                let (mut flow, prepare) = ReconnectDurabilityFlowV1::begin(
                    record(90, 1, 0xb1, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                    &prepare,
                    ReconnectPrepareDispositionV1::Prepared,
                ))
                .map_err(foundation_error)?;
                let current = current_authority_from_record(prepare.record(), record_now)
                    .map_err(foundation_error)?;
                let commit = flow
                    .authorize_commit(current, record_now)
                    .map_err(foundation_error)?;
                let deadline = commit.authorization().authorization_deadline();
                assert!(postgres_clock(&lock_pool).await? <= deadline);

                let session_id = prepare
                    .record()
                    .identity()
                    .game_session_id()
                    .as_bytes()
                    .to_vec();
                let mut lock = lock_pool.begin().await?;
                sqlx::query(
                    "SELECT game_session_id FROM game_durability_reconnect_sessions \
                     WHERE game_session_id = encode($1, 'hex')::uuid FOR UPDATE",
                )
                .bind(session_id.as_slice())
                .fetch_one(&mut *lock)
                .await?;

                let blocked_journal = journal.clone();
                let blocked_commit = commit.clone();
                let blocked = tokio::spawn(async move {
                    blocked_journal.commit(&blocked_commit).await
                });
                tokio::time::sleep(Duration::from_millis(100)).await;
                assert!(
                    !blocked.is_finished(),
                    "commit must be waiting on the held per-session row lock"
                );
                while postgres_clock(&lock_pool).await? <= deadline {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                lock.commit().await?;

                assert_eq!(
                    blocked.await??,
                    ReconnectCommitDispositionV1::RejectedStaleAuthority
                );
                let session: (i64, Option<Vec<u8>>, i16, Option<Vec<u8>>) = sqlx::query_as(
                    "SELECT current_generation::BIGINT, current_transport_ref, session_state, prepared_attempt_ref \
                     FROM game_durability_reconnect_sessions \
                     WHERE game_session_id = encode($1, 'hex')::uuid",
                )
                .bind(session_id.as_slice())
                .fetch_one(&lock_pool)
                .await?;
                assert_eq!(session.0, 7);
                assert!(session.1.is_none());
                assert_eq!(session.2, 1);
                assert!(session.3.is_none());
                let consumed: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM game_durability_recovery_grant_consumptions",
                )
                .fetch_one(&lock_pool)
                .await?;
                assert_eq!(consumed, 0, "expired commit must not consume recovery nonce");
                assert_eq!(
                    journal.reconcile(&prepare).await?,
                    oteryn_game_server::foundation::ReconnectDurableReconciliationSnapshotV1::terminal(
                        prepare.record().clone(),
                    )
                );
                lock_pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn commit_nonce_relation_wait_cannot_outlive_authorization_deadline()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("commit_nonce_relation_deadline").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let lock_pool = sqlx::PgPool::connect(&database_url).await?;
                let record_now = postgres_clock(&lock_pool).await?;
                let (mut flow, prepare) = ReconnectDurabilityFlowV1::begin(
                    record(90, 1, 0xb1, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                    &prepare,
                    ReconnectPrepareDispositionV1::Prepared,
                ))
                .map_err(foundation_error)?;
                let current = current_authority_from_record(prepare.record(), record_now)
                    .map_err(foundation_error)?;
                let commit = flow
                    .authorize_commit(current, record_now)
                    .map_err(foundation_error)?;
                let deadline = commit.authorization().authorization_deadline();
                assert!(postgres_clock(&lock_pool).await? <= deadline);

                let session_id = prepare
                    .record()
                    .identity()
                    .game_session_id()
                    .as_bytes()
                    .to_vec();
                let mut lock = lock_pool.begin().await?;
                sqlx::query("LOCK TABLE game_durability_recovery_grant_consumptions IN SHARE MODE")
                    .execute(&mut *lock).await?;

                let blocked_journal = journal.clone();
                let blocked_commit = commit.clone();
                let blocked = tokio::spawn(async move {
                    blocked_journal.commit(&blocked_commit).await
                });
                tokio::time::timeout(Duration::from_secs(5), async {
                    loop {
                        let waiting: bool = sqlx::query_scalar(
                            "SELECT EXISTS (SELECT 1 FROM pg_locks WHERE \
                             relation = 'game_durability_recovery_grant_consumptions'::regclass \
                             AND NOT granted AND pid <> pg_backend_pid())")
                            .fetch_one(&lock_pool).await?;
                        if waiting { break Ok::<(), sqlx::Error>(()); }
                        tokio::task::yield_now().await;
                    }
                }).await??;
                assert!(!blocked.is_finished(), "commit must wait on the observed nonce relation lock");
                while postgres_clock(&lock_pool).await? <= deadline {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                lock.commit().await?;

                assert_eq!(
                    blocked.await??,
                    ReconnectCommitDispositionV1::RejectedStaleAuthority
                );
                let session: (i64, Option<Vec<u8>>, i16, Option<Vec<u8>>) = sqlx::query_as(
                    "SELECT current_generation::BIGINT, current_transport_ref, session_state, prepared_attempt_ref \
                     FROM game_durability_reconnect_sessions \
                     WHERE game_session_id = encode($1, 'hex')::uuid",
                )
                .bind(session_id.as_slice())
                .fetch_one(&lock_pool)
                .await?;
                assert_eq!(session.0, 7);
                assert!(session.1.is_none());
                assert_eq!(session.2, 1);
                assert!(session.3.is_none());
                let consumed: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM game_durability_recovery_grant_consumptions",
                )
                .fetch_one(&lock_pool)
                .await?;
                assert_eq!(consumed, 0, "expired commit must not consume recovery nonce");
                assert_eq!(
                    journal.reconcile(&prepare).await?,
                    oteryn_game_server::foundation::ReconnectDurableReconciliationSnapshotV1::terminal(
                        prepare.record().clone(),
                    )
                );
                lock_pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn prepare_row_lock_wait_cannot_outlive_prepared_deadline() -> Result<(), Box<dyn std::error::Error>>
{
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("prepare_deadline_lock").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let lock_pool = sqlx::PgPool::connect(&database_url).await?;
                let record_now = postgres_clock(&lock_pool).await?;
                let prepared_deadline = record_now + 2;
                let (_flow, request) = ReconnectDurabilityFlowV1::begin(
                    record_with_prepared_deadline(91, 1, 0xc1, record_now, prepared_deadline)
                        .map_err(foundation_error)?,
                );
                let session_id = request
                    .record()
                    .identity()
                    .game_session_id()
                    .as_bytes()
                    .to_vec();
                sqlx::query(
                    "INSERT INTO game_durability_reconnect_sessions (\
                        game_session_id, account_id, character_id, world_id, runtime_scope_kind, \
                        runtime_scope_world_id, runtime_scope_channel_id, runtime_scope_instance_id, \
                        control_loss_epoch, original_grace_deadline, predecessor_generation, \
                        character_lease_generation, scope_ownership_generation, current_generation\
                     ) VALUES (encode($1, 'hex')::uuid, '123e4567-e89b-12d3-a456-426614174000'::uuid, \
                        encode($2, 'hex')::uuid, encode($3, 'hex')::uuid, 1, encode($3, 'hex')::uuid, \
                        encode($4, 'hex')::uuid, NULL, 3, $5, 7, 9, 10, 7)",
                )
                .bind(session_id.as_slice())
                .bind(uuid_v7(11).as_slice())
                .bind(uuid_v7(12).as_slice())
                .bind(uuid_v7(13).as_slice())
                .bind(record_now + 120)
                .execute(&lock_pool)
                .await?;
                assert!(postgres_clock(&lock_pool).await? <= prepared_deadline);

                let mut lock = lock_pool.begin().await?;
                sqlx::query(
                    "SELECT game_session_id FROM game_durability_reconnect_sessions \
                     WHERE game_session_id = encode($1, 'hex')::uuid FOR UPDATE",
                )
                .bind(session_id.as_slice())
                .fetch_one(&mut *lock)
                .await?;

                let blocked_journal = journal.clone();
                let blocked_request = request.clone();
                let blocked = tokio::spawn(async move {
                    blocked_journal.prepare(&blocked_request).await
                });
                tokio::time::sleep(Duration::from_millis(100)).await;
                assert!(
                    !blocked.is_finished(),
                    "prepare must be waiting on the held per-session row lock"
                );
                while postgres_clock(&lock_pool).await? <= prepared_deadline {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                lock.commit().await?;

                let blocked_result = blocked.await?;
                assert_eq!(
                    blocked_result?,
                    ReconnectPrepareDispositionV1::RejectedStaleAuthority
                );
                let replay_result = journal.prepare(&request).await;
                assert_eq!(
                    replay_result?,
                    ReconnectPrepareDispositionV1::ExistingTerminal
                );
                let session: (i64, Option<Vec<u8>>, i16, i16, Option<Vec<u8>>) = sqlx::query_as(
                    "SELECT current_generation::BIGINT, current_transport_ref, session_state, attempt_count, prepared_attempt_ref \
                     FROM game_durability_reconnect_sessions \
                     WHERE game_session_id = encode($1, 'hex')::uuid",
                )
                .bind(session_id.as_slice())
                .fetch_one(&lock_pool)
                .await?;
                assert_eq!(session.0, 7);
                assert!(session.1.is_none());
                assert_eq!(session.2, 1);
                assert_eq!(session.3, 1);
                assert!(session.4.is_none());
                lock_pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}
