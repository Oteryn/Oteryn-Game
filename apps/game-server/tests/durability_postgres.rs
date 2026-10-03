// Include the unchanged Foundation source in this test crate so privately sealed
// fixture owners and Durability use one type universe, without a production seal.
extern crate oteryn_game_server as production_server;
extern crate self as oteryn_game_server;
pub use production_server::admission_evidence;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_recovery_fence.rs"]
pub mod character_recovery_fence;
pub use production_server::domain;
#[path = "../src/foundation/mod.rs"]
pub mod foundation;
#[allow(dead_code, unused_imports)]
#[path = "../src/native_admission_source/mod.rs"]
pub mod native_admission_source;

#[path = "support/authority_matrix.rs"]
mod authority_matrix;
#[path = "support/authority_recovery.rs"]
mod authority_recovery;
#[path = "../src/durability/mod.rs"]
mod durability;
#[path = "support/postgres.rs"]
mod postgres;
use sqlx::Connection;

static PROCESS_ROOT_LEDGER_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

async fn seed_shared_root_replacement_predecessor(
    url: &str,
    seed: authority_matrix::Seed,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut connection = <sqlx::PgConnection as sqlx::Connection>::connect(url).await?;
    sqlx::query(
        "INSERT INTO game_durability_reconnect_sessions (\
            game_session_id, account_id, character_id, world_id, runtime_scope_kind, \
            runtime_scope_world_id, runtime_scope_channel_id, runtime_scope_instance_id, \
            control_loss_epoch, original_grace_deadline, predecessor_generation, \
            character_lease_generation, scope_ownership_generation, current_generation, session_state\
         ) VALUES (\
            encode($1, 'hex')::uuid, $2::text::uuid, encode($3, 'hex')::uuid, \
            encode($4, 'hex')::uuid, 1, encode($4, 'hex')::uuid, \
            encode($5, 'hex')::uuid, NULL, 3, $6, 7, 9, 9, 7, 1\
         )",
    )
    .bind(authority_matrix::uuid(10).as_slice())
    .bind(authority_matrix::ACCOUNT)
    .bind(authority_matrix::uuid(seed.character).as_slice())
    .bind(authority_matrix::uuid(12).as_slice())
    .bind(authority_matrix::uuid(13).as_slice())
    .bind(seed.now + 120)
    .execute(&mut connection)
    .await?;
    sqlx::query(
        "INSERT INTO game_durability_control_loss_continuity (\
            character_id, control_loss_epoch, account_id, world_id, context_game_session_id, \
            original_grace_deadline, protection_entitlement_state, protection_rearm_state\
         ) VALUES (\
            encode($1, 'hex')::uuid, 3, $2::text::uuid, encode($3, 'hex')::uuid, \
            encode($4, 'hex')::uuid, $5, 1, 1\
         )",
    )
    .bind(authority_matrix::uuid(seed.character).as_slice())
    .bind(authority_matrix::ACCOUNT)
    .bind(authority_matrix::uuid(12).as_slice())
    .bind(authority_matrix::uuid(10).as_slice())
    .bind(seed.now + 120)
    .execute(&mut connection)
    .await?;
    // This positive fixture explicitly supplies complete owning history; production
    // migration never infers completeness from the predecessor row.
    sqlx::query("INSERT INTO game_durability_session_use_ledgers (character_id, version, complete, revision, revision_floor) VALUES (encode($1,'hex')::uuid,1,TRUE,1,1)")
        .bind(authority_matrix::uuid(seed.character).as_slice()).execute(&mut connection).await?;
    sqlx::query("INSERT INTO game_durability_session_use_memberships (game_session_id, character_id, membership_revision, operation_binding) VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,$3)")
        .bind(authority_matrix::uuid(10).as_slice()).bind(authority_matrix::uuid(seed.character).as_slice()).bind([1_u8;16].as_slice()).execute(&mut connection).await?;
    connection.close().await?;
    Ok(())
}

#[test]
fn owning_loss_codec_retains_distinct_protection_and_source_generations()
-> Result<(), Box<dyn std::error::Error>> {
    use authority_matrix::checked;
    use durability::fresh_admission::{decode_fresh_loss, encode_fresh_loss};
    use foundation::*;
    let facts = checked(FreshAdmissionFacts::new(
        [7; 32],
        authority_matrix::character(2)?,
        authority_matrix::world(3)?,
        authority_matrix::channel(4)?,
        2,
        1,
    ))?;
    let commit = checked(FreshAdmissionCommit::from_facts(
        authority_matrix::session(9)?,
        facts,
        authority_matrix::transport(10)?,
    ))?;
    let session = checked(GameSessionAuthoritySnapshot::from_current_facts(
        commit,
        GameSessionState::Active,
        commit.connection_generation(),
        Some(commit.initial_transport()),
        checked(CharacterLease::new(commit.character_id(), 2))?,
        Some(CharacterWorldEligibilityClaimV1::new(
            commit.character_id(),
            commit.world_id(),
        )),
        RuntimeScopeRefV1::channel(commit.world_id(), commit.channel_id()),
        checked(ScopeOwnershipGeneration::new(1))?,
    ))?;
    let operation = ControlLossOperationV1 {
        version: 1,
        authorized_at: 100,
        observation: ControlLossObservationV1 {
            source_authority: session.current_runtime_scope(),
            source_revision: 1,
            accepted_source_revision: 1,
            decision_identity: checked(ControlLossEpochRefV1::new(1))?,
            accepted_decision_identity: checked(ControlLossEpochRefV1::new(1))?,
            observed_at: 100,
            session,
            account_presence: checked(AccountPresenceClaimV1::new(
                "00000000-0000-4000-8000-000000000001",
                commit.character_id(),
            ))?,
            placement_identity: [9; 16],
            placement_revision: 1,
            actor_present: true,
            runtime_ready: true,
            cause: ControlLossCauseV1::AuthoritativeUnexpectedLoss,
            loss_epoch: checked(ControlLossEpochRefV1::new(1))?,
            loss_origin: 100,
            original_grace_deadline: 220,
            history: ControlLossHistoryV1::FreshOrigin,
            protection: RecoveryProtectionContinuityV1 {
                usage: RecoveryProtectionUseV1::Unused {
                    entitlement_generation: 1,
                },
                rearm: RecoveryProtectionRearmV1::Satisfied {
                    generation: 7,
                    established_at: 90,
                },
            },
        },
    };
    let encoded = encode_fresh_loss(&operation)?;
    assert_eq!(encode_fresh_loss(&operation)?, encoded);
    assert_eq!(decode_fresh_loss(&encoded, commit)?, operation);
    let mut restored = checked(ControlLossFlowV1::restore(decode_fresh_loss(
        &encoded, commit,
    )?))?;
    assert!(
        restored.take_request().is_err(),
        "decoded history cannot yield live authority"
    );
    for end in 0..encoded.len() {
        assert!(decode_fresh_loss(&encoded[..end], commit).is_err());
    }
    assert!(decode_fresh_loss(&(encoded.clone() + " "), commit).is_err());
    use base64::Engine;
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(
        serde_json::from_str::<serde_json::Value>(&encoded)?["payload"]
            .as_str()
            .ok_or("missing payload")?,
    )?;
    let wrap = |bytes: &[u8]| {
        format!(
            "{{\"version\":1,\"payload\":\"{}\"}}",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
        )
    };
    for end in 0..payload.len() {
        assert!(decode_fresh_loss(&wrap(&payload[..end]), commit).is_err());
    }
    let mut malformed = payload.clone();
    malformed.push(0);
    assert!(decode_fresh_loss(&wrap(&malformed), commit).is_err());
    malformed = payload.clone();
    malformed[0] = 2;
    assert!(decode_fresh_loss(&wrap(&malformed), commit).is_err());
    malformed = payload;
    malformed[1..9].copy_from_slice(&0i64.to_be_bytes());
    assert!(
        decode_fresh_loss(&wrap(&malformed), commit).is_err(),
        "historically impossible authorization must reject"
    );
    assert!(decode_fresh_loss(&"x".repeat(65_537), commit).is_err());
    let other_commit = checked(FreshAdmissionCommit::from_facts(
        authority_matrix::session(8)?,
        facts,
        commit.initial_transport(),
    ))?;
    assert!(decode_fresh_loss(&encoded, other_commit).is_err());
    let mut changed = operation.clone();
    changed.observation.protection.usage = RecoveryProtectionUseV1::Unused {
        entitlement_generation: 2,
    };
    assert_ne!(encode_fresh_loss(&changed)?, encoded);
    assert_eq!(
        decode_fresh_loss(&encode_fresh_loss(&changed)?, commit)?,
        changed
    );
    changed = operation.clone();
    changed.observation.protection.rearm = RecoveryProtectionRearmV1::Satisfied {
        generation: 8,
        established_at: 90,
    };
    assert_ne!(encode_fresh_loss(&changed)?, encoded);
    assert_eq!(
        decode_fresh_loss(&encode_fresh_loss(&changed)?, commit)?,
        changed
    );
    changed = operation.clone();
    changed.observation.source_revision = 2;
    changed.observation.accepted_source_revision = 2;
    assert_ne!(encode_fresh_loss(&changed)?, encoded);
    assert_eq!(
        decode_fresh_loss(&encode_fresh_loss(&changed)?, commit)?,
        changed
    );
    changed = operation;
    changed.observation.protection.usage = RecoveryProtectionUseV1::NotEntitled;
    assert_ne!(encode_fresh_loss(&changed)?, encoded);
    assert_eq!(
        decode_fresh_loss(&encode_fresh_loss(&changed)?, commit)?,
        changed
    );
    changed.observation.protection.rearm = RecoveryProtectionRearmV1::NotRearmed {
        generation: 8,
        stable_control_started_at: Some(90),
        accepted_deadline: Some(110),
    };
    assert_ne!(encode_fresh_loss(&changed)?, encoded);
    assert_eq!(
        decode_fresh_loss(&encode_fresh_loss(&changed)?, commit)?,
        changed
    );
    changed.observation.protection.rearm = RecoveryProtectionRearmV1::NotRearmed {
        generation: 8,
        stable_control_started_at: None,
        accepted_deadline: None,
    };
    assert_eq!(
        decode_fresh_loss(&encode_fresh_loss(&changed)?, commit)?,
        changed
    );
    changed.observation.protection.usage = RecoveryProtectionUseV1::Activated {
        entitlement_generation: 2,
        activated_at: 95,
        deadline: 99,
    };
    assert!(
        encode_fresh_loss(&changed).is_err(),
        "Activated is not lawful FreshOrigin history"
    );
    Ok(())
}

#[test]
fn owning_fresh_loss_is_atomic_and_raw_prepare_does_not_supply_authority()
-> Result<(), Box<dyn std::error::Error>> {
    use durability::admission_authority_guards::AdmissionGuardStore;
    use durability::fresh_admission::{
        FreshAdmissionStore, FreshLossReconciliation, FreshReconciliation,
    };
    use foundation::admission_authority_publication::*;
    use foundation::*;
    struct LossSource(std::sync::Mutex<ControlLossObservationV1>);
    impl foundation::fnd04_verifier::recovery_source_sealed::Sealed for LossSource {}
    impl ControlLossSourceV1 for LossSource {
        fn resolve_loss(
            &self,
            _: GameSessionId,
            _: i64,
        ) -> Result<ControlLossObservationV1, ReconnectDurabilityErrorV1> {
            Ok(match self.0.lock() {
                Ok(observation) => observation.clone(),
                Err(poisoned) => poisoned.into_inner().clone(),
            })
        }
    }
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        for scenario in 0..6 {
        let not_entitled = scenario == 1;
        let database = postgres::IsolatedPostgres::create("owning_fresh_loss").await?;
        let result = async {
            let url = database.database_url()?;
            MigrationExecutor::connect_migration(&url).await?.apply_embedded_ledger().await?;
            let pool = sqlx::PgPool::connect(&url).await?;
            let now = postgres_clock(&pool).await?;
            let mut owner = postgres::fresh::Source::new(now)?;
            let guards = AdmissionGuardStore::connect_runtime(&url, 8192).await?;
            guards.publish(&authority_matrix::checked(AdmissionAuthorityPublicationV1::prepare(&owner, now))?).await?;
            let store = FreshAdmissionStore::connect_runtime(&url, 65536, 8192).await?;
            let request = owner.request()?;
            store.commit(&request).await?;
            let FreshReconciliation::Committed(initial) = store.reconcile(request.operation()).await? else { return Err("missing fresh session".into()); };
            let session = initial.current_session;
            // Loss is timed on the database clock sampled with the session.
            let (timed, clock) = store.current_session_at(session.commit().game_session_id()).await?;
            assert_eq!(timed, session);
            assert!((now..=now + 5).contains(&clock));
            let source = std::sync::Arc::new(LossSource(std::sync::Mutex::new(ControlLossObservationV1 {
                source_authority: session.current_runtime_scope(), source_revision: 1, accepted_source_revision: 1,
                decision_identity: authority_matrix::checked(ControlLossEpochRefV1::new(1))?,
                accepted_decision_identity: authority_matrix::checked(ControlLossEpochRefV1::new(1))?,
                observed_at: now, session,
                account_presence: authority_matrix::checked(AccountPresenceClaimV1::new("00000000-0000-4000-8000-000000000001", session.commit().character_id()))?,
                placement_identity: [9;16], placement_revision: 1, actor_present: true, runtime_ready: true,
                cause: ControlLossCauseV1::AuthoritativeUnexpectedLoss,
                loss_epoch: authority_matrix::checked(ControlLossEpochRefV1::new(1))?, loss_origin: now,
                original_grace_deadline: if scenario == 5 { now + 2 } else { now + 120 }, history: ControlLossHistoryV1::FreshOrigin,
                protection: RecoveryProtectionContinuityV1 {
                    usage: if not_entitled { RecoveryProtectionUseV1::NotEntitled } else { RecoveryProtectionUseV1::Unused { entitlement_generation: 1 } },
                    rearm: if not_entitled { RecoveryProtectionRearmV1::NotRearmed { generation: 7, stable_control_started_at: Some(now - 10), accepted_deadline: Some(now + 10) } } else { RecoveryProtectionRearmV1::Satisfied { generation: 1, established_at: now } },
                },
            })));
            let template = authority_matrix::prepared_record(authority_matrix::Seed { now, generation: 1, epoch: 1, transport: 44, ..authority_matrix::Seed::fixed() })?;
            let raw = authority_matrix::checked(ReconnectDurabilityRecordV1::new(
                authority_matrix::checked(ReconnectIdentityV1::new(session.commit().game_session_id(), template.identity().reconnect_attempt_ref(), "00000000-0000-4000-8000-000000000001", session.commit().character_id(), session.commit().world_id(), session.current_runtime_scope()))?,
                template.connection(),
                authority_matrix::checked(ReconnectAuthorityFenceV1::new(session.current_character_lease().generation(), session.current_scope_generation()))?,
                template.continuity(), template.proof().clone(), template.fnd02().clone(), template.compatibility().clone(),
            ))?;
            // Regression for the rejected legacy projection: candidate generation
            // is 2 while owning entitlement/rearm generations are 1. They are
            // separate namespaces, never a lawful protection-fence conversion.
            assert_eq!(raw.connection().candidate().get(), 2);
            assert!(not_entitled || matches!(source.0.lock().map_err(|_| "loss source lock poisoned")?.protection.usage, RecoveryProtectionUseV1::Unused { entitlement_generation: 1 }));
            let raw_v1 = ReconnectDurabilityFlowV1::begin(raw.clone()).1;
            let raw_v2 = ReconnectDurabilityFlowV2::begin(raw, None).1;
            let reconnect = durability::AdmissionReconnectJournalV2::connect_runtime(&url).await?;
            assert_eq!(reconnect.legacy().prepare(&raw_v1).await?, ReconnectPrepareDispositionV1::RejectedStaleAuthority);
            assert_eq!(reconnect.prepare(&raw_v2).await?, ReconnectPrepareDispositionV2::RejectedStaleAuthority);
            let authorization = authority_matrix::checked(ControlLossAuthorizationV1::authorize(source.as_ref(), session.commit().game_session_id(), now))?;
            let mut flow = ControlLossFlowV1::begin(authorization);
            let loss = std::sync::Arc::new(authority_matrix::checked(flow.take_request())?);
            assert_eq!(store.reconcile_fresh_loss(loss.operation()).await?, FreshLossReconciliation::Absent);
            if scenario >= 4 {
                // A later, refused admission attempt for the same account
                // re-observes Platform security and republishes the Account
                // row. Presence still names this session, so loss commits.
                let key = owner.rows[0].key.clone();
                let mut row = guards.load(&[key]).await?.pop().flatten().ok_or("missing account row")?;
                row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet { expected_publication_revision: row.publication_revision };
                row.publication_revision += 1;
                row.source.source_revision += 1;
                row.source.decision_identity = "platform-refresh".into();
                if let AdmissionAuthorityGuardStateV1::Account { security, presence } = &mut row.state {
                    security.provenance.publication_revision = row.publication_revision;
                    assert_eq!(*presence, Some((session.commit().character_id(), session.commit().game_session_id())));
                } else { return Err("missing account fixture".into()); }
                owner.rows = vec![row];
                let publication = authority_matrix::checked(AdmissionAuthorityPublicationV1::prepare(&owner, now))?;
                assert_eq!(guards.publish(&publication).await?, durability::admission_authority_guards::GuardPublicationDisposition::Applied);
            }
            if scenario == 5 {
                // Grace expiry: the loss commits, is not released before its
                // deadline, then releases from the *current* (refreshed) rows.
                let account = "00000000-0000-4000-8000-000000000001";
                let id = session.commit().game_session_id();
                assert!(matches!(store.commit_fresh_loss(loss.clone(), source.clone()).await?, ControlLossOutcomeV1::Committed { .. }));
                assert!(matches!(store.release_expired_loss(id, account).await?, durability::fresh_admission::ExpiredLossReleaseV1::NotExpired { .. }));
                tokio::time::sleep(std::time::Duration::from_millis(3100)).await;
                // Another account has no claim rows naming this session.
                assert_eq!(store.release_expired_loss(id, "00000000-0000-4000-8000-000000000099").await.ok(), None);
                let durability::fresh_admission::ExpiredLossReleaseV1::Released { decided_at } = store.release_expired_loss(id, account).await? else { return Err("expired loss was not released".into()); };
                assert!(decided_at >= now + 2);
                let (released, _) = store.current_session_at(id).await?;
                assert_eq!(released.session_state(), GameSessionState::Terminal);
                assert_eq!(released.current_character_lease(), session.current_character_lease());
                let keys = vec![AdmissionAuthorityGuardKeyV1::Account { account_id: account.into() }, AdmissionAuthorityGuardKeyV1::Character(session.commit().character_id())];
                let rows = guards.load(&keys).await?;
                assert!(matches!(rows[0].as_ref().map(|row| &row.state), Some(AdmissionAuthorityGuardStateV1::Account { presence: None, .. })));
                assert!(matches!(rows[1].as_ref().map(|row| &row.state), Some(AdmissionAuthorityGuardStateV1::Character { holder: None, lease_generation, .. }) if *lease_generation == session.current_character_lease().generation()));
                // Terminal (also after a lost acknowledgement): nothing further to
                // release, and the owner may remove the exact actor.
                assert_eq!(store.release_expired_loss(id, account).await?, durability::fresh_admission::ExpiredLossReleaseV1::Terminal);
                pool.close().await;
                return Ok(());
            }
            if (2..4).contains(&scenario) {
                // Publish one independently valid current runtime change while
                // leaving the session, claims, and loss source exactly unchanged.
                owner.rows.retain(|row| matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
                let row = &mut owner.rows[0];
                row.publication_revision = 2;
                row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet { expected_publication_revision: 1 };
                row.source.source_revision = 2;
                row.source.decision_identity = "runtime-owner-next".into();
                if let AdmissionAuthorityGuardStateV1::Runtime { ownership_generation, ready, .. } = &mut row.state {
                    if scenario == 2 { *ownership_generation = 2; } else { *ready = false; }
                } else { return Err("missing runtime fixture".into()); }
                let publication = authority_matrix::checked(AdmissionAuthorityPublicationV1::prepare(&owner, now))?;
                assert_eq!(guards.publish(&publication).await?, durability::admission_authority_guards::GuardPublicationDisposition::Applied);
                assert_eq!(guards.load(&[owner.rows[0].key.clone()]).await?, vec![Some(owner.rows[0].clone())]);
                assert_eq!(store.commit_fresh_loss(loss.clone(), source.clone()).await?, ControlLossOutcomeV1::Rejected);
                assert_eq!(store.reconcile(request.operation()).await?, FreshReconciliation::Committed(initial.clone()));
                assert_eq!(source.0.lock().map_err(|_| "loss source lock poisoned")?.session, session);
                let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_admission_lifecycle_receipts").fetch_one(&pool).await?;
                assert_eq!(receipts, 0);
                pool.close().await;
                return Ok(());
            }
            source.0.lock().map_err(|_| "loss source lock poisoned")?.cause = ControlLossCauseV1::HealthyController;
            assert_eq!(store.commit_fresh_loss(loss.clone(), source.clone()).await?, ControlLossOutcomeV1::Rejected);
            assert_eq!(store.reconcile(request.operation()).await?, FreshReconciliation::Committed(initial.clone()));
            source.0.lock().map_err(|_| "loss source lock poisoned")?.cause = ControlLossCauseV1::AuthoritativeUnexpectedLoss;
            // Force the final receipt effect to fail after tentative session write; complete loss truth must roll back together.
            sqlx::query("CREATE FUNCTION reject_test_loss_receipt() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'loss receipt test rollback' USING ERRCODE = '23514'; END; $$")
                .execute(&pool).await?;
            sqlx::query("CREATE TRIGGER reject_test_loss_receipt BEFORE INSERT ON game_durability_admission_lifecycle_receipts FOR EACH ROW EXECUTE FUNCTION reject_test_loss_receipt()")
                .execute(&pool).await?;
            let rollback = store.commit_fresh_loss(loss.clone(), source.clone()).await;
            assert!(matches!(rollback, Err(DurabilityError::Database(_))));
            assert_eq!(store.reconcile(request.operation()).await?, FreshReconciliation::Committed(initial.clone()));
            let continuity: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_control_loss_continuity").fetch_one(&pool).await?;
            assert_eq!(continuity, 0);
            sqlx::query("DROP TRIGGER reject_test_loss_receipt ON game_durability_admission_lifecycle_receipts").execute(&pool).await?;
            let outcome = store.commit_fresh_loss(loss.clone(), source.clone()).await?;
            assert!(matches!(outcome, ControlLossOutcomeV1::Committed { .. }));
            assert_eq!(store.commit_fresh_loss(loss.clone(), source.clone()).await?, outcome);
            let FreshReconciliation::Committed(current) = store.reconcile(request.operation()).await? else { return Err("missing loss session".into()); };
            assert_eq!(current.current_session.session_state(), GameSessionState::Reconnectable);
            assert_eq!(current.current_session.current_transport(), None);
            assert_eq!(current.current_session.current_control_loss_epoch().map(ControlLossEpochRefV1::get), Some(1));
            assert_eq!(current.current_session.current_character_lease(), session.current_character_lease());
            assert_eq!(current.receipt, initial.receipt);
            let reopened = FreshAdmissionStore::connect_runtime(&url, 65536, 8192).await?;
            let FreshLossReconciliation::Committed { completion, current: recovered_current } = reopened.reconcile_fresh_loss(loss.operation()).await? else { return Err("missing durable original loss".into()); };
            assert_eq!(completion.operation, *loss.operation());
            assert_eq!(completion.outcome, outcome);
            assert_eq!(recovered_current, current);
            let mut durable_source = reopened.loss_completion_source(loss.operation()).await?.ok_or("missing durable completion source")?;
            assert_eq!(durable_source.current_snapshot(), current.as_ref());
            assert_eq!(durable_source.take_loss_completion(loss.operation()).map_err(|_| "durable completion failed")?, Some(*completion.clone()));
            assert_eq!(durable_source.take_loss_completion(loss.operation()).map_err(|_| "durable completion failed")?, None);
            let mut historical = authority_matrix::checked(ControlLossFlowV1::restore(completion.operation.clone()))?;
            assert!(historical.take_request().is_err());
            let mut conflicting = loss.operation().clone();
            conflicting.observation.source_revision = 2;
            conflicting.observation.accepted_source_revision = 2;
            assert_eq!(reopened.reconcile_fresh_loss(&conflicting).await?, FreshLossReconciliation::Conflict);
            let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_admission_lifecycle_receipts").fetch_one(&pool).await?;
            assert_eq!(receipts, 1);
            let attempts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_reconnect_attempts").fetch_one(&pool).await?;
            assert_eq!(attempts, 0);
            assert_eq!(reconnect.legacy().prepare(&raw_v1).await?, ReconnectPrepareDispositionV1::Unavailable);
            assert_eq!(reconnect.prepare(&raw_v2).await?, ReconnectPrepareDispositionV2::Unavailable);
            // A valid historical DTO for a different account cannot be paired
            // with this canonical fresh receipt, even though the session matches.
            let mut wrong_account = loss.operation().clone();
            wrong_account.observation.account_presence = authority_matrix::checked(AccountPresenceClaimV1::new("00000000-0000-4000-8000-000000000099", session.commit().character_id()))?;
            let bad_original = durability::fresh_admission::encode_fresh_loss(&wrong_account)?;
            sqlx::query("ALTER TABLE game_durability_admission_lifecycle_receipts DISABLE TRIGGER USER").execute(&pool).await?;
            sqlx::query("UPDATE game_durability_admission_lifecycle_receipts SET operation_json = $1").bind(bad_original).execute(&pool).await?;
            sqlx::query("ALTER TABLE game_durability_admission_lifecycle_receipts ENABLE TRIGGER USER").execute(&pool).await?;
            assert!(matches!(reopened.reconcile_fresh_loss(loss.operation()).await, Err(DurabilityError::InvalidStoredState)));
            sqlx::query("ALTER TABLE game_durability_admission_lifecycle_receipts DISABLE TRIGGER USER").execute(&pool).await?;
            sqlx::query("UPDATE game_durability_admission_lifecycle_receipts SET operation_json = $1").bind(durability::fresh_admission::encode_fresh_loss(loss.operation())?).execute(&pool).await?;
            sqlx::query("ALTER TABLE game_durability_admission_lifecycle_receipts ENABLE TRIGGER USER").execute(&pool).await?;
            // One mirror corruption: predecessor2 cannot precede current1.
            sqlx::query("UPDATE game_durability_reconnect_sessions SET predecessor_generation = 2").execute(&pool).await?;
            assert!(matches!(reopened.reconcile_fresh_loss(loss.operation()).await, Err(DurabilityError::InvalidStoredState)));
            assert!(matches!(store.reconcile(request.operation()).await, Err(DurabilityError::InvalidStoredState)));
            sqlx::query("UPDATE game_durability_reconnect_sessions SET predecessor_generation = 1").execute(&pool).await?;
            sqlx::query("UPDATE game_durability_reconnect_sessions SET original_grace_deadline = original_grace_deadline + 1").execute(&pool).await?;
            assert!(matches!(reopened.reconcile_fresh_loss(loss.operation()).await, Err(DurabilityError::InvalidStoredState)));
            sqlx::query("UPDATE game_durability_reconnect_sessions SET original_grace_deadline = original_grace_deadline - 1").execute(&pool).await?;
            // Corrupt only the stored original decision time in this isolated
            // fixture; exact bytes alone must not authenticate an impossible L.
            sqlx::query("ALTER TABLE game_durability_admission_lifecycle_receipts DISABLE TRIGGER USER").execute(&pool).await?;
            sqlx::query("UPDATE game_durability_admission_lifecycle_receipts SET decided_at = 0").execute(&pool).await?;
            sqlx::query("ALTER TABLE game_durability_admission_lifecycle_receipts ENABLE TRIGGER USER").execute(&pool).await?;
            assert!(matches!(store.commit_fresh_loss(loss.clone(), source.clone()).await, Err(DurabilityError::InvalidStoredState)));
            assert!(matches!(reopened.reconcile_fresh_loss(loss.operation()).await, Err(DurabilityError::InvalidStoredState)));
            assert_eq!(store.reconcile(request.operation()).await?, FreshReconciliation::Committed(current));
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
            "game_durability_session_use_ledgers",
            "game_durability_session_use_memberships",
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
                if !matches!(error, DurabilityError::Database(_)) {
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
                let membership_effects: (i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_durability_session_use_ledgers), (SELECT count(*) FROM game_durability_session_use_memberships)").fetch_one(&pool).await?;
                assert_eq!(membership_effects,(0,0),"membership survived failure at {table}");
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
fn owning_loss_fences_v2_replacement_and_reconciliation() -> Result<(), Box<dyn std::error::Error>>
{
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        use authority_matrix::{LiveSource, Seed, prepared_record};
        use foundation::{ReconnectDurabilityFlowV2, ReconnectPrepareDispositionV2};
        for replacement in [false, true] {
            let database = postgres::IsolatedPostgres::create("owning_loss_v2_fence").await?;
            let result = async {
                let url = database.database_url()?;
                MigrationExecutor::connect_migration(&url).await?.apply_embedded_ledger().await?;
                let seed = Seed { now: unix_now().map_err(foundation_error)?, ..Seed::fixed() };
                let record = prepared_record(seed)?;
                let source = LiveSource::read(seed);
                if replacement { seed_shared_root_replacement_predecessor(&url, seed).await?; }
                let authorization = if replacement { Some(source.authorize_replacement(&record)?) } else { None };
                let (_, request) = ReconnectDurabilityFlowV2::begin(record.clone(), authorization);
                let journal = durability::AdmissionReconnectJournalV2::connect_runtime(&url).await?;
                assert_eq!(journal.prepare(&request).await?, ReconnectPrepareDispositionV2::Prepared);
                let before = journal.reconcile(&request).await?;
                let mut connection = sqlx::PgConnection::connect(&url).await?;
                // Change only the durable owning-loss discriminator. The independently
                // valid legacy record and current authority remain unchanged.
                let mut key = b"owning-loss-v1".to_vec();
                let loss_session = if replacement { authority_matrix::session(10)? } else { record.identity().game_session_id() };
                key.extend_from_slice(loss_session.as_bytes());
                key.extend_from_slice(&record.continuity().control_loss_epoch().get().to_be_bytes());
                sqlx::query("INSERT INTO game_durability_admission_lifecycle_receipts (operation_key, operation_json, decided_at) VALUES ($1, '{}', $2)")
                    .bind(key).bind(seed.now).execute(&mut connection).await?;
                connection.close().await?;
                assert_eq!(journal.prepare(&request).await?, ReconnectPrepareDispositionV2::Unavailable);
                assert!(matches!(journal.reconcile(&request).await, Err(DurabilityError::Unavailable)), "owning loss must not be projected as legacy reconciliation: {before:?}");
                let mut connection = sqlx::PgConnection::connect(&url).await?;
                let attempts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_reconnect_attempts").fetch_one(&mut connection).await?;
                assert_eq!(attempts, 1, "rejected retry must not write another attempt");
                connection.close().await?;
                Ok::<(), Box<dyn std::error::Error>>(())
            }.await;
            database.cleanup().await?;
            result?;
        }
        Ok(())
    })
}

#[test]
fn shared_root_positive_v1_v2_authority_matrix_is_configured_postgres_proof()
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
                let root = durability::DurabilityRoot::connect_test_runtime(&url)?;
                assert!(root.maintain_ready_once().await?);
                assert!(root.is_ready());
                assert!(!root.has_ready_demand());
                let journal = AdmissionReconnectJournal::from_root(root.clone());
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
                assert!(root.is_ready());
                assert!(!root.has_ready_demand());
                drop(journal);
                let reloaded = AdmissionReconnectJournal::from_root(root.clone());
                let v1 = reloaded.reconcile(&request).await?;
                let typed = durability::AdmissionReconnectJournalV2::from_root(root.clone());
                let (_, request_v2) =
                    oteryn_game_server::foundation::ReconnectDurabilityFlowV2::begin(
                        record.clone(),
                        None,
                    );
                let v2 = typed.reconcile(&request_v2).await?;
                assert!(root.is_ready());
                assert!(!root.has_ready_demand());
                authority_matrix::run_loaded_matrix(seed, &record, &source, v1, v2)?;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn shared_root_caller_cancellation_preserves_detached_journal_custody()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            use authority_matrix::{Seed, prepared_record};

            let database =
                postgres::IsolatedPostgres::create("shared_root_cancelled_caller").await?;
            let result = async {
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
                let (_flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
                let root = durability::DurabilityRoot::connect_test_runtime(&url)?;
                assert!(root.maintain_ready_once().await?);
                assert!(root.is_ready());
                assert!(!root.has_ready_demand());
                let journal = AdmissionReconnectJournal::from_root(root.clone());

                let mut blocker = <sqlx::PgConnection as sqlx::Connection>::connect(&url).await?;
                sqlx::query("BEGIN").execute(&mut blocker).await?;
                sqlx::query(
                    "LOCK TABLE game_durability_reconnect_attempts IN ACCESS EXCLUSIVE MODE",
                )
                .execute(&mut blocker)
                .await?;

                let caller_journal = journal.clone();
                let caller_request = request.clone();
                let caller =
                    tokio::spawn(async move { caller_journal.prepare(&caller_request).await });

                tokio::time::timeout(Duration::from_secs(1), async {
                    while root.is_ready() {
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .map_err(|_| std::io::Error::other("root holder was not checked out"))?;
                assert!(!root.has_ready_demand());

                caller.abort();
                let cancelled = match caller.await {
                    Ok(_) => {
                        return Err(
                            std::io::Error::other("caller task unexpectedly completed").into()
                        );
                    }
                    Err(cancelled) => cancelled,
                };
                assert!(cancelled.is_cancelled());
                assert!(
                    !root.is_ready(),
                    "caller cancellation released the root holder before detached work finished"
                );
                assert!(!root.has_ready_demand());

                sqlx::query("ROLLBACK").execute(&mut blocker).await?;
                blocker.close().await?;

                tokio::time::timeout(Duration::from_secs(1), async {
                    while !root.is_ready() {
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .map_err(|_| {
                    std::io::Error::other("detached journal task did not return holder")
                })?;
                assert!(!root.has_ready_demand());

                assert_eq!(
                    journal.prepare(&request).await?,
                    ReconnectPrepareDispositionV1::ExistingPrepared
                );
                assert_eq!(
                    journal.reconcile(&request).await?,
                    ReconnectDurableReconciliationSnapshotV1::prepared(record)
                );
                assert!(root.is_ready());
                assert!(!root.has_ready_demand());
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn shared_root_positive_v2_terminal_replacement_is_configured_postgres_proof()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            use authority_matrix::{LiveSource, Seed, checked, prepared_record, v2_budget};

            let database = postgres::IsolatedPostgres::create("shared_root_v2_replacement").await?;
            let result = async {
                let url = database.database_url()?;
                MigrationExecutor::connect_migration(&url)
                    .await?
                    .apply_embedded_ledger()
                    .await?;
                let seed = Seed {
                    now: unix_now().map_err(foundation_error)?,
                    ..Seed::fixed()
                };
                seed_shared_root_replacement_predecessor(&url, seed).await?;
                let record = prepared_record(seed)?;
                let source = LiveSource::read(seed);
                let root = durability::DurabilityRoot::connect_test_runtime(&url)?;
                assert!(root.maintain_ready_once().await?);
                assert!(root.is_ready());
                assert!(!root.has_ready_demand());

                let legacy = AdmissionReconnectJournal::from_root(root.clone());
                let typed = durability::AdmissionReconnectJournalV2::from_root(root.clone());
                let (mut flow, request) =
                    oteryn_game_server::foundation::ReconnectDurabilityFlowV2::begin(
                        record.clone(),
                        Some(source.authorize_replacement(&record)?),
                    );
                let disposition = typed.prepare(&request).await?;
                assert_eq!(
                    disposition,
                    oteryn_game_server::foundation::ReconnectPrepareDispositionV2::Prepared
                );
                assert!(root.is_ready());
                assert!(!root.has_ready_demand());

                let mut budget = v2_budget(seed)?;
                checked(flow.accept_prepare_completion(
                    oteryn_game_server::foundation::ReconnectPrepareCompletionV2::for_request(
                        &request,
                        disposition,
                    ),
                    &mut budget,
                ))?;
                let commit = checked(flow.authorize_commit(source.bind(&record)?, seed.now + 2))?;
                assert_eq!(
                    legacy.commit(&commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );
                assert!(root.is_ready());
                assert!(!root.has_ready_demand());
                assert_eq!(
                    checked(flow.accept_commit_completion(
                        ReconnectCommitCompletionV1::for_request(
                            &commit,
                            ReconnectCommitDispositionV1::Committed,
                        ),
                    ))?,
                    ReconnectCommitActionV1::ReconcileSameAttempt
                );

                let snapshot = typed.reconcile(&request).await?;
                assert!(root.is_ready());
                assert!(!root.has_ready_demand());
                assert!(matches!(
                    checked(flow.accept_reconciliation(
                        snapshot,
                        source.bind(&record)?,
                        &mut budget,
                    ))?,
                    oteryn_game_server::foundation::ReconnectProjectionDecisionV2::InstallController { .. }
                ));
                assert!(root.is_ready());
                assert!(!root.has_ready_demand());
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

const WP3_HOSTILE_PG_CHILD: &str = "OTERYN_WP3_HOSTILE_PG_CHILD";

#[test]
fn wp3_deterministic_pg_options_ignore_ambient_sources() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os(WP3_HOSTILE_PG_CHILD).is_some() {
        use sqlx::ConnectOptions;
        use sqlx::postgres::{PgConnectOptions, PgSslMode};

        let options = PgConnectOptions::from_str_without_environment(
            "postgresql://explicit@db.example/oteryn?hostaddr=127.0.0.1&sslmode=verify-full",
        )?;
        assert_eq!(options.get_host(), "db.example");
        assert_eq!(options.get_host_addr(), Some("127.0.0.1"));
        assert_eq!(options.get_port(), 5432);
        assert_eq!(options.get_username(), "explicit");
        assert_eq!(options.get_database(), Some("oteryn"));
        assert!(matches!(options.get_ssl_mode(), PgSslMode::VerifyFull));
        assert_eq!(options.to_url_lossy().password(), None);
        return Ok(());
    }

    let pgpass =
        std::env::temp_dir().join(format!("oteryn-wp3-hostile-pgpass-{}", std::process::id()));
    std::fs::write(&pgpass, "db.example:5432:oteryn:ambient:ambient-secret\n")?;
    let output = Command::new(std::env::current_exe()?)
        .arg("--exact")
        .arg("wp3_deterministic_pg_options_ignore_ambient_sources")
        .arg("--nocapture")
        .env(WP3_HOSTILE_PG_CHILD, "1")
        .env("PGHOST", "hostile.example")
        .env("PGHOSTADDR", "203.0.113.99")
        .env("PGPORT", "6543")
        .env("PGUSER", "ambient")
        .env("PGPASSWORD", "ambient-secret")
        .env("PGDATABASE", "ambient_db")
        .env("PGSSLMODE", "disable")
        .env("PGPASSFILE", &pgpass)
        .output()?;
    let _ = std::fs::remove_file(pgpass);
    if !output.status.success() {
        return Err(format!(
            "hostile PG child failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(())
}

#[test]
fn wp3_process_root_is_singleton_ready_only_and_detached_task_safe()
-> Result<(), Box<dyn std::error::Error>> {
    use durability::{DB_PASS_DEADLINE, DurabilityError, DurabilityRoot, DurabilityRootConfig};
    use std::net::{IpAddr, Ipv4Addr};
    use std::sync::mpsc;

    fn production_config() -> Result<DurabilityRootConfig, DurabilityError> {
        DurabilityRootConfig::new(
            IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7)),
            5432,
            "db.example",
            "oteryn",
            "explicit",
            "test-secret",
            b"-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n",
        )
    }

    let _process_root_guard = match PROCESS_ROOT_LEDGER_TEST_LOCK.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    let config = production_config()?;
    assert!(matches!(
        production_config(),
        Err(DurabilityError::RootUnavailable)
    ));
    let root = DurabilityRoot::new(config)?;

    assert_eq!(DB_PASS_DEADLINE, Duration::from_secs(2));
    assert!(!root.is_ready());
    assert!(root.has_ready_demand());
    assert!(matches!(
        root.try_acquire_ready(),
        Err(DurabilityError::RootUnavailable)
    ));

    let (started_tx, started_rx) = mpsc::channel();
    let detached_root = root.clone();
    let task = root.spawn_task(async move {
        assert!(
            started_tx.send(()).is_ok(),
            "singleton test receiver must remain alive"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
        drop(detached_root);
        7u8
    });
    started_rx.recv_timeout(Duration::from_secs(2))?;
    drop(root);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    assert_eq!(runtime.block_on(task)?, 7);
    assert!(matches!(
        production_config(),
        Err(DurabilityError::RootUnavailable)
    ));
    Ok(())
}

#[test]
fn wp3_root_journal_ready_miss_is_fail_closed_without_connect()
-> Result<(), Box<dyn std::error::Error>> {
    use durability::{DurabilityRoot, DurabilityRootConfig};
    use std::net::{IpAddr, Ipv4Addr};

    let root = DurabilityRoot::new(DurabilityRootConfig::new_with_isolated_test_ledger(
        IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7)),
        5432,
        "db.example",
        "oteryn",
        "explicit",
        "test-secret",
        b"-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n",
    )?)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let result = runtime.block_on(async {
        let journal = AdmissionReconnectJournal::from_root(root.clone());
        let (_flow, request) = ReconnectDurabilityFlowV1::begin(
            record(201, 1, 0xa1, unix_now().map_err(foundation_error)?)
                .map_err(foundation_error)?,
        );

        assert!(matches!(
            journal.prepare(&request).await,
            Err(DurabilityError::RootUnavailable)
        ));
        assert!(!root.is_ready());
        assert!(root.has_ready_demand());
        Ok::<(), Box<dyn std::error::Error>>(())
    });
    drop(runtime);
    drop(root);
    result
}

#[test]
fn wp3_process_scoped_root_keeps_final_runtime_owner_outside_async_tasks()
-> Result<(), Box<dyn std::error::Error>> {
    use durability::{DurabilityRoot, DurabilityRootConfig};
    use std::net::{IpAddr, Ipv4Addr};

    let root = DurabilityRoot::new(DurabilityRootConfig::new_with_isolated_test_ledger(
        IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7)),
        5432,
        "db.example",
        "oteryn",
        "explicit",
        "test-secret",
        b"-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n",
    )?)?;
    let async_owner = root.clone();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async move {
        tokio::task::yield_now().await;
        drop(async_owner);
    });

    drop(runtime);
    drop(root);
    Ok(())
}

#[test]
fn wp3_shared_root_deadline_retires_and_rearms_on_configured_postgres()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            use durability::{DB_PASS_DEADLINE, DurabilityRoot};

            let database = postgres::IsolatedPostgres::create("wp3_shared_root_deadline").await?;
            let result = async {
                let url = database.database_url()?;
                MigrationExecutor::connect_migration(&url)
                    .await?
                    .apply_embedded_ledger()
                    .await?;
                let root = DurabilityRoot::connect_test_runtime(&url)?;
                assert!(root.maintain_ready_once().await?);
                assert!(root.is_ready());
                assert!(!root.has_ready_demand());

                let issued = root.try_issue_semantic_pass()?;
                let result = issued
                    .run(|_holder, _deadline| {
                        Box::pin(async move {
                            tokio::time::sleep(DB_PASS_DEADLINE + Duration::from_millis(50)).await;
                            Ok(())
                        })
                    })
                    .await;

                assert!(matches!(
                    result,
                    Err(DurabilityError::RootPassDeadlineExceeded)
                ));
                assert!(!root.is_ready());
                assert!(root.has_ready_demand());
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn wp3_commit_outcome_unknown_retires_holder_and_preserves_success_path()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            use durability::{DurabilityError, DurabilityRoot};

            let database = postgres::IsolatedPostgres::create("wp3_commit_outcome_unknown").await?;
            let result = async {
                let url = database.database_url()?;
                MigrationExecutor::connect_migration(&url)
                    .await?
                    .apply_embedded_ledger()
                    .await?;
                let root = DurabilityRoot::connect_test_runtime(&url)?;
                assert!(root.maintain_ready_once().await?);

                let ambiguous: Result<(), DurabilityError> = root
                    .try_issue_semantic_pass()?
                    .run(|holder, _deadline| {
                        Box::pin(async move {
                            sqlx::query(
                                "SELECT set_config('oteryn.wp3_holder_marker', 'ambiguous', false)",
                            )
                            .execute(&mut **holder)
                            .await?;
                            Err(DurabilityError::CommitOutcomeUnknown)
                        })
                    })
                    .await;

                assert!(matches!(
                    ambiguous,
                    Err(DurabilityError::CommitOutcomeUnknown)
                ));
                assert!(!root.is_ready(), "ambiguous holder must not return to idle");
                assert!(root.has_ready_demand());
                assert!(root.maintain_ready_once().await?);

                let marker: Option<String> = root
                    .try_issue_semantic_pass()?
                    .run(|holder, _deadline| {
                        Box::pin(async move {
                            sqlx::query_scalar(
                                "SELECT current_setting('oteryn.wp3_holder_marker', true)",
                            )
                            .fetch_one(&mut **holder)
                            .await
                            .map_err(DurabilityError::from)
                        })
                    })
                    .await?;

                assert_eq!(marker, None, "recovery must use a successor holder");
                assert!(
                    root.is_ready(),
                    "successful pass must still return holder to idle"
                );
                assert!(!root.has_ready_demand());
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;

            database.cleanup().await?;
            result
        })
}

#[test]
fn wp3_root_readiness_requires_compatible_schema() -> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            use durability::DurabilityRoot;

            let database = postgres::IsolatedPostgres::create("wp3_root_schema_gate").await?;
            let result = async {
                let url = database.database_url()?;
                let root = DurabilityRoot::connect_test_runtime(&url)?;

                assert!(matches!(
                    root.maintain_ready_once().await,
                    Err(DurabilityError::SchemaIncompatible(
                        SchemaCompatibility::MissingMigrationLedger
                    ))
                ));
                assert!(!root.is_ready());
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;

            database.cleanup().await?;
            result
        })
}

#[test]
fn wp3_return_finality_deadline_hard_retires_exact_holder() -> Result<(), Box<dyn std::error::Error>>
{
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            use sqlx::pool::PoolConnectionReturnDisposition;
            use sqlx::postgres::PgPoolOptions;
            use std::time::Instant;

            let database = postgres::IsolatedPostgres::create("wp3_return_finality_deadline").await?;
            let result = async {
                let url = database.database_url()?;
                let pool = PgPoolOptions::new()
                    .max_connections(1)
                    .min_connections(0)
                    .after_release(|_connection, _metadata| {
                        Box::pin(async {
                            std::future::pending::<()>().await;
                            Ok(true)
                        })
                    })
                    .connect(&url)
                    .await?;

                let mut holder = pool.acquire().await?;
                let deadline = Instant::now() + Duration::from_millis(100);
                let disposition = holder.return_to_pool_observed_until(deadline).await;

                assert_eq!(
                    disposition,
                    PoolConnectionReturnDisposition::RetiredClosed,
                    "deadline expiry must retire the exact holder before reporting terminal finality"
                );
                assert_eq!(pool.num_idle(), 0);
                assert_eq!(pool.size(), 0, "retired generation must release pool capacity");
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;

            database.cleanup().await?;
            result
        })
}

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
fn existing_terminal_replay_precedes_cross_character_account_incumbent()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database =
                postgres::IsolatedPostgres::create("terminal_replay_before_incumbent").await?;
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
                sqlx::query(
                    "UPDATE game_durability_reconnect_attempts SET state = 4                      WHERE game_session_id = encode($1, 'hex')::uuid AND reconnect_attempt_ref = $2",
                )
                .bind(first.record().identity().game_session_id().as_bytes().as_slice())
                .bind(first.record().identity().reconnect_attempt_ref().to_be_bytes().as_slice())
                .execute(&pool)
                .await?;
                sqlx::query(
                    "UPDATE game_durability_reconnect_sessions SET session_state = 3                      WHERE game_session_id = encode($1, 'hex')::uuid",
                )
                .bind(first.record().identity().game_session_id().as_bytes().as_slice())
                .execute(&pool)
                .await?;

                let second =
                    record_for_actor(21, 121, 1, 0x34, now).map_err(foundation_error)?;
                let identity = second.identity();
                let same_account = ReconnectIdentityV1::new(
                    identity.game_session_id(),
                    identity.reconnect_attempt_ref(),
                    "123e4567-e89b-12d3-a456-426614174000",
                    identity.character_id(),
                    identity.world_id(),
                    identity.runtime_scope(),
                )
                .map_err(foundation_error)?;
                let second = ReconnectDurabilityRecordV1::new(
                    same_account,
                    second.connection(),
                    second.authority(),
                    second.continuity(),
                    second.proof().clone(),
                    second.fnd02().clone(),
                    second.compatibility().clone(),
                )
                .map_err(foundation_error)?;
                let (_, second) = ReconnectDurabilityFlowV1::begin(second);
                assert_eq!(
                    journal.prepare(&second).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );

                assert_eq!(
                    journal.prepare(&first).await?,
                    ReconnectPrepareDispositionV1::ExistingTerminal
                );

                let (_, changed) = ReconnectDurabilityFlowV1::begin(
                    record(20, 1, 0x35, now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&changed).await?,
                    ReconnectPrepareDispositionV1::IdempotencyConflict
                );
                // Reload persisted history through both compatibility entry points.
                let reloaded = durability::AdmissionReconnectJournalV2::connect_runtime(&url).await?;
                assert_eq!(
                    reloaded.legacy().prepare(&first).await?,
                    ReconnectPrepareDispositionV1::ExistingTerminal
                );
                assert_eq!(
                    reloaded.legacy().prepare(&changed).await?,
                    ReconnectPrepareDispositionV1::IdempotencyConflict
                );
                let (_, first_v2) = foundation::ReconnectDurabilityFlowV2::begin(first.record().clone(), None);
                let (_, changed_v2) = foundation::ReconnectDurabilityFlowV2::begin(changed.record().clone(), None);
                assert_eq!(
                    reloaded.prepare(&first_v2).await?,
                    foundation::ReconnectPrepareDispositionV2::ExistingTerminal {
                        disposition: foundation::ReconnectDurableTerminalDispositionV1::StaleAuthority,
                    }
                );
                assert_eq!(
                    reloaded.prepare(&changed_v2).await?,
                    foundation::ReconnectPrepareDispositionV2::IdempotencyConflict
                );
                let sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM game_durability_reconnect_sessions")
                    .fetch_one(&pool).await?;
                let memberships: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM game_durability_session_use_memberships")
                    .fetch_one(&pool).await?;
                assert_eq!((sessions, memberships), (2, 2));
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

#[test]
fn session_use_ledger_capacity_replay_and_sealed_reload() -> Result<(), Box<dyn std::error::Error>>
{
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        use foundation::admission_authority_publication::*;
        use foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1;
        use durability::fresh_admission::{FreshAdmissionStore, FreshReconciliation};
        for count in [65535_i64, 65536] {
            let database = postgres::IsolatedPostgres::create("session_use_capacity").await?;
            let result = async {
                let url = database.database_url()?;
                MigrationExecutor::connect_migration(&url).await?.apply_embedded_ledger().await?;
                let pool = sqlx::PgPool::connect(&url).await?;
                let now = postgres_clock(&pool).await?;
                let owner = postgres::fresh::Source::new(now)?;
                let character = owner.current.character_id;
                let guards = durability::admission_authority_guards::AdmissionGuardStore::connect_runtime(&url,8192).await?;
                guards.publish(&authority_matrix::checked(AdmissionAuthorityPublicationV1::prepare(&owner,now))?).await?;
                // Independent complete historical fixture, with every membership
                // represented explicitly; no current-session inference.
                sqlx::query("INSERT INTO game_durability_session_use_ledgers VALUES (encode($1,'hex')::uuid,1,TRUE,$2::text::numeric,$2::text::numeric)")
                    .bind(character.as_bytes().as_slice()).bind(count.to_string()).execute(&pool).await?;
                sqlx::query("INSERT INTO game_durability_session_use_memberships SELECT ('00000000-0000-7000-8000-' || lpad(to_hex(n + 100000),12,'0'))::uuid, encode($1,'hex')::uuid, n, decode(md5(n::text),'hex') FROM generate_series(1,$2::bigint) AS n")
                    .bind(character.as_bytes().as_slice()).bind(count).execute(&pool).await?;
                let store = FreshAdmissionStore::connect_runtime(&url,65536,8192).await?;
                let request = owner.request()?;
                let outcome = store.commit(&request).await;
                if count == 65536 {
                    assert!(matches!(outcome, Err(DurabilityError::AdmissionGameSessionLedgerExhausted)));
                    let sessions: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_reconnect_sessions").fetch_one(&pool).await?;
                    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_fresh_admission_receipts").fetch_one(&pool).await?;
                    assert_eq!((sessions,receipts),(0,0));
                } else {
                    assert!(matches!(outcome?,FreshAdmissionDurableOutcomeV1::Committed(_)));
                    assert!(matches!(store.commit(&request).await?,FreshAdmissionDurableOutcomeV1::ExistingCommitted(_)));
                    let FreshReconciliation::Committed(current) = store.reconcile(request.operation()).await? else { return Err("missing initial session".into()); };
                    let reloaded = FreshAdmissionStore::connect_runtime(&url,65536,8192).await?;
                    let lookup = GameSessionUseRequestV1::new_session(character,authority_matrix::session(900000)?,Some(current.current_session.current_game_session_id()),[9;16],65536,Some(GameSessionUseCurrentFenceV1::from_snapshot(current.current_session)));
                    let source = reloaded.session_use_source(lookup).await?;
                    let authority = GameSessionUseAuthorityV1::from_owning_source(&source);
                    assert_eq!(authority.authorize_terminal_replacement(lookup),Err(GameSessionUseAuthorizationErrorV1::TerminalReplacementGameSessionLedgerExhausted));
                    assert_eq!(authority.authorize_early_terminal_replacement(lookup),Err(GameSessionUseAuthorizationErrorV1::EarlyTerminalReplacementGameSessionLedgerExhausted));
                    assert_eq!(authority.authorize_post_grace_recovery(lookup),Err(GameSessionUseAuthorizationErrorV1::PostGraceRecoveryGameSessionLedgerExhausted));
                }
                let members: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_session_use_memberships").fetch_one(&pool).await?;
                assert_eq!(members,65536);
                pool.close().await;
                Ok::<(),Box<dyn std::error::Error>>(())
            }.await;
            database.cleanup().await?;
            result?;
        }
        Ok(())
    })
}

struct LifecycleOwner {
    current: foundation::GameSessionAuthoritySnapshot<foundation::AuthenticatedTransportRefV1>,
    transition: foundation::admission_authority_publication::AdmissionClaimTransitionEvidenceV1,
}
impl foundation::fnd04_verifier::fresh_source_sealed::Sealed for LifecycleOwner {}
impl foundation::admission_authority_publication::AdmissionClaimOwningSourceV1 for LifecycleOwner {
    fn prepare_fresh_claim(
        &self,
        _: &foundation::fresh_admission_durability::FreshAdmissionAuditBindingV1,
        _: i64,
    ) -> Result<
        foundation::admission_authority_publication::AdmissionClaimTransitionEvidenceV1,
        foundation::admission_authority_publication::AdmissionAuthorityPublicationErrorV1,
    > {
        Err(foundation::admission_authority_publication::AdmissionAuthorityPublicationErrorV1::Unavailable)
    }
    fn prepare_lifecycle_claim(
        &self,
        _: &foundation::admission_authority_publication::AdmissionClaimLifecycleOperationV1,
        _: i64,
    ) -> Result<
        foundation::admission_authority_publication::AdmissionClaimLifecycleResolutionV1,
        foundation::admission_authority_publication::AdmissionAuthorityPublicationErrorV1,
    > {
        Ok(
            foundation::admission_authority_publication::AdmissionClaimLifecycleResolutionV1 {
                current_session: self.current,
                evidence: self.transition.clone(),
            },
        )
    }
}

#[test]
fn terminal_release_claims_are_atomic_and_exactly_replayed_after_reload()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        use foundation::admission_authority_publication::*;
        use durability::fresh_admission::{FreshAdmissionStore, FreshReconciliation};
        let database = postgres::IsolatedPostgres::create("terminal_release_claims").await?;
        let result = async {
            let url = database.database_url()?;
            MigrationExecutor::connect_migration(&url).await?.apply_embedded_ledger().await?;
            let pool = sqlx::PgPool::connect(&url).await?;
            let now = postgres_clock(&pool).await?;
            let owner = postgres::fresh::Source::new(now)?;
            let guards = durability::admission_authority_guards::AdmissionGuardStore::connect_runtime(&url,8192).await?;
            guards.publish(&authority_matrix::checked(AdmissionAuthorityPublicationV1::prepare(&owner,now))?).await?;
            let store = FreshAdmissionStore::connect_runtime(&url,65536,8192).await?;
            let request = owner.request()?;
            store.commit(&request).await?;
            let FreshReconciliation::Committed(before) = store.reconcile(request.operation()).await? else { return Err("missing initial session".into()); };
            let predecessors = request.operation().transition.successors.clone();
            let mut successors = predecessors.clone();
            for row in &mut successors {
                row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet { expected_publication_revision: row.publication_revision };
                row.publication_revision += 1;
                row.source.source_revision += 1;
                row.source.decision_identity = "release-3".into();
                match &mut row.state {
                    AdmissionAuthorityGuardStateV1::Account { security, presence } => { *presence = None; security.provenance.publication_revision = row.publication_revision; }
                    AdmissionAuthorityGuardStateV1::Character { holder, .. } => *holder = None,
                    _ => return Err("unexpected lifecycle key".into()),
                }
            }
            let lifecycle_owner = LifecycleOwner { current: before.current_session, transition: AdmissionClaimTransitionEvidenceV1 { predecessors: predecessors.clone(), successors: successors.clone(), prepared_at: now } };
            let transition = authority_matrix::checked(TerminalReleaseClaimTransitionV1::prepare(&lifecycle_owner, &owner.current.account_id, before.current_session, now))?;
            assert_eq!(store.reconcile_lifecycle(transition.evidence()).await?,None);
            sqlx::query("CREATE FUNCTION reject_release_receipt() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'release rollback' USING ERRCODE = '23514'; END; $$").execute(&pool).await?;
            sqlx::query("CREATE TRIGGER reject_release_receipt BEFORE INSERT ON game_durability_admission_lifecycle_receipts FOR EACH ROW EXECUTE FUNCTION reject_release_receipt()").execute(&pool).await?;
            assert!(matches!(store.release(&transition).await,Err(DurabilityError::Database(_))));
            assert_eq!(store.reconcile(request.operation()).await?,FreshReconciliation::Committed(before.clone()));
            let keys: Vec<_> = predecessors.iter().map(|row|row.key.clone()).collect();
            assert_eq!(guards.load(&keys).await?,predecessors.into_iter().map(Some).collect::<Vec<_>>());
            sqlx::query("DROP TRIGGER reject_release_receipt ON game_durability_admission_lifecycle_receipts").execute(&pool).await?;
            let decided_at = store.release(&transition).await?;
            let reloaded = FreshAdmissionStore::connect_runtime(&url,65536,8192).await?;
            assert_eq!(reloaded.release(&transition).await?,decided_at);
            assert_eq!(reloaded.reconcile_lifecycle(transition.evidence()).await?,Some(decided_at));
            assert_eq!(guards.load(&keys).await?,successors.into_iter().map(Some).collect::<Vec<_>>());
            let FreshReconciliation::Committed(after) = store.reconcile(request.operation()).await? else { return Err("missing released session".into()); };
            assert_eq!(after.current_session.session_state(),foundation::GameSessionState::Terminal);
            assert_eq!(after.current_session.current_transport(),None);
            assert_eq!(after.current_session.current_character_lease(),before.current_session.current_character_lease());
            let members: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_session_use_memberships").fetch_one(&pool).await?;
            assert_eq!(members,1);
            pool.close().await;
            Ok::<(),Box<dyn std::error::Error>>(())
        }.await;
        database.cleanup().await?;
        result
    })
}

#[test]
fn session_use_source_rejects_incomplete_corrupt_and_globally_reused_membership()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        use foundation::admission_authority_publication::*;
        use durability::fresh_admission::{FreshAdmissionStore, FreshReconciliation};
        for scenario in 0..3 {
            let database = postgres::IsolatedPostgres::create("session_use_negative").await?;
            let result = async {
                let url = database.database_url()?;
                MigrationExecutor::connect_migration(&url).await?.apply_embedded_ledger().await?;
                let pool = sqlx::PgPool::connect(&url).await?;
                let now = postgres_clock(&pool).await?;
                let owner = postgres::fresh::Source::new(now)?;
                let guards = durability::admission_authority_guards::AdmissionGuardStore::connect_runtime(&url,8192).await?;
                guards.publish(&authority_matrix::checked(AdmissionAuthorityPublicationV1::prepare(&owner,now))?).await?;
                let store = FreshAdmissionStore::connect_runtime(&url,65536,8192).await?;
                let request = owner.request()?;
                store.commit(&request).await?;
                let FreshReconciliation::Committed(current) = store.reconcile(request.operation()).await? else { return Err("missing initial session".into()); };
                let candidate = authority_matrix::session(900001)?;
                let lookup = GameSessionUseRequestV1::new_session(owner.current.character_id,candidate,Some(current.current_session.current_game_session_id()),[9;16],1,Some(GameSessionUseCurrentFenceV1::from_snapshot(current.current_session)));
                assert!(GameSessionUseAuthorityV1::from_owning_source(&store.session_use_source(lookup).await?).authorize_terminal_replacement(lookup).is_ok());
                match scenario {
                    0 => { sqlx::query("UPDATE game_durability_session_use_ledgers SET complete = FALSE").execute(&pool).await?; }
                    1 => { sqlx::query("UPDATE game_durability_session_use_ledgers SET revision = 2, revision_floor = 2").execute(&pool).await?; }
                    _ => {
                        sqlx::query("INSERT INTO game_durability_session_use_ledgers VALUES (encode($1,'hex')::uuid,1,TRUE,1,1)")
                            .bind(authority_matrix::uuid(999).as_slice()).execute(&pool).await?;
                        sqlx::query("INSERT INTO game_durability_session_use_memberships VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,$3)")
                            .bind(candidate.as_bytes().as_slice()).bind(authority_matrix::uuid(999).as_slice()).bind([9_u8;16].as_slice()).execute(&pool).await?;
                    }
                }
                let reloaded = FreshAdmissionStore::connect_runtime(&url,65536,8192).await?;
                if scenario == 1 {
                    assert!(matches!(reloaded.session_use_source(lookup).await,Err(DurabilityError::InvalidStoredState)));
                } else {
                    let source = reloaded.session_use_source(lookup).await?;
                    let expected = if scenario == 0 { GameSessionUseAuthorizationErrorV1::StaleAuthority } else { GameSessionUseAuthorizationErrorV1::CandidateAlreadyUsed };
                    assert_eq!(GameSessionUseAuthorityV1::from_owning_source(&source).authorize_terminal_replacement(lookup),Err(expected));
                }
                pool.close().await;
                Ok::<(),Box<dyn std::error::Error>>(())
            }.await;
            database.cleanup().await?;
            result?;
        }
        Ok(())
    })
}

#[test]
fn session_use_migration_preserves_known_history_as_incomplete()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let database = postgres::IsolatedPostgres::create("session_use_migration").await?;
        let result = async {
            let url = database.database_url()?;
            let pool = sqlx::PgPool::connect(&url).await?;
            sqlx::raw_sql(include_str!("../migrations/0001_admission_reconnect_journal.sql")).execute(&pool).await?;
            sqlx::query("INSERT INTO game_durability_reconnect_sessions (game_session_id,account_id,character_id,world_id,runtime_scope_kind,runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,original_grace_deadline,predecessor_generation,character_lease_generation,scope_ownership_generation,current_generation) VALUES (encode($1,'hex')::uuid,$2::text::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($4,'hex')::uuid,encode($5,'hex')::uuid,3,1000,7,9,10,7)")
                .bind(authority_matrix::uuid(20).as_slice()).bind(authority_matrix::ACCOUNT).bind(authority_matrix::uuid(11).as_slice()).bind(authority_matrix::uuid(12).as_slice()).bind(authority_matrix::uuid(13).as_slice()).execute(&pool).await?;
            sqlx::raw_sql(include_str!("../migrations/0002_fresh_admission_authority.sql")).execute(&pool).await?;
            let state: (bool,String,String) = sqlx::query_as("SELECT complete, revision::text, revision_floor::text FROM game_durability_session_use_ledgers").fetch_one(&pool).await?;
            assert_eq!(state,(false,"1".into(),"1".into()));
            let ids: Vec<Vec<u8>> = sqlx::query_scalar("SELECT uuid_send(game_session_id) FROM game_durability_session_use_memberships").fetch_all(&pool).await?;
            assert_eq!(ids,vec![authority_matrix::uuid(20).to_vec()]);
            pool.close().await;
            Ok::<(),Box<dyn std::error::Error>>(())
        }.await;
        database.cleanup().await?;
        result
    })
}

#[test]
fn lawful_legacy_claim_replacement_persists_lineage_and_releases_successor()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        use foundation::admission_authority_publication::*;
        use foundation::*;
        use durability::fresh_admission::FreshAdmissionStore;
        use authority_matrix::checked;
        let database = postgres::IsolatedPostgres::create("legacy_claim_replacement").await?;
        let result = async {
            let url = database.database_url()?;
            MigrationExecutor::connect_migration(&url).await?.apply_embedded_ledger().await?;
            let pool = sqlx::PgPool::connect(&url).await?;
            let now = postgres_clock(&pool).await?;
            let owner = postgres::fresh::Source::new(now)?;
            let guards = durability::admission_authority_guards::AdmissionGuardStore::connect_runtime(&url,8192).await?;
            guards.publish(&checked(AdmissionAuthorityPublicationV1::prepare(&owner,now))?).await?;
            let store = FreshAdmissionStore::connect_runtime(&url,65536,8192).await?;
            let fresh = owner.request()?;
            store.commit(&fresh).await?;
            let original_id = fresh.operation().authorization.candidate_session;
            // Independent canonical legacy-loss fixture. This is deliberately not
            // proof of an owning-fresh-loss bridge: that bridge remains unavailable.
            sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=1,current_transport_ref=NULL,control_loss_epoch=1,original_grace_deadline=$1,predecessor_generation=1")
                .bind(now+120).execute(&pool).await?;
            sqlx::query("INSERT INTO game_durability_control_loss_continuity (character_id,control_loss_epoch,account_id,world_id,context_game_session_id,original_grace_deadline,protection_entitlement_state,protection_rearm_state) VALUES (encode($1,'hex')::uuid,1,$2::text::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,1,1)")
                .bind(owner.current.character_id.as_bytes().as_slice()).bind(&owner.current.account_id).bind(owner.current.world_id.as_bytes().as_slice()).bind(original_id.as_bytes().as_slice()).bind(now+120).execute(&pool).await?;
            let before = store.current_session(original_id).await?;
            let terminal = checked(GameSessionAuthoritySnapshot::from_persisted_current_facts(original_id,before.commit(),GameSessionState::Terminal,before.current_connection_generation(),None,before.current_character_lease(),before.current_character_world_eligibility(),before.current_runtime_scope(),before.current_scope_generation()))?;
            let terminal = checked(terminal.with_control_loss_continuity(checked(ControlLossEpochRefV1::new(1))?,now+120))?;
            let template = authority_matrix::prepared_record(authority_matrix::Seed { now,generation:1,epoch:1,transport:44,..authority_matrix::Seed::fixed() })?;
            let candidate_id = authority_matrix::session(900100)?;
            let record = checked(ReconnectDurabilityRecordV1::new(
                checked(ReconnectIdentityV1::new(candidate_id,template.identity().reconnect_attempt_ref(),&owner.current.account_id,owner.current.character_id,owner.current.world_id,before.current_runtime_scope()))?,
                template.connection(),checked(ReconnectAuthorityFenceV1::new(before.current_character_lease().generation(),before.current_scope_generation()))?,template.continuity(),template.proof().clone(),template.fnd02().clone(),template.compatibility().clone()))?;
            let presence = checked(AccountPresenceClaimV1::new(&owner.current.account_id,owner.current.character_id))?;
            let authorization = checked(TerminalGameSessionReplacementAuthorizationV1::from_current_authority(&owner.current.account_id,Some(&presence),original_id,candidate_id,terminal,&record))?;
            let predecessors = fresh.operation().transition.successors.clone();
            let mut successors = predecessors.clone();
            for row in &mut successors {
                row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet { expected_publication_revision:row.publication_revision };
                row.publication_revision += 1; row.source.source_revision += 1; row.source.decision_identity="replacement-3".into();
                match &mut row.state {
                    AdmissionAuthorityGuardStateV1::Account { security,presence } => { security.provenance.publication_revision=row.publication_revision; *presence=Some((owner.current.character_id,candidate_id)); }
                    AdmissionAuthorityGuardStateV1::Character { holder,.. } => *holder=Some(candidate_id),
                    _ => return Err("unexpected claim".into()),
                }
            }
            let lifecycle_owner=LifecycleOwner { current:terminal,transition:AdmissionClaimTransitionEvidenceV1 { predecessors,successors:successors.clone(),prepared_at:now } };
            let claims=checked(TerminalReplacementClaimTransitionV1::prepare(&lifecycle_owner,&authorization,terminal,&record,now))?;
            let (_,request)=ReconnectDurabilityFlowV2::begin(record.clone(),Some(authorization));
            let lookup=GameSessionUseRequestV1::new_session(owner.current.character_id,candidate_id,Some(original_id),record.connection().transport_ref().to_bytes(),1,Some(GameSessionUseCurrentFenceV1::from_snapshot(before)));
            let journal=durability::AdmissionReconnectJournalV2::connect_runtime(&url).await?;
            assert!(matches!(journal.prepare(&request).await,Err(DurabilityError::Unavailable)));
            sqlx::query("CREATE FUNCTION reject_replacement_receipt() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'replacement rollback' USING ERRCODE = '23514'; END; $$").execute(&pool).await?;
            sqlx::query("CREATE TRIGGER reject_replacement_receipt BEFORE INSERT ON game_durability_admission_lifecycle_receipts FOR EACH ROW EXECUTE FUNCTION reject_replacement_receipt()").execute(&pool).await?;
            assert!(matches!(journal.prepare_with_claims(&request,&claims,lookup).await,Err(DurabilityError::Database(_))));
            assert_eq!(store.current_session(original_id).await?,before);
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_durability_session_use_memberships").fetch_one(&pool).await?,1);
            assert_eq!(store.reconcile_lifecycle(claims.evidence()).await?,None);
            sqlx::query("DROP TRIGGER reject_replacement_receipt ON game_durability_admission_lifecycle_receipts").execute(&pool).await?;
            assert_eq!(journal.prepare_with_claims(&request,&claims,lookup).await?,ReconnectPrepareDispositionV2::Prepared);
            let decided_at=store.reconcile_lifecycle(claims.evidence()).await?.ok_or("missing lifecycle receipt")?;
            assert_eq!(journal.prepare_with_claims(&request,&claims,lookup).await?,ReconnectPrepareDispositionV2::ExistingPrepared);
            assert_eq!(store.reconcile_lifecycle(claims.evidence()).await?,Some(decided_at));
            let reloaded=FreshAdmissionStore::connect_runtime(&url,65536,8192).await?;
            let first_current=reloaded.current_session(candidate_id).await?;
            assert_eq!(first_current.current_game_session_id(),candidate_id);
            assert_eq!(first_current.commit().game_session_id(),original_id);
            // A second replacement must preserve the initial binding and retain
            // the intermediate ID permanently, including after another reload.
            let mut stale_release_rows=successors.clone();
            for row in &mut stale_release_rows {
                row.precondition=AdmissionPublicationPreconditionV1::CompareAndSet {expected_publication_revision:row.publication_revision};
                row.publication_revision+=1;row.source.source_revision+=1;row.source.decision_identity="stale-release-4".into();
                match &mut row.state {
                    AdmissionAuthorityGuardStateV1::Account {security,presence}=>{security.provenance.publication_revision=row.publication_revision;*presence=None;}
                    AdmissionAuthorityGuardStateV1::Character {holder,..}=>*holder=None,
                    _=>return Err("unexpected stale release key".into()),
                }
            }
            let stale_owner=LifecycleOwner {current:first_current,transition:AdmissionClaimTransitionEvidenceV1 {predecessors:successors.clone(),successors:stale_release_rows,prepared_at:now}};
            let stale_release=checked(TerminalReleaseClaimTransitionV1::prepare(&stale_owner,&owner.current.account_id,first_current,now))?;
            let next_id=authority_matrix::session(900101)?;
            let next_identity=checked(ReconnectIdentityV1::new(next_id,checked(ReconnectAttemptRef::new(2))?,&owner.current.account_id,owner.current.character_id,owner.current.world_id,first_current.current_runtime_scope()))?;
            let next_connection=checked(ReconnectConnectionFenceV1::new(first_current.current_connection_generation(),checked(ConnectionGeneration::new(2))?,authority_matrix::transport(45)?))?;
            let next_record=checked(ReconnectDurabilityRecordV1::new(next_identity,next_connection,record.authority(),record.continuity(),record.proof().clone(),record.fnd02().clone(),record.compatibility().clone()))?;
            let next_terminal=checked(GameSessionAuthoritySnapshot::from_persisted_current_facts(candidate_id,first_current.commit(),GameSessionState::Terminal,first_current.current_connection_generation(),None,first_current.current_character_lease(),first_current.current_character_world_eligibility(),first_current.current_runtime_scope(),first_current.current_scope_generation()))?;
            let next_terminal=checked(next_terminal.with_control_loss_continuity(checked(ControlLossEpochRefV1::new(1))?,now+120))?;
            let next_authorization=checked(TerminalGameSessionReplacementAuthorizationV1::from_current_authority(&owner.current.account_id,Some(&presence),candidate_id,next_id,next_terminal,&next_record))?;
            let mut next_successors=successors.clone();
            for row in &mut next_successors {
                row.precondition=AdmissionPublicationPreconditionV1::CompareAndSet { expected_publication_revision:row.publication_revision };
                row.publication_revision+=1;row.source.source_revision+=1;row.source.decision_identity="replacement-4".into();
                match &mut row.state {
                    AdmissionAuthorityGuardStateV1::Account {security,presence}=>{security.provenance.publication_revision=row.publication_revision;*presence=Some((owner.current.character_id,next_id));}
                    AdmissionAuthorityGuardStateV1::Character {holder,..}=>*holder=Some(next_id),
                    _=>return Err("unexpected successor key".into()),
                }
            }
            let next_owner=LifecycleOwner { current:next_terminal,transition:AdmissionClaimTransitionEvidenceV1 {predecessors:successors.clone(),successors:next_successors.clone(),prepared_at:now}};
            let next_claims=checked(TerminalReplacementClaimTransitionV1::prepare(&next_owner,&next_authorization,next_terminal,&next_record,now))?;
            let (_,next_request)=ReconnectDurabilityFlowV2::begin(next_record.clone(),Some(next_authorization));
            let next_lookup=GameSessionUseRequestV1::new_session(owner.current.character_id,next_id,Some(candidate_id),next_record.connection().transport_ref().to_bytes(),2,Some(GameSessionUseCurrentFenceV1::from_snapshot(first_current)));
            assert_eq!(journal.prepare_with_claims(&next_request,&next_claims,next_lookup).await?,ReconnectPrepareDispositionV2::Prepared);
            assert!(reloaded.release(&stale_release).await.is_err());
            let successor_keys:Vec<_>=next_successors.iter().map(|row|row.key.clone()).collect();
            assert_eq!(guards.load(&successor_keys).await?,next_successors.iter().cloned().map(Some).collect::<Vec<_>>());
            let current=reloaded.current_session(next_id).await?;
            assert_eq!(current.commit().game_session_id(),original_id);
            let retired=GameSessionUseRequestV1::new_session(owner.current.character_id,candidate_id,Some(next_id),[99;16],3,Some(GameSessionUseCurrentFenceV1::from_snapshot(current)));
            let retired_source=reloaded.session_use_source(retired).await?;
            let retired_authority=GameSessionUseAuthorityV1::from_owning_source(&retired_source);
            assert_eq!(retired_authority.authorize_terminal_replacement(retired),Err(GameSessionUseAuthorizationErrorV1::CandidateAlreadyUsed));
            assert_eq!(retired_authority.authorize_early_terminal_replacement(retired),Err(GameSessionUseAuthorizationErrorV1::CandidateAlreadyUsed));
            assert_eq!(retired_authority.authorize_post_grace_recovery(retired),Err(GameSessionUseAuthorizationErrorV1::CandidateAlreadyUsed));
            let successors=next_successors;
            let candidate_id=next_id;
            let mut released=successors.clone();
            for row in &mut released {
                row.precondition=AdmissionPublicationPreconditionV1::CompareAndSet { expected_publication_revision:row.publication_revision };
                row.publication_revision+=1;row.source.source_revision+=1;row.source.decision_identity="release-5".into();
                match &mut row.state {
                    AdmissionAuthorityGuardStateV1::Account { security,presence } => {security.provenance.publication_revision=row.publication_revision;*presence=None;}
                    AdmissionAuthorityGuardStateV1::Character {holder,..} => *holder=None,
                    _=>return Err("unexpected release key".into()),
                }
            }
            let release_owner=LifecycleOwner {current,transition:AdmissionClaimTransitionEvidenceV1 {predecessors:successors,successors:released.clone(),prepared_at:now}};
            let release=checked(TerminalReleaseClaimTransitionV1::prepare(&release_owner,&owner.current.account_id,current,now))?;
            reloaded.release(&release).await?;
            assert_eq!(reloaded.current_session(candidate_id).await?.session_state(),GameSessionState::Terminal);
            let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_durability_session_use_memberships").fetch_one(&pool).await?;
            assert_eq!(count,3);
            pool.close().await;
            Ok::<(),Box<dyn std::error::Error>>(())
        }.await;
        database.cleanup().await?;
        result
    })
}

#[test]
fn concurrent_fresh_replay_commits_exactly_one_membership() -> Result<(), Box<dyn std::error::Error>>
{
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        use foundation::admission_authority_publication::*;
        use foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1;
        use durability::fresh_admission::FreshAdmissionStore;
        let database = postgres::IsolatedPostgres::create("concurrent_membership").await?;
        let result = async {
            let url = database.database_url()?;
            MigrationExecutor::connect_migration(&url).await?.apply_embedded_ledger().await?;
            let pool = sqlx::PgPool::connect(&url).await?;
            let owner=postgres::fresh::Source::new(postgres_clock(&pool).await?)?;
            let guards=durability::admission_authority_guards::AdmissionGuardStore::connect_runtime(&url,8192).await?;
            guards.publish(&authority_matrix::checked(AdmissionAuthorityPublicationV1::prepare(&owner,owner.now))?).await?;
            let first=FreshAdmissionStore::connect_runtime(&url,65536,8192).await?;
            let second=FreshAdmissionStore::connect_runtime(&url,65536,8192).await?;
            let request=owner.request()?;
            let second_request=request.clone();
            let first_task=tokio::spawn(async move { first.commit(&request).await });
            let b=second.commit(&second_request).await;
            let a=first_task.await?;
            assert!(matches!((a?,b?), (FreshAdmissionDurableOutcomeV1::Committed(_),FreshAdmissionDurableOutcomeV1::ExistingCommitted(_)) | (FreshAdmissionDurableOutcomeV1::ExistingCommitted(_),FreshAdmissionDurableOutcomeV1::Committed(_))));
            let (count,revision): (i64,String)=sqlx::query_as("SELECT (SELECT count(*) FROM game_durability_session_use_memberships),revision::text FROM game_durability_session_use_ledgers").fetch_one(&pool).await?;
            assert_eq!((count,revision),(1,"1".into()));
            assert!(sqlx::query("DELETE FROM game_durability_session_use_memberships").execute(&pool).await.is_err());
            assert!(sqlx::query("DELETE FROM game_durability_session_use_ledgers").execute(&pool).await.is_err());
            assert!(sqlx::query("UPDATE game_durability_session_use_ledgers SET revision=0,revision_floor=0").execute(&pool).await.is_err());
            pool.close().await;
            Ok::<(),Box<dyn std::error::Error>>(())
        }.await;
        database.cleanup().await?;
        result
    })
}

/// NATIVE_ENTRY_CONTENT_ACTIVATION_V1 (#935): the per-scope activation floor is monotonic and
/// predecessor-bound, exact replays succeed, conflicts refuse, rows are immutable, and only a
/// scope granted operation 4 may issue.
#[test]
fn content_activation_floor_is_monotonic_predecessor_bound_and_immutable()
-> Result<(), Box<dyn std::error::Error>> {
    use durability::content_activation::ContentActivationRequest;
    use foundation::{ChannelId, WorldId};
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("content_activation_floor").await?;
            let result = async {
                let url = database.database_url()?;
                MigrationExecutor::connect_migration(&url)
                    .await?
                    .apply_embedded_ledger()
                    .await?;
                let world = WorldId::decode(&authority_matrix::uuid(60))?;
                let channel = ChannelId::decode(&authority_matrix::uuid(61))?;
                let other_channel = ChannelId::decode(&authority_matrix::uuid(62))?;
                let root = durability::DurabilityRoot::connect_test_runtime(&url)?;
                assert!(root.maintain_ready_once().await?);
                let request = |sequence, previous, digest: u8| ContentActivationRequest {
                    world_id: world,
                    channel_id: channel,
                    activation_sequence: sequence,
                    previous_sequence: previous,
                    server_artifact_digest: [digest; 32],
                    client_artifact_digest: [digest.wrapping_add(1); 32],
                    frame_binding_digest: [digest.wrapping_add(2); 32],
                };

                // Without an operation-4 grant for this scope the issuance is refused.
                assert!(!root.record_content_activation(&request(1, None, 1)).await?);
                let mut connection = sqlx::PgConnection::connect(&url).await?;
                sqlx::query(
                    "INSERT INTO game_control_scope_grants (control_role, world_id, channel_id, operation) \
                     VALUES (session_user, encode($1, 'hex')::uuid, encode($2, 'hex')::uuid, 4)",
                )
                .bind(world.as_bytes().as_slice())
                .bind(channel.as_bytes().as_slice())
                .execute(&mut connection)
                .await?;
                assert_eq!(root.read_current_content_activation(world, channel).await?, None);
                // An operation-4 grant never authorizes an assignment receipt of kind 4.
                let receipt = sqlx::query(
                    "INSERT INTO game_runtime_scope_assignment_receipts (operation_key, command, \
                     scope_key, ownership_generation, state, holder_node_id, \
                     holder_registration_revision, source_revision, decision_identity, decided_at) \
                     VALUES (decode(repeat('00', 32), 'hex'), \
                     '\\x0104'::bytea || decode(repeat('00', 32), 'hex') \
                       || set_byte('\\x00'::bytea, 0, octet_length(session_user)) \
                       || convert_to(session_user, 'UTF8'), \
                     '\\x01'::bytea || $1 || $2, 1, 1, encode($1, 'hex')::uuid, 1, 99, \
                     'runtime-scope-assignment:99', 0)",
                )
                .bind(world.as_bytes().as_slice())
                .bind(channel.as_bytes().as_slice())
                .execute(&mut connection)
                .await;
                let code = match &receipt {
                    Err(sqlx::Error::Database(error)) => error.code().map(|code| code.into_owned()),
                    _ => None,
                };
                assert_eq!(code.as_deref(), Some("42501"), "kind-4 receipt: {receipt:?}");

                assert!(root.record_content_activation(&request(1, None, 1)).await?);
                assert!(root.record_content_activation(&request(1, None, 1)).await?, "exact replay");
                assert!(
                    !root.record_content_activation(&request(1, None, 9)).await?,
                    "conflicting replay"
                );
                assert!(
                    !root.record_content_activation(&request(2, None, 2)).await?,
                    "a second empty start is stale"
                );
                assert!(root.record_content_activation(&request(3, Some(1), 3)).await?);
                assert!(
                    !root.record_content_activation(&request(4, Some(1), 4)).await?,
                    "predecessor is no longer current"
                );
                assert!(
                    root.record_content_activation(&request(3, Some(3), 3)).await.is_err(),
                    "a predecessor not below the sequence is refused before the database"
                );
                let current = root
                    .read_current_content_activation(world, channel)
                    .await?
                    .ok_or("current activation")?;
                assert_eq!(current.activation_sequence, 3);
                assert_eq!(current.server_artifact_digest, [3; 32]);
                assert_eq!(current.client_artifact_digest, [4; 32]);
                assert_eq!(current.frame_binding_digest, [5; 32]);
                assert_eq!(
                    root.read_current_content_activation(world, other_channel).await?,
                    None,
                    "the floor is per Channel scope"
                );
                for statement in [
                    "UPDATE game_content_activations SET issued_at = issued_at + 1",
                    "DELETE FROM game_content_activations",
                ] {
                    assert!(sqlx::query(statement).execute(&mut connection).await.is_err());
                }
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn complete_reconnect_resumes_an_owning_loss_session_exactly_once()
-> Result<(), Box<dyn std::error::Error>> {
    use base64::Engine;
    use durability::admission_authority_guards::AdmissionGuardStore;
    use durability::fresh_admission::{FreshAdmissionStore, FreshReconciliation};
    use ed25519_dalek::{Signer, SigningKey};
    use foundation::admission_authority_publication::*;
    use foundation::fnd04_verifier::*;
    use foundation::*;
    struct LossSource(ControlLossObservationV1);
    impl foundation::fnd04_verifier::recovery_source_sealed::Sealed for LossSource {}
    impl ControlLossSourceV1 for LossSource {
        fn resolve_loss(
            &self,
            _: GameSessionId,
            _: i64,
        ) -> Result<ControlLossObservationV1, ReconnectDurabilityErrorV1> {
            Ok(self.0.clone())
        }
    }
    #[derive(Clone)]
    struct Security {
        signing: RecoverySigningTrustObservationV2,
        security: RecoveryAccountSecurityObservationV2,
    }
    impl foundation::fnd04_verifier::recovery_source_sealed::Sealed for Security {}
    impl RecoveryDurabilityEvidenceSourceV2 for Security {
        fn signing_trust(
            &self,
            _: &str,
            _: i64,
        ) -> Result<RecoverySigningTrustObservationV2, Fnd04EvidenceError> {
            Ok(self.signing.clone())
        }
        fn account_security(
            &self,
            _: &str,
            _: i64,
        ) -> Result<RecoveryAccountSecurityObservationV2, Fnd04EvidenceError> {
            Ok(self.security.clone())
        }
    }
    struct Owner {
        current: std::sync::Mutex<CompleteReconnectCurrentV1>,
        security: Security,
    }
    impl foundation::fnd04_verifier::recovery_source_sealed::Sealed for Owner {}
    impl CompleteReconnectSourceV1 for Owner {
        fn resolve_reconnect(
            &self,
            _: &ReconnectIdentityV1,
            _: i64,
        ) -> Result<CompleteReconnectCurrentV1, ReconnectDurabilityErrorV1> {
            Ok(match self.current.lock() {
                Ok(current) => current.clone(),
                Err(poisoned) => poisoned.into_inner().clone(),
            })
        }
        fn recovery_v2_source(&self) -> Option<&dyn RecoveryDurabilityEvidenceSourceV2> {
            Some(&self.security)
        }
    }
    struct Report(Option<CompleteReconnectCompletionV1>);
    impl foundation::fnd04_verifier::recovery_source_sealed::Sealed for Report {}
    impl CompleteReconnectCompletionSourceV1 for Report {
        fn take_complete_reconnect_completion(
            &mut self,
            _: &CompleteReconnectDurabilityOperationV1,
        ) -> Result<Option<CompleteReconnectCompletionV1>, ReconnectDurabilityErrorV1> {
            Ok(self.0.take())
        }
    }
    fn uuid(bytes: &[u8; 16]) -> String {
        let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        format!(
            "{}-{}-{}-{}-{}",
            &hex[..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..]
        )
    }
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    // The whole reconnect scenario is one large future; keep it off the test-thread stack.
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(Box::pin(async {
        let database = postgres::IsolatedPostgres::create("complete_reconnect").await?;
        let result = async {
            let url = database.database_url()?;
            MigrationExecutor::connect_migration(&url).await?.apply_embedded_ledger().await?;
            let pool = sqlx::PgPool::connect(&url).await?;
            let now = postgres_clock(&pool).await?;
            let owner = postgres::fresh::Source::new(now)?;
            let guards = AdmissionGuardStore::connect_runtime(&url, 8192).await?;
            guards.publish(&authority_matrix::checked(AdmissionAuthorityPublicationV1::prepare(&owner, now))?).await?;
            let store = FreshAdmissionStore::connect_runtime(&url, 65536, 8192).await?;
            let request = owner.request()?;
            store.commit(&request).await?;
            let FreshReconciliation::Committed(initial) = store.reconcile(request.operation()).await? else { return Err("missing fresh session".into()); };
            let session = initial.current_session;
            let account = "00000000-0000-4000-8000-000000000001";
            let presence = authority_matrix::checked(AccountPresenceClaimV1::new(account, session.commit().character_id()))?;
            let protection = RecoveryProtectionContinuityV1 {
                usage: RecoveryProtectionUseV1::Unused { entitlement_generation: 1 },
                rearm: RecoveryProtectionRearmV1::Satisfied { generation: 1, established_at: now },
            };
            let loss_source = std::sync::Arc::new(LossSource(ControlLossObservationV1 {
                source_authority: session.current_runtime_scope(), source_revision: 1, accepted_source_revision: 1,
                decision_identity: authority_matrix::checked(ControlLossEpochRefV1::new(1))?,
                accepted_decision_identity: authority_matrix::checked(ControlLossEpochRefV1::new(1))?,
                observed_at: now, session, account_presence: presence.clone(),
                placement_identity: [9; 16], placement_revision: 1, actor_present: true, runtime_ready: true,
                cause: ControlLossCauseV1::AuthoritativeUnexpectedLoss,
                loss_epoch: authority_matrix::checked(ControlLossEpochRefV1::new(1))?, loss_origin: now,
                original_grace_deadline: now + 120, history: ControlLossHistoryV1::FreshOrigin, protection,
            }));
            let authorization = authority_matrix::checked(ControlLossAuthorizationV1::authorize(loss_source.as_ref(), session.commit().game_session_id(), now))?;
            let mut loss_flow = ControlLossFlowV1::begin(authorization);
            let loss = std::sync::Arc::new(authority_matrix::checked(loss_flow.take_request())?);
            let ControlLossOutcomeV1::Committed { decided_at: loss_decided_at } = store.commit_fresh_loss(loss.clone(), loss_source.clone()).await? else { return Err("loss not committed".into()); };
            let (lost, now) = store.current_session_at(session.commit().game_session_id()).await?;
            assert_eq!(lost.session_state(), GameSessionState::Reconnectable);

            // A Platform recovery credential for exactly this account, character and world.
            let key = SigningKey::from_bytes(&[23; 32]);
            let character = uuid(session.commit().character_id().as_bytes());
            let world = uuid(session.commit().world_id().as_bytes());
            let nonce = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([8; 32]);
            let header = r#"{"alg":"Ed25519","kid":"recovery-1","typ":"oteryn-recovery+jwt"}"#;
            let payload = format!(
                r#"{{"iss":"urn:oteryn:platform:game-recovery","aud":"urn:oteryn:game:recovery","iat":{now},"nbf":{now},"exp":{},"jti":"{nonce}","profile":"oteryn-reauth-recovery-v1","purpose":"existing_actor_recovery","attempt_ref":"00000000-0000-7000-8000-000000000001","account_id":"{account}","character_id":"{character}","world_id":"{world}","account_security_generation":"1","protocol_major":1,"transport_profile":1,"ruleset_revision":"rules-1","content_revision":"content-1","map_revision":"map-1","world_policy_revision":"policy-1"}}"#,
                now + 30
            );
            let input = format!("{}.{}", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header), base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload));
            let token = format!("{input}.{}", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key.sign(input.as_bytes()).to_bytes()));
            let provenance = |purpose| FreshEvidenceProvenanceV1 {
                source_authority: "independent-platform-fixture".into(), purpose, scope: Fnd04EvidenceScope::ExistingActorRecovery,
                source_revision: 1, accepted_source_revision: 1, decision_identity: "recovery-1".into(), accepted_decision_identity: "recovery-1".into(),
                source_observed_at: now, clock_uncertainty_seconds: 0, publication_revision: 1,
            };
            let security = Security {
                signing: RecoverySigningTrustObservationV2 { key_id: "recovery-1".into(), public_key: key.verifying_key().to_bytes(), trusted: true, provenance: provenance(FreshEvidencePurposeV1::SigningTrust) },
                security: RecoveryAccountSecurityObservationV2 { account_id: account.into(), minimum_generation: 1, allowed: true, provenance: provenance(FreshEvidencePurposeV1::PlatformSecurity) },
            };
            let recovery = RecoveryCurrentEvidence {
                account_id: account.into(), character_id: session.commit().character_id(), world_id: session.commit().world_id(),
                ruleset_revision: "rules-1".into(), content_revision: "content-1".into(), map_revision: "map-1".into(), world_policy_revision: "policy-1".into(),
            };
            let identity = authority_matrix::checked(ReconnectIdentityV1::new(
                session.commit().game_session_id(), authority_matrix::checked(ReconnectAttemptRef::new(20))?, account,
                session.commit().character_id(), session.commit().world_id(), session.current_runtime_scope(),
            ))?;
            let candidate = authority_matrix::checked(ReconnectCandidateBindingV1::new(
                session.commit().game_session_id(), identity.reconnect_attempt_ref(),
                authority_matrix::checked(ConnectionGeneration::new(2))?,
                authority_matrix::checked(AuthenticatedTransportRefV1::decode(&[0x61; 16]))?, now + 5,
            ))?;
            let keys = vec![
                AdmissionAuthorityGuardKeyV1::Account { account_id: account.into() },
                AdmissionAuthorityGuardKeyV1::Character(session.commit().character_id()),
            ];
            let claims = guards.load(&keys).await?.into_iter().collect::<Option<Vec<_>>>().ok_or("missing claim rows")?;
            let snapshot = CompleteReconnectSnapshotV1 {
                replacement_anchor: None, predecessor_attempts: vec![],
                loss: loss.operation().clone(), loss_decided_at,
                source_authority: lost.current_runtime_scope(), source_revision: 1, accepted_source_revision: 1, observed_at: now,
                session: lost, account_presence: presence, actor_present: true, runtime_ready: true,
                placement_identity: [9; 16], placement_revision: 1, protection,
                budget: store.recovery_budget(lost.commit().game_session_id(), authority_matrix::checked(ControlLossEpochRefV1::new(1))?).await?,
                candidate,
                proof_transition: CompleteReconnectProofTransitionV1 {
                    owner: lost.current_runtime_scope(), revision: 1, accepted_revision: 1, observed_at: now,
                    predecessor_session: lost.commit().game_session_id(), predecessor_generation: 1,
                    successor_session: lost.commit().game_session_id(), successor_generation: 2, candidate,
                },
                fnd02: authority_matrix::checked(Fnd02ReconciliationFenceV1::new(authority_matrix::checked(CommandId::new(1))?, vec![], 1, vec![]))?,
                recovery: recovery.clone(), claims,
            };
            let source = std::sync::Arc::new(Owner {
                current: std::sync::Mutex::new(CompleteReconnectCurrentV1 { snapshot, prepared: None }),
                security: security.clone(),
            });
            {
                // Recovery grants control: claims the source read before an owner refresh
                // (presence and holder still this session) no longer authorize it.
                let stale = source.current.lock().map_err(|_| "owner lock")?.clone();
                let mut row = guards.load(&[keys[0].clone()]).await?.pop().flatten().ok_or("missing account row")?;
                row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet { expected_publication_revision: row.publication_revision };
                row.publication_revision += 1;
                row.source.source_revision += 1;
                row.source.decision_identity = "platform-refresh".into();
                if let AdmissionAuthorityGuardStateV1::Account { security, .. } = &mut row.state {
                    security.provenance.publication_revision = row.publication_revision;
                }
                let mut refresher = postgres::fresh::Source::new(now)?;
                refresher.rows = vec![row];
                assert_eq!(guards.publish(&authority_matrix::checked(AdmissionAuthorityPublicationV1::prepare(&refresher, now))?).await?, durability::admission_authority_guards::GuardPublicationDisposition::Applied);
                let stale = std::sync::Arc::new(Owner { current: std::sync::Mutex::new(stale), security: security.clone() });
                let verified = verify_recovery_grant_durability_v2(&token, now, &RecoveryDurabilityTrustContextV2::from_owning_source(&security), &recovery)
                    .map_err(|error| format!("verify: {error:?}"))?;
                let authorization = CompleteReconnectAuthorizationV1::authorize(stale.as_ref(), identity.clone(), CompleteReconnectProofV1::V2(Box::new(verified)), now)
                    .map_err(|e| format!("stale authorize: {e:?}"))?;
                let mut stale_flow = CompleteReconnectFlowV1::begin(authorization, None).map_err(|e| format!("stale begin: {e:?}"))?;
                let request = std::sync::Arc::new(stale_flow.take_request(CompleteReconnectRequestKindV1::Prepare).map_err(|e| format!("stale take: {e:?}"))?);
                assert_eq!(store.apply_complete_reconnect(request, stale).await?, CompleteReconnectOutcomeV1::Rejected);
                source.current.lock().map_err(|_| "owner lock")?.snapshot.claims =
                    guards.load(&keys).await?.into_iter().collect::<Option<Vec<_>>>().ok_or("missing claim rows")?;
            }
            // The durable original loss is read back exactly.
            assert_eq!(
                store.owning_loss(lost.commit().game_session_id(), authority_matrix::checked(ControlLossEpochRefV1::new(1))?).await?,
                Some((loss.operation().clone(), loss_decided_at))
            );
            {
                // A PREPARE whose COMMIT is abandoned is withdrawn: the session accepts a
                // later attempt and the budget keeps the withdrawn one as Terminal.
                let abandoned = source.current.lock().map_err(|_| "owner lock")?.clone();
                let abandoned = std::sync::Arc::new(Owner { current: std::sync::Mutex::new(abandoned), security: security.clone() });
                let identity19 = authority_matrix::checked(ReconnectIdentityV1::new(
                    lost.commit().game_session_id(), authority_matrix::checked(ReconnectAttemptRef::new(19))?, account,
                    lost.commit().character_id(), lost.commit().world_id(), lost.current_runtime_scope(),
                ))?;
                let candidate19 = authority_matrix::checked(ReconnectCandidateBindingV1::new(
                    lost.commit().game_session_id(), identity19.reconnect_attempt_ref(),
                    authority_matrix::checked(ConnectionGeneration::new(2))?,
                    authority_matrix::checked(AuthenticatedTransportRefV1::decode(&[0x64; 16]))?, now + 5,
                ))?;
                {
                    let mut current = abandoned.current.lock().map_err(|_| "owner lock")?;
                    current.snapshot.candidate = candidate19;
                    current.snapshot.proof_transition.candidate = candidate19;
                }
                let verified = verify_recovery_grant_durability_v2(&token, now, &RecoveryDurabilityTrustContextV2::from_owning_source(&security), &recovery)
                    .map_err(|error| format!("verify: {error:?}"))?;
                let authorization = CompleteReconnectAuthorizationV1::authorize(abandoned.as_ref(), identity19.clone(), CompleteReconnectProofV1::V2(Box::new(verified)), now)
                    .map_err(|e| format!("abandoned authorize: {e:?}"))?;
                let mut abandoned_flow = CompleteReconnectFlowV1::begin(authorization, None).map_err(|e| format!("abandoned begin: {e:?}"))?;
                let request = std::sync::Arc::new(abandoned_flow.take_request(CompleteReconnectRequestKindV1::Prepare).map_err(|e| format!("abandoned take: {e:?}"))?);
                assert!(matches!(store.apply_complete_reconnect(request, abandoned).await?, CompleteReconnectOutcomeV1::Prepared { .. }));
                assert!(store.abort_complete_reconnect(&identity19).await?);
                assert!(!store.abort_complete_reconnect(&identity19).await?);
                let budget = store.recovery_budget(lost.commit().game_session_id(), authority_matrix::checked(ControlLossEpochRefV1::new(1))?).await?;
                assert_eq!(budget.entries().len(), 1);
                assert_eq!(budget.entries()[0].disposition, RetainedRecoveryAttemptDispositionV1::Terminal);
                source.current.lock().map_err(|_| "owner lock")?.snapshot.budget = budget;
            }
            let verified = verify_recovery_grant_durability_v2(&token, now, &RecoveryDurabilityTrustContextV2::from_owning_source(&security), &recovery)
                .map_err(|error| format!("verify: {error:?}"))?;
            let authorization = CompleteReconnectAuthorizationV1::authorize(source.as_ref(), identity.clone(), CompleteReconnectProofV1::V2(Box::new(verified)), now).map_err(|e| format!("authorize: {e:?}"))?;
            {
                // A retained budget the durable receipts do not support is refused before any
                // write: no reservation, no named attempt.
                let mut forged = source.current.lock().map_err(|_| "owner lock")?.clone();
                forged.snapshot.budget = authority_matrix::checked(RetainedRecoveryBudgetV1::restore(
                    authority_matrix::checked(ControlLossEpochRefV1::new(1))?, RecoveryEpochStateV1::Open, true,
                    vec![RetainedRecoveryAttemptV1 { attempt: authority_matrix::checked(ReconnectAttemptRef::new(7))?, transport: authority_matrix::checked(AuthenticatedTransportRefV1::decode(&[0x62; 16]))?, disposition: RetainedRecoveryAttemptDispositionV1::Terminal }],
                ))?;
                let forged = std::sync::Arc::new(Owner { current: std::sync::Mutex::new(forged), security: security.clone() });
                let verified = verify_recovery_grant_durability_v2(&token, now, &RecoveryDurabilityTrustContextV2::from_owning_source(&security), &recovery)
                    .map_err(|error| format!("verify: {error:?}"))?;
                let authorization = CompleteReconnectAuthorizationV1::authorize(forged.as_ref(), identity.clone(), CompleteReconnectProofV1::V2(Box::new(verified)), now)
                    .map_err(|e| format!("forged authorize: {e:?}"))?;
                let mut forged_flow = CompleteReconnectFlowV1::begin(authorization, None).map_err(|e| format!("forged begin: {e:?}"))?;
                let request = std::sync::Arc::new(forged_flow.take_request(CompleteReconnectRequestKindV1::Prepare).map_err(|e| format!("forged take: {e:?}"))?);
                assert_eq!(store.apply_complete_reconnect(request, forged).await?, CompleteReconnectOutcomeV1::Rejected);
                let reservations: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_transport_ref_reservations WHERE transport_ref = $1").bind([0x61u8; 16].as_slice()).fetch_one(&pool).await?;
                assert_eq!(reservations, 0);
            }
            let mut flow = CompleteReconnectFlowV1::begin(authorization, None).map_err(|e| format!("begin: {e:?}"))?;
            let prepare = std::sync::Arc::new(flow.take_request(CompleteReconnectRequestKindV1::Prepare).map_err(|e| format!("take prepare: {e:?}"))?);
            let prepared = store.apply_complete_reconnect(prepare.clone(), source.clone()).await?;
            assert!(matches!(prepared, CompleteReconnectOutcomeV1::Prepared { .. }), "{prepared:?}");
            {
                // Only one attempt may be prepared at a time.
                let other = source.current.lock().map_err(|_| "owner lock")?.clone();
                let other = std::sync::Arc::new(Owner { current: std::sync::Mutex::new(other), security: security.clone() });
                let identity21 = authority_matrix::checked(ReconnectIdentityV1::new(
                    lost.commit().game_session_id(), authority_matrix::checked(ReconnectAttemptRef::new(21))?, account,
                    lost.commit().character_id(), lost.commit().world_id(), lost.current_runtime_scope(),
                ))?;
                let candidate21 = authority_matrix::checked(ReconnectCandidateBindingV1::new(
                    lost.commit().game_session_id(), identity21.reconnect_attempt_ref(),
                    authority_matrix::checked(ConnectionGeneration::new(2))?,
                    authority_matrix::checked(AuthenticatedTransportRefV1::decode(&[0x63; 16]))?, now + 5,
                ))?;
                {
                    let mut current = other.current.lock().map_err(|_| "owner lock")?;
                    current.snapshot.candidate = candidate21;
                    current.snapshot.proof_transition.candidate = candidate21;
                }
                let verified = verify_recovery_grant_durability_v2(&token, now, &RecoveryDurabilityTrustContextV2::from_owning_source(&security), &recovery)
                    .map_err(|error| format!("verify: {error:?}"))?;
                let authorization = CompleteReconnectAuthorizationV1::authorize(other.as_ref(), identity21, CompleteReconnectProofV1::V2(Box::new(verified)), now)
                    .map_err(|e| format!("second authorize: {e:?}"))?;
                let mut second = CompleteReconnectFlowV1::begin(authorization, None).map_err(|e| format!("second begin: {e:?}"))?;
                let request = std::sync::Arc::new(second.take_request(CompleteReconnectRequestKindV1::Prepare).map_err(|e| format!("second take: {e:?}"))?);
                assert_eq!(store.apply_complete_reconnect(request, other).await?, CompleteReconnectOutcomeV1::Rejected);
            }
            // Exact replay of the same PREPARE returns the original decision.
            assert_eq!(store.apply_complete_reconnect(prepare.clone(), source.clone()).await?, prepared);
            let budget = store.recovery_budget(lost.commit().game_session_id(), authority_matrix::checked(ControlLossEpochRefV1::new(1))?).await?;
            assert_eq!(budget.entries().len(), 2);
            assert_eq!(budget.entries()[1].disposition, RetainedRecoveryAttemptDispositionV1::Prepared);
            {
                let mut current = source.current.lock().map_err(|_| "owner lock")?;
                current.snapshot.budget = budget;
                current.prepared = Some(Box::new(flow.operation().clone()));
            }
            flow.accept_completion(&mut Report(Some(CompleteReconnectCompletionV1 { operation: flow.operation().clone(), outcome: prepared })))
                .map_err(|error| format!("prepared completion: {error:?}"))?;
            // COMMIT is reauthorized from fresh sources, never from the PREPARE capability.
            let fresh = verify_recovery_grant_durability_v2(&token, now, &RecoveryDurabilityTrustContextV2::from_owning_source(&security), &recovery)
                .map_err(|error| format!("reverify: {error:?}"))?;
            let final_authorization = CompleteReconnectAuthorizationV1::reauthorize_history(flow.operation().recovery.clone(), CompleteReconnectProofV1::V2(Box::new(fresh)), source.as_ref(), now)
                .map_err(|e| format!("reauthorize: {e:?}"))?;
            flow.resume_prepared(final_authorization, source.as_ref(), now).map_err(|e| format!("resume: {e:?}"))?;
            let commit = std::sync::Arc::new(flow.take_request(CompleteReconnectRequestKindV1::Commit).map_err(|e| format!("take commit: {e:?} phase {:?}", flow.phase()))?);
            sqlx::query("INSERT INTO game_durability_recovery_grant_consumptions (recovery_grant_nonce, game_session_id, reconnect_attempt_ref) VALUES ($1, encode($2,'hex')::uuid, $3)")
                .bind([8u8; 32].as_slice()).bind(lost.commit().game_session_id().as_bytes().as_slice()).bind(9u64.to_be_bytes().as_slice())
                .execute(&pool).await?;
            assert_eq!(store.apply_complete_reconnect(commit.clone(), source.clone()).await?, CompleteReconnectOutcomeV1::Rejected);
            assert_eq!(store.current_session_at(lost.commit().game_session_id()).await?.0, lost);
            sqlx::query("DELETE FROM game_durability_recovery_grant_consumptions").execute(&pool).await?;
            let committed = store.apply_complete_reconnect(commit.clone(), source.clone()).await?;
            let CompleteReconnectOutcomeV1::Committed { decided_at } = committed else { return Err(format!("commit: {committed:?}").into()); };
            assert!(decided_at >= loss_decided_at);
            // Same GameSession, strictly newer connection generation on the candidate transport.
            let (resumed, _) = store.current_session_at(lost.commit().game_session_id()).await?;
            assert_eq!(resumed.session_state(), GameSessionState::Active);
            assert_eq!(resumed.current_game_session_id(), lost.commit().game_session_id());
            assert_eq!(resumed.current_connection_generation().get(), 2);
            assert_eq!(resumed.current_transport(), Some(candidate.transport_ref()));
            assert_eq!(resumed.current_character_lease(), lost.current_character_lease());
            let consumed: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_recovery_grant_consumptions").fetch_one(&pool).await?;
            assert_eq!(consumed, 1);
            // Replay and reconcile return the original decision; nothing is decided twice.
            assert_eq!(store.apply_complete_reconnect(commit.clone(), source.clone()).await?, committed);
            let reconciled = store.reconcile_complete_reconnect(flow.operation()).await?.ok_or("missing completion")?;
            assert_eq!(reconciled.outcome, committed);
            let restored = store.recovery_budget(lost.commit().game_session_id(), authority_matrix::checked(ControlLossEpochRefV1::new(1))?).await?;
            assert_eq!(restored.state(), RecoveryEpochStateV1::Restored);
            assert_eq!(restored.entries()[0].disposition, RetainedRecoveryAttemptDispositionV1::Terminal);
            assert_eq!(restored.entries()[1].disposition, RetainedRecoveryAttemptDispositionV1::Committed);
            // A fresh PREPARE against the resumed (ACTIVE) session is refused.
            let active = source.current.lock().map_err(|_| "owner lock")?.snapshot.clone();
            assert!(CompleteReconnectAuthorizationV1::authorize(
                &Owner { current: std::sync::Mutex::new(CompleteReconnectCurrentV1 { snapshot: CompleteReconnectSnapshotV1 { session: resumed, ..active }, prepared: None }), security },
                identity, CompleteReconnectProofV1::V1Token(token), now).is_err());
            // The resumed connection ends again: only its exact transport releases the
            // session (never stranded ACTIVE), and a foreign transport changes nothing.
            let resumed_id = lost.commit().game_session_id();
            assert_eq!(
                store.release_abandoned_session(resumed_id, account, authority_matrix::checked(AuthenticatedTransportRefV1::decode(&[0x63; 16]))?).await?,
                durability::fresh_admission::ExpiredLossReleaseV1::NotApplicable
            );
            assert!(matches!(
                store.release_abandoned_session(resumed_id, account, candidate.transport_ref()).await?,
                durability::fresh_admission::ExpiredLossReleaseV1::Released { .. }
            ));
            assert_eq!(store.current_session_at(resumed_id).await?.0.session_state(), GameSessionState::Terminal);
            let rows = guards.load(&keys).await?;
            assert!(matches!(rows[0].as_ref().map(|row| &row.state), Some(AdmissionAuthorityGuardStateV1::Account { presence: None, .. })));
            assert!(matches!(rows[1].as_ref().map(|row| &row.state), Some(AdmissionAuthorityGuardStateV1::Character { holder: None, .. })));
            pool.close().await;
            Ok::<(), Box<dyn std::error::Error>>(())
        }.await;
        database.cleanup().await?;
        result
    }))
}

#[test]
fn resumed_session_loss_opens_the_next_epoch_and_resumes_again()
-> Result<(), Box<dyn std::error::Error>> {
    use base64::Engine;
    use durability::admission_authority_guards::AdmissionGuardStore;
    use durability::fresh_admission::{
        FreshAdmissionStore, FreshLossReconciliation, FreshReconciliation,
    };
    use ed25519_dalek::{Signer, SigningKey};
    use foundation::admission_authority_publication::*;
    use foundation::fnd04_verifier::*;
    use foundation::*;
    struct LossSource(ControlLossObservationV1);
    impl foundation::fnd04_verifier::recovery_source_sealed::Sealed for LossSource {}
    impl ControlLossSourceV1 for LossSource {
        fn resolve_loss(
            &self,
            _: GameSessionId,
            _: i64,
        ) -> Result<ControlLossObservationV1, ReconnectDurabilityErrorV1> {
            Ok(self.0.clone())
        }
    }
    #[derive(Clone)]
    struct Security {
        signing: RecoverySigningTrustObservationV2,
        security: RecoveryAccountSecurityObservationV2,
    }
    impl foundation::fnd04_verifier::recovery_source_sealed::Sealed for Security {}
    impl RecoveryDurabilityEvidenceSourceV2 for Security {
        fn signing_trust(
            &self,
            _: &str,
            _: i64,
        ) -> Result<RecoverySigningTrustObservationV2, Fnd04EvidenceError> {
            Ok(self.signing.clone())
        }
        fn account_security(
            &self,
            _: &str,
            _: i64,
        ) -> Result<RecoveryAccountSecurityObservationV2, Fnd04EvidenceError> {
            Ok(self.security.clone())
        }
    }
    struct Owner {
        current: std::sync::Mutex<CompleteReconnectCurrentV1>,
        security: Security,
    }
    impl foundation::fnd04_verifier::recovery_source_sealed::Sealed for Owner {}
    impl CompleteReconnectSourceV1 for Owner {
        fn resolve_reconnect(
            &self,
            _: &ReconnectIdentityV1,
            _: i64,
        ) -> Result<CompleteReconnectCurrentV1, ReconnectDurabilityErrorV1> {
            Ok(match self.current.lock() {
                Ok(current) => current.clone(),
                Err(poisoned) => poisoned.into_inner().clone(),
            })
        }
        fn recovery_v2_source(&self) -> Option<&dyn RecoveryDurabilityEvidenceSourceV2> {
            Some(&self.security)
        }
    }
    struct Report(Option<CompleteReconnectCompletionV1>);
    impl foundation::fnd04_verifier::recovery_source_sealed::Sealed for Report {}
    impl CompleteReconnectCompletionSourceV1 for Report {
        fn take_complete_reconnect_completion(
            &mut self,
            _: &CompleteReconnectDurabilityOperationV1,
        ) -> Result<Option<CompleteReconnectCompletionV1>, ReconnectDurabilityErrorV1> {
            Ok(self.0.take())
        }
    }
    fn uuid(bytes: &[u8; 16]) -> String {
        let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        format!(
            "{}-{}-{}-{}-{}",
            &hex[..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..]
        )
    }
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let database = postgres::IsolatedPostgres::create("resumed_session_loss").await?;
        let result = async {
            let url = database.database_url()?;
            MigrationExecutor::connect_migration(&url).await?.apply_embedded_ledger().await?;
            let pool = sqlx::PgPool::connect(&url).await?;
            let now = postgres_clock(&pool).await?;
            let owner = postgres::fresh::Source::new(now)?;
            let guards = AdmissionGuardStore::connect_runtime(&url, 8192).await?;
            guards.publish(&authority_matrix::checked(AdmissionAuthorityPublicationV1::prepare(&owner, now))?).await?;
            let store = FreshAdmissionStore::connect_runtime(&url, 65536, 8192).await?;
            let request = owner.request()?;
            store.commit(&request).await?;
            let FreshReconciliation::Committed(initial) = store.reconcile(request.operation()).await? else { return Err("missing fresh session".into()); };
            let session = initial.current_session;
            let session_id = session.commit().game_session_id();
            let account = "00000000-0000-4000-8000-000000000001";
            let presence = authority_matrix::checked(AccountPresenceClaimV1::new(account, session.commit().character_id()))?;
            let protection = RecoveryProtectionContinuityV1 {
                usage: RecoveryProtectionUseV1::Unused { entitlement_generation: 1 },
                rearm: RecoveryProtectionRearmV1::Satisfied { generation: 1, established_at: now },
            };
            let loss_source = std::sync::Arc::new(LossSource(ControlLossObservationV1 {
                source_authority: session.current_runtime_scope(), source_revision: 1, accepted_source_revision: 1,
                decision_identity: authority_matrix::checked(ControlLossEpochRefV1::new(1))?,
                accepted_decision_identity: authority_matrix::checked(ControlLossEpochRefV1::new(1))?,
                observed_at: now, session, account_presence: presence.clone(),
                placement_identity: [9; 16], placement_revision: 1, actor_present: true, runtime_ready: true,
                cause: ControlLossCauseV1::AuthoritativeUnexpectedLoss,
                loss_epoch: authority_matrix::checked(ControlLossEpochRefV1::new(1))?, loss_origin: now,
                original_grace_deadline: now + 120, history: ControlLossHistoryV1::FreshOrigin, protection,
            }));
            let authorization = authority_matrix::checked(ControlLossAuthorizationV1::authorize(loss_source.as_ref(), session_id, now))?;
            let mut loss_flow = ControlLossFlowV1::begin(authorization);
            let loss = std::sync::Arc::new(authority_matrix::checked(loss_flow.take_request())?);
            let ControlLossOutcomeV1::Committed { decided_at: loss_decided_at } = store.commit_fresh_loss(loss.clone(), loss_source.clone()).await? else { return Err("loss not committed".into()); };
            let (lost, _now_after_loss) = store.current_session_at(session_id).await?;
            assert_eq!(lost.session_state(), GameSessionState::Reconnectable);
            // The durable clock may have advanced by the time the commit actually
            // decided; every later timing check requires `now >= loss_decided_at`.
            let now = now.max(loss_decided_at);

            // A Platform recovery credential for exactly this account, character and world.
            let key = SigningKey::from_bytes(&[23; 32]);
            let character = uuid(session.commit().character_id().as_bytes());
            let world = uuid(session.commit().world_id().as_bytes());
            let nonce = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([8; 32]);
            let header = r#"{"alg":"Ed25519","kid":"recovery-1","typ":"oteryn-recovery+jwt"}"#;
            let payload = format!(
                r#"{{"iss":"urn:oteryn:platform:game-recovery","aud":"urn:oteryn:game:recovery","iat":{now},"nbf":{now},"exp":{},"jti":"{nonce}","profile":"oteryn-reauth-recovery-v1","purpose":"existing_actor_recovery","attempt_ref":"00000000-0000-7000-8000-000000000001","account_id":"{account}","character_id":"{character}","world_id":"{world}","account_security_generation":"1","protocol_major":1,"transport_profile":1,"ruleset_revision":"rules-1","content_revision":"content-1","map_revision":"map-1","world_policy_revision":"policy-1"}}"#,
                now + 30
            );
            let input = format!("{}.{}", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header), base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload));
            let token = format!("{input}.{}", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key.sign(input.as_bytes()).to_bytes()));
            let provenance = |purpose| FreshEvidenceProvenanceV1 {
                source_authority: "independent-platform-fixture".into(), purpose, scope: Fnd04EvidenceScope::ExistingActorRecovery,
                source_revision: 1, accepted_source_revision: 1, decision_identity: "recovery-1".into(), accepted_decision_identity: "recovery-1".into(),
                source_observed_at: now, clock_uncertainty_seconds: 0, publication_revision: 1,
            };
            let security = Security {
                signing: RecoverySigningTrustObservationV2 { key_id: "recovery-1".into(), public_key: key.verifying_key().to_bytes(), trusted: true, provenance: provenance(FreshEvidencePurposeV1::SigningTrust) },
                security: RecoveryAccountSecurityObservationV2 { account_id: account.into(), minimum_generation: 1, allowed: true, provenance: provenance(FreshEvidencePurposeV1::PlatformSecurity) },
            };
            let recovery = RecoveryCurrentEvidence {
                account_id: account.into(), character_id: session.commit().character_id(), world_id: session.commit().world_id(),
                ruleset_revision: "rules-1".into(), content_revision: "content-1".into(), map_revision: "map-1".into(), world_policy_revision: "policy-1".into(),
            };
            let identity = authority_matrix::checked(ReconnectIdentityV1::new(
                session_id, authority_matrix::checked(ReconnectAttemptRef::new(20))?, account,
                session.commit().character_id(), session.commit().world_id(), session.current_runtime_scope(),
            ))?;
            let candidate = authority_matrix::checked(ReconnectCandidateBindingV1::new(
                session_id, identity.reconnect_attempt_ref(),
                authority_matrix::checked(ConnectionGeneration::new(2))?,
                authority_matrix::checked(AuthenticatedTransportRefV1::decode(&[0x61; 16]))?, now + 5,
            ))?;
            let keys = vec![
                AdmissionAuthorityGuardKeyV1::Account { account_id: account.into() },
                AdmissionAuthorityGuardKeyV1::Character(session.commit().character_id()),
            ];
            let claims = guards.load(&keys).await?.into_iter().collect::<Option<Vec<_>>>().ok_or("missing claim rows")?;
            let snapshot = CompleteReconnectSnapshotV1 {
                replacement_anchor: None, predecessor_attempts: vec![],
                loss: loss.operation().clone(), loss_decided_at,
                source_authority: lost.current_runtime_scope(), source_revision: 1, accepted_source_revision: 1, observed_at: now,
                session: lost, account_presence: presence, actor_present: true, runtime_ready: true,
                placement_identity: [9; 16], placement_revision: 1, protection,
                budget: store.recovery_budget(session_id, authority_matrix::checked(ControlLossEpochRefV1::new(1))?).await?,
                candidate,
                proof_transition: CompleteReconnectProofTransitionV1 {
                    owner: lost.current_runtime_scope(), revision: 1, accepted_revision: 1, observed_at: now,
                    predecessor_session: session_id, predecessor_generation: 1,
                    successor_session: session_id, successor_generation: 2, candidate,
                },
                fnd02: authority_matrix::checked(Fnd02ReconciliationFenceV1::new(authority_matrix::checked(CommandId::new(1))?, vec![], 1, vec![]))?,
                recovery: recovery.clone(), claims,
            };
            let source = std::sync::Arc::new(Owner {
                current: std::sync::Mutex::new(CompleteReconnectCurrentV1 { snapshot, prepared: None }),
                security: security.clone(),
            });
            let verified = verify_recovery_grant_durability_v2(&token, now, &RecoveryDurabilityTrustContextV2::from_owning_source(&security), &recovery)
                .map_err(|error| format!("verify: {error:?}"))?;
            let authorization = CompleteReconnectAuthorizationV1::authorize(source.as_ref(), identity.clone(), CompleteReconnectProofV1::V2(Box::new(verified)), now)
                .map_err(|e| format!("authorize: {e:?}"))?;
            let mut flow = CompleteReconnectFlowV1::begin(authorization, None).map_err(|e| format!("begin: {e:?}"))?;
            let prepare = std::sync::Arc::new(flow.take_request(CompleteReconnectRequestKindV1::Prepare).map_err(|e| format!("take prepare: {e:?}"))?);
            let prepared = store.apply_complete_reconnect(prepare.clone(), source.clone()).await?;
            assert!(matches!(prepared, CompleteReconnectOutcomeV1::Prepared { .. }), "{prepared:?}");
            let budget = store.recovery_budget(session_id, authority_matrix::checked(ControlLossEpochRefV1::new(1))?).await?;
            {
                let mut current = source.current.lock().map_err(|_| "owner lock")?;
                current.snapshot.budget = budget;
                current.prepared = Some(Box::new(flow.operation().clone()));
            }
            flow.accept_completion(&mut Report(Some(CompleteReconnectCompletionV1 { operation: flow.operation().clone(), outcome: prepared })))
                .map_err(|error| format!("prepared completion: {error:?}"))?;
            let fresh = verify_recovery_grant_durability_v2(&token, now, &RecoveryDurabilityTrustContextV2::from_owning_source(&security), &recovery)
                .map_err(|error| format!("reverify: {error:?}"))?;
            let final_authorization = CompleteReconnectAuthorizationV1::reauthorize_history(flow.operation().recovery.clone(), CompleteReconnectProofV1::V2(Box::new(fresh)), source.as_ref(), now)
                .map_err(|e| format!("reauthorize: {e:?}"))?;
            flow.resume_prepared(final_authorization, source.as_ref(), now).map_err(|e| format!("resume: {e:?}"))?;
            let commit = std::sync::Arc::new(flow.take_request(CompleteReconnectRequestKindV1::Commit).map_err(|e| format!("take commit: {e:?} phase {:?}", flow.phase()))?);
            let committed = store.apply_complete_reconnect(commit.clone(), source.clone()).await?;
            let CompleteReconnectOutcomeV1::Committed { decided_at } = committed else { return Err(format!("commit: {committed:?}").into()); };
            assert!(decided_at >= loss_decided_at);
            let (resumed, _) = store.current_session_at(session_id).await?;
            assert_eq!(resumed.session_state(), GameSessionState::Active);
            assert_eq!(resumed.current_game_session_id(), session_id);
            assert_eq!(resumed.current_connection_generation().get(), 2);
            assert_eq!(resumed.current_transport(), Some(candidate.transport_ref()));

            // 1. `resumed_history` reports the epoch-1 loss's retained history, with
            // protection now Activated by the just-committed same-session resume.
            let history_epoch1 = store.resumed_history(session_id).await?.ok_or("missing resumed history")?;
            let ControlLossHistoryV1::Resumed { budget: budget1, original_grace_deadline: grace1, protection: protection1 } = history_epoch1.clone() else {
                return Err("expected Resumed history".into());
            };
            assert_eq!(budget1.epoch(), authority_matrix::checked(ControlLossEpochRefV1::new(1))?);
            assert_eq!(budget1.state(), RecoveryEpochStateV1::Restored);
            let committed_entries: Vec<_> = budget1.entries().iter().filter(|entry| entry.disposition == RetainedRecoveryAttemptDispositionV1::Committed).collect();
            assert_eq!(committed_entries.len(), 1);
            assert_eq!(committed_entries[0].transport, candidate.transport_ref());
            assert_eq!(grace1, resumed.current_original_grace_deadline().ok_or("grace")?);
            assert_eq!(
                protection1,
                RecoveryProtectionContinuityV1 {
                    usage: RecoveryProtectionUseV1::Activated { entitlement_generation: 1, activated_at: decided_at, deadline: decided_at + 4 },
                    // `after_complete_reconnect` only ever touches `usage`; the original
                    // rearm evidence (fixed at `protection`'s construction, before the
                    // durable clock may have advanced) is carried through unchanged.
                    rearm: protection.rearm,
                }
            );

            let now2 = postgres_clock(&pool).await?;
            let presence2 = authority_matrix::checked(AccountPresenceClaimV1::new(account, session.commit().character_id()))?;
            let make_observation = |epoch: u64, grace_deadline: i64, history: ControlLossHistoryV1, protection: RecoveryProtectionContinuityV1| -> Result<ControlLossObservationV1, Box<dyn std::error::Error>> {
                Ok(ControlLossObservationV1 {
                    source_authority: resumed.current_runtime_scope(), source_revision: 1, accepted_source_revision: 1,
                    decision_identity: authority_matrix::checked(ControlLossEpochRefV1::new(epoch))?,
                    accepted_decision_identity: authority_matrix::checked(ControlLossEpochRefV1::new(epoch))?,
                    observed_at: now2, session: resumed, account_presence: presence2.clone(),
                    placement_identity: [9; 16], placement_revision: 1, actor_present: true, runtime_ready: true,
                    cause: ControlLossCauseV1::AuthoritativeUnexpectedLoss,
                    loss_epoch: authority_matrix::checked(ControlLossEpochRefV1::new(epoch))?, loss_origin: now2,
                    original_grace_deadline: grace_deadline, history, protection,
                })
            };

            // 2a. Skipping an epoch (3 instead of 2) is otherwise foundation-valid
            // (budget.epoch() 1 < loss_epoch 3) but is rejected by the durable
            // re-derivation's own epoch-continuity check.
            {
                let observation = make_observation(3, now2 + 60, ControlLossHistoryV1::Resumed { budget: budget1.clone(), original_grace_deadline: grace1, protection: protection1 }, protection1)?;
                let source3 = std::sync::Arc::new(LossSource(observation));
                let authorization3 = authority_matrix::checked(ControlLossAuthorizationV1::authorize(source3.as_ref(), session_id, now2))?;
                let mut flow3 = ControlLossFlowV1::begin(authorization3);
                let request3 = std::sync::Arc::new(authority_matrix::checked(flow3.take_request())?);
                assert_eq!(store.commit_fresh_loss(request3, source3.clone()).await?, ControlLossOutcomeV1::Rejected, "an epoch skip must be rejected");
                assert_eq!(store.current_session_at(session_id).await?.0, resumed, "the session row is unchanged");
            }

            // 2b. A retained history whose protection was not genuinely left by the
            // resume (Foundation only checks internal self-consistency between the
            // observation's protection and the history's protection, never that this
            // protection is what the resume actually produced) is rejected by the
            // durable layer, which re-derives the real protection independently.
            {
                let observation = make_observation(2, now2 + 60, ControlLossHistoryV1::Resumed { budget: budget1.clone(), original_grace_deadline: grace1, protection }, protection)?;
                let source_bogus = std::sync::Arc::new(LossSource(observation));
                let authorization_bogus = authority_matrix::checked(ControlLossAuthorizationV1::authorize(source_bogus.as_ref(), session_id, now2))?;
                let mut flow_bogus = ControlLossFlowV1::begin(authorization_bogus);
                let request_bogus = std::sync::Arc::new(authority_matrix::checked(flow_bogus.take_request())?);
                assert_eq!(store.commit_fresh_loss(request_bogus, source_bogus.clone()).await?, ControlLossOutcomeV1::Rejected, "a forged retained protection must be rejected");
                assert_eq!(store.current_session_at(session_id).await?.0, resumed, "the session row is unchanged");
            }

            // 2c. A tampered `original_grace_deadline` inside the retained history is
            // NOT expressible as a durable-layer `Ok(Rejected)`: Foundation's own
            // `ControlLossObservationV1::validate` (admission_recovery_inner.rs,
            // the `Resumed` arm) requires the history's `original_grace_deadline`
            // to exactly equal `session.current_original_grace_deadline()` of that
            // SAME observation. Tampering the history's field alone therefore fails
            // inside `ControlLossAuthorizationV1::authorize` itself (a hard `Err`
            // from the `?` operator), never reaching `commit_fresh_loss`. Tampering
            // the session's own grace field to keep the two consistent instead makes
            // the whole session snapshot disagree with the live database row, which
            // is then rejected for session/predecessor mismatch, not for the grace
            // field specifically, and duplicates case 2a's mechanism rather than
            // exercising a new one. So this sub-case is skipped.

            // 3. The genuine epoch-2 loss commits with the retained history exactly
            // as `resumed_history` reported it.
            let observation2 = make_observation(2, now2 + 60, ControlLossHistoryV1::Resumed { budget: budget1.clone(), original_grace_deadline: grace1, protection: protection1 }, protection1)?;
            let source2 = std::sync::Arc::new(LossSource(observation2));
            let authorization2 = authority_matrix::checked(ControlLossAuthorizationV1::authorize(source2.as_ref(), session_id, now2))?;
            let mut flow2 = ControlLossFlowV1::begin(authorization2);
            let loss2 = std::sync::Arc::new(authority_matrix::checked(flow2.take_request())?);
            let ControlLossOutcomeV1::Committed { decided_at: loss2_decided_at } = store.commit_fresh_loss(loss2.clone(), source2.clone()).await? else { return Err("epoch-2 loss not committed".into()); };
            // The epoch-2 loss's own committed grace deadline is fixed at `loss2_now + 60`;
            // keep that value for the later assertion while advancing `now2` for every
            // timing check downstream of this commit (the durable clock may have moved on).
            let loss2_now = now2;
            let now2 = now2.max(loss2_decided_at);

            let epoch2 = authority_matrix::checked(ControlLossEpochRefV1::new(2))?;
            type EpochRow = (i64, i16, i16, Option<Vec<u8>>, Option<Vec<u8>>);
            let row: EpochRow = sqlx::query_as(
                "SELECT control_loss_epoch::BIGINT, session_state, attempt_count, prepared_attempt_ref, current_transport_ref \
                 FROM game_durability_reconnect_sessions WHERE game_session_id = encode($1,'hex')::uuid",
            )
            .bind(session_id.as_bytes().as_slice())
            .fetch_one(&pool)
            .await?;
            assert_eq!(row.0, 2);
            assert_eq!(row.1, 1, "RECONNECTABLE");
            assert_eq!(row.2, 0);
            assert!(row.3.is_none(), "no prepared attempt");
            assert!(row.4.is_none(), "no current transport");

            let (owning2, owning2_decided_at) = store.owning_loss(session_id, epoch2).await?.ok_or("missing epoch-2 receipt")?;
            assert_eq!(&owning2, loss2.operation());
            assert_eq!(owning2_decided_at, loss2_decided_at);

            // Idempotent replay: the same request re-commits to the same decision.
            assert_eq!(
                store.commit_fresh_loss(loss2.clone(), source2.clone()).await?,
                ControlLossOutcomeV1::Committed { decided_at: loss2_decided_at }
            );
            assert!(matches!(
                store.reconcile_fresh_loss(loss2.operation()).await?,
                FreshLossReconciliation::Committed { .. }
            ));

            // 4. Resume again at epoch 2, through the same complete-reconnect path,
            // with a new grant nonce and a new candidate transport/attempt.
            let (lost2, _now_after_loss2) = store.current_session_at(session_id).await?;
            assert_eq!(lost2.session_state(), GameSessionState::Reconnectable);
            // Fresh platform security/signing evidence observed at `now2`: the first
            // resume's evidence (observed at the original `now`) falls outside
            // `EVIDENCE_FRESHNESS_SECONDS_V1` by the time this second resume runs.
            let provenance2 = |purpose| FreshEvidenceProvenanceV1 {
                source_authority: "independent-platform-fixture".into(), purpose, scope: Fnd04EvidenceScope::ExistingActorRecovery,
                source_revision: 1, accepted_source_revision: 1, decision_identity: "recovery-1".into(), accepted_decision_identity: "recovery-1".into(),
                source_observed_at: now2, clock_uncertainty_seconds: 0, publication_revision: 1,
            };
            let security2 = Security {
                signing: RecoverySigningTrustObservationV2 { key_id: "recovery-1".into(), public_key: key.verifying_key().to_bytes(), trusted: true, provenance: provenance2(FreshEvidencePurposeV1::SigningTrust) },
                security: RecoveryAccountSecurityObservationV2 { account_id: account.into(), minimum_generation: 1, allowed: true, provenance: provenance2(FreshEvidencePurposeV1::PlatformSecurity) },
            };
            let nonce2 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([12; 32]);
            let payload2 = format!(
                r#"{{"iss":"urn:oteryn:platform:game-recovery","aud":"urn:oteryn:game:recovery","iat":{now2},"nbf":{now2},"exp":{},"jti":"{nonce2}","profile":"oteryn-reauth-recovery-v1","purpose":"existing_actor_recovery","attempt_ref":"00000000-0000-7000-8000-000000000002","account_id":"{account}","character_id":"{character}","world_id":"{world}","account_security_generation":"1","protocol_major":1,"transport_profile":1,"ruleset_revision":"rules-1","content_revision":"content-1","map_revision":"map-1","world_policy_revision":"policy-1"}}"#,
                now2 + 30
            );
            let input2 = format!("{}.{}", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header), base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload2));
            let token2 = format!("{input2}.{}", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key.sign(input2.as_bytes()).to_bytes()));
            let identity2 = authority_matrix::checked(ReconnectIdentityV1::new(
                session_id, authority_matrix::checked(ReconnectAttemptRef::new(30))?, account,
                session.commit().character_id(), session.commit().world_id(), lost2.current_runtime_scope(),
            ))?;
            let candidate2 = authority_matrix::checked(ReconnectCandidateBindingV1::new(
                session_id, identity2.reconnect_attempt_ref(),
                authority_matrix::checked(ConnectionGeneration::new(3))?,
                authority_matrix::checked(AuthenticatedTransportRefV1::decode(&[0x71; 16]))?, now2 + 5,
            ))?;
            let claims2 = guards.load(&keys).await?.into_iter().collect::<Option<Vec<_>>>().ok_or("missing claim rows")?;
            let snapshot2 = CompleteReconnectSnapshotV1 {
                replacement_anchor: None, predecessor_attempts: vec![],
                loss: loss2.operation().clone(), loss_decided_at: loss2_decided_at,
                source_authority: lost2.current_runtime_scope(), source_revision: 1, accepted_source_revision: 1, observed_at: now2,
                session: lost2, account_presence: presence2.clone(), actor_present: true, runtime_ready: true,
                placement_identity: [9; 16], placement_revision: 1, protection: protection1,
                budget: store.recovery_budget(session_id, epoch2).await?,
                candidate: candidate2,
                proof_transition: CompleteReconnectProofTransitionV1 {
                    owner: lost2.current_runtime_scope(), revision: 1, accepted_revision: 1, observed_at: now2,
                    predecessor_session: session_id, predecessor_generation: 2,
                    successor_session: session_id, successor_generation: 3, candidate: candidate2,
                },
                fnd02: authority_matrix::checked(Fnd02ReconciliationFenceV1::new(authority_matrix::checked(CommandId::new(1))?, vec![], 1, vec![]))?,
                recovery: recovery.clone(), claims: claims2,
            };
            let source_resume2 = std::sync::Arc::new(Owner {
                current: std::sync::Mutex::new(CompleteReconnectCurrentV1 { snapshot: snapshot2, prepared: None }),
                security: security2.clone(),
            });
            let verified2 = verify_recovery_grant_durability_v2(&token2, now2, &RecoveryDurabilityTrustContextV2::from_owning_source(&security2), &recovery)
                .map_err(|error| format!("verify2: {error:?}"))?;
            let authorization_resume2 = CompleteReconnectAuthorizationV1::authorize(source_resume2.as_ref(), identity2.clone(), CompleteReconnectProofV1::V2(Box::new(verified2)), now2)
                .map_err(|e| format!("authorize2: {e:?}"))?;
            let mut flow_resume2 = CompleteReconnectFlowV1::begin(authorization_resume2, None).map_err(|e| format!("begin2: {e:?}"))?;
            let prepare2 = std::sync::Arc::new(flow_resume2.take_request(CompleteReconnectRequestKindV1::Prepare).map_err(|e| format!("take prepare2: {e:?}"))?);
            let prepared2 = store.apply_complete_reconnect(prepare2.clone(), source_resume2.clone()).await?;
            assert!(matches!(prepared2, CompleteReconnectOutcomeV1::Prepared { .. }), "{prepared2:?}");
            let budget2_after_prepare = store.recovery_budget(session_id, epoch2).await?;
            {
                let mut current = source_resume2.current.lock().map_err(|_| "owner lock")?;
                current.snapshot.budget = budget2_after_prepare;
                current.prepared = Some(Box::new(flow_resume2.operation().clone()));
            }
            flow_resume2.accept_completion(&mut Report(Some(CompleteReconnectCompletionV1 { operation: flow_resume2.operation().clone(), outcome: prepared2 })))
                .map_err(|error| format!("prepared2 completion: {error:?}"))?;
            let fresh2 = verify_recovery_grant_durability_v2(&token2, now2, &RecoveryDurabilityTrustContextV2::from_owning_source(&security2), &recovery)
                .map_err(|error| format!("reverify2: {error:?}"))?;
            let final_authorization2 = CompleteReconnectAuthorizationV1::reauthorize_history(flow_resume2.operation().recovery.clone(), CompleteReconnectProofV1::V2(Box::new(fresh2)), source_resume2.as_ref(), now2)
                .map_err(|e| format!("reauthorize2: {e:?}"))?;
            flow_resume2.resume_prepared(final_authorization2, source_resume2.as_ref(), now2).map_err(|e| format!("resume2: {e:?}"))?;
            let commit2 = std::sync::Arc::new(flow_resume2.take_request(CompleteReconnectRequestKindV1::Commit).map_err(|e| format!("take commit2: {e:?} phase {:?}", flow_resume2.phase()))?);
            let committed2 = store.apply_complete_reconnect(commit2.clone(), source_resume2.clone()).await?;
            let CompleteReconnectOutcomeV1::Committed { decided_at: decided_at2 } = committed2 else { return Err(format!("commit2: {committed2:?}").into()); };
            assert!(decided_at2 >= loss2_decided_at);

            let (resumed2, _) = store.current_session_at(session_id).await?;
            assert_eq!(resumed2.session_state(), GameSessionState::Active);
            assert_eq!(resumed2.current_game_session_id(), session_id);
            assert_eq!(resumed2.current_connection_generation().get(), 3);
            assert_eq!(resumed2.current_transport(), Some(candidate2.transport_ref()));
            assert_eq!(resumed2.current_control_loss_epoch().map(ControlLossEpochRefV1::get), Some(2));

            // 5. `resumed_history` now reports the epoch-2 budget, Restored, with the
            // same (already Activated) protection unchanged.
            let history_epoch2 = store.resumed_history(session_id).await?.ok_or("missing epoch-2 resumed history")?;
            let ControlLossHistoryV1::Resumed { budget: budget2, original_grace_deadline: grace2, protection: protection2 } = history_epoch2 else {
                return Err("expected Resumed history at epoch 2".into());
            };
            assert_eq!(budget2.epoch(), epoch2);
            assert_eq!(budget2.state(), RecoveryEpochStateV1::Restored);
            let committed_entries2: Vec<_> = budget2.entries().iter().filter(|entry| entry.disposition == RetainedRecoveryAttemptDispositionV1::Committed).collect();
            assert_eq!(committed_entries2.len(), 1);
            assert_eq!(committed_entries2[0].transport, candidate2.transport_ref());
            assert_eq!(grace2, loss2_now + 60);
            assert_eq!(protection2, protection1);

            pool.close().await;
            Ok::<(), Box<dyn std::error::Error>>(())
        }.await;
        database.cleanup().await?;
        result
    })
}
