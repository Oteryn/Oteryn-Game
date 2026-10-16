// Store-boundary qualification using the real atomic owner and existing independent
// signed fresh-admission fixture. This does not stand in for Platform/TLS or transport E2E.
extern crate oteryn_game_server as production_server;
extern crate self as oteryn_game_server;
pub use production_server::admission_evidence;
pub use production_server::domain;
#[path = "support/authority_matrix.rs"]
mod authority_matrix;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_recovery_fence.rs"]
pub mod character_recovery_fence;
#[allow(dead_code, unused_imports)]
#[path = "../src/durability/mod.rs"]
mod durability;
#[allow(dead_code, unused_imports)]
#[path = "../src/foundation/mod.rs"]
pub mod foundation;
#[allow(dead_code, unused_imports)]
#[path = "../src/native_admission_source/mod.rs"]
pub mod native_admission_source;
#[path = "support/postgres.rs"]
mod postgres;

use durability::admission_authority_guards::AdmissionGuardStore;
use durability::fresh_admission::{CurrentCapabilityCheck, FreshAdmissionStore};
use foundation::RuntimeScopeRefV1;
use foundation::admission_authority_publication::AdmissionAuthorityPublicationV1;
use foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

// The shared PostgreSQL support module includes its existing reconnect contract tests.
// Their root data builders are reused verbatim from durability_postgres.rs
// (foundation_error through record_for_actor_epoch_with_protection), excluding
// the unused postgres_clock helper. Only visibility changes for root reexports.
mod reconnect_contract_fixture {
    use super::{foundation::*, postgres};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub(super) fn foundation_error(error: ReconnectDurabilityErrorV1) -> std::io::Error {
        std::io::Error::other(format!(
            "Foundation V1 record construction failed: {error:?}"
        ))
    }

    pub(super) fn unix_now() -> Result<i64, ReconnectDurabilityErrorV1> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?
            .as_secs()
            .try_into()
            .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)
    }

    pub(super) fn uuid_v7(raw: u64) -> [u8; 16] {
        let mut value = [0u8; 16];
        value[8..].copy_from_slice(&raw.to_be_bytes());
        value[6] = 0x70;
        value[8] = (value[8] & 0x3f) | 0x80;
        value
    }

    pub(super) fn record(
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
    pub(super) fn record_for_epoch(
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

    pub(super) fn record_for_actor(
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
                    CommandId::new(1)
                        .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
                    PendingCommandDispositionV1::PendingOriginal,
                ),
                PendingCommandReconciliationV1::new(
                    CommandId::new(2)
                        .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
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
}

use reconnect_contract_fixture::{
    foundation_error, record, record_for_actor, record_for_epoch, unix_now, uuid_v7,
};

async fn require_postgres_17_6(pool: &sqlx::PgPool) -> TestResult {
    let version: i32 = sqlx::query_scalar("SELECT current_setting('server_version_num')::int")
        .fetch_one(pool)
        .await?;
    if version != 170006 {
        return Err(format!("ADMIT0 requires PostgreSQL 17.6 (170006), received {version}").into());
    }
    Ok(())
}

async fn durable_counts(
    pool: &sqlx::PgPool,
) -> Result<(i64, i64, i64, i64, i64, i64), sqlx::Error> {
    sqlx::query_as("SELECT (SELECT count(*) FROM game_durability_fresh_admission_receipts), (SELECT count(*) FROM game_durability_reconnect_sessions), (SELECT count(*) FROM game_durability_transport_ref_reservations), (SELECT count(*) FROM game_durability_admission_guard_history), (SELECT count(*) FROM game_durability_recovery_grant_consumptions), (SELECT count(*) FROM game_durability_session_use_memberships)").fetch_one(pool).await
}

#[test]
fn admit0_fresh_atomic_refusal_preserves_nonce_claims_and_reservations() -> TestResult {
    if postgres::postgres_e2e_availability()? == postgres::PostgresE2eAvailability::NotConfigured {
        eprintln!("ADMIT0 PG NOT RUN: PostgreSQL 17.6 fixture is not configured");
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("admit0_fresh_refusal").await?;
            let result = async {
                let url = database.database_url()?;
                let pool = sqlx::PgPool::connect(&url).await?;
                require_postgres_17_6(&pool).await?;
                durability::MigrationExecutor::connect_migration(&url)
                    .await?
                    .apply_embedded_ledger()
                    .await?;
                let now: i64 = sqlx::query_scalar(
                    "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint",
                )
                .fetch_one(&pool)
                .await?;
                // This fixture actually signs/verifies the grant and authorizes claims.
                // Its owner observations exist independently before the request/receipt.
                let source = postgres::fresh::Source::new(now)?;
                let guards = AdmissionGuardStore::connect_runtime(&url, 8192).await?;
                guards
                    .publish(&authority_matrix::checked(
                        AdmissionAuthorityPublicationV1::prepare(&source, now),
                    )?)
                    .await?;
                let store = FreshAdmissionStore::connect_runtime(&url, 65536, 8192).await?;
                let request = source.request()?;
                let scope =
                    RuntimeScopeRefV1::channel(source.current.world_id, source.current.channel_id);
                let keys: Vec<_> = source.rows.iter().map(|row| row.key.clone()).collect();
                let before_guards = guards.load(&keys).await?;
                let before = durable_counts(&pool).await?;
                let generation = source.current.scope_ownership_generation;
                let policy = source.current.world_policy_revision.clone();
                // Same valid authenticated request in every case. Change one support or
                // expected binding fact; the exact deciding store must reject before effects.
                for case in 0..8 {
                    let mut support = vec![4, 6, 18];
                    let mut bound_scope = scope;
                    let mut bound_generation = generation;
                    let mut bound_policy = policy.clone();
                    match case {
                        0..=2 => {
                            support.remove(case);
                        }
                        3 => {
                            bound_scope = RuntimeScopeRefV1::channel(
                                authority_matrix::world(30)?,
                                source.current.channel_id,
                            );
                        }
                        4 => {
                            bound_scope = RuntimeScopeRefV1::channel(
                                source.current.world_id,
                                authority_matrix::channel(40)?,
                            );
                        }
                        5 => {
                            bound_generation -= 1;
                        }
                        6 => {
                            bound_generation += 1;
                        }
                        _ => {
                            bound_policy = "other-policy".into();
                        }
                    }
                    let observed = Arc::new(AtomicBool::new(false));
                    let mark = observed.clone();
                    let check: CurrentCapabilityCheck =
                        Arc::new(move |current, current_generation, current_policy| {
                            mark.store(true, Ordering::SeqCst);
                            current == bound_scope
                                && current_generation == bound_generation
                                && current_policy == bound_policy
                                && [4, 6, 18].iter().all(|id| support.contains(id))
                        });
                    let deciding = store.clone();
                    let outcome = store
                        .commit_revalidated(
                            &request,
                            Some(Box::new(move |tx| {
                                Box::pin(async move {
                                    deciding
                                        .check_current_runtime_capabilities(
                                            tx, scope, generation, &check,
                                        )
                                        .await
                                })
                            })),
                        )
                        .await?;
                    assert_eq!(
                        outcome,
                        FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority,
                        "case {case}"
                    );
                    assert!(
                        observed.load(Ordering::SeqCst),
                        "current callback was not exercised"
                    );
                    assert_eq!(durable_counts(&pool).await?, before, "case {case}");
                    assert_eq!(guards.load(&keys).await?, before_guards, "case {case}");
                }
                let expected_policy = policy;
                let valid: CurrentCapabilityCheck =
                    Arc::new(move |current, current_generation, current_policy| {
                        current == scope
                            && current_generation == generation
                            && current_policy == expected_policy
                    });
                let deciding = store.clone();
                let committed = store
                    .commit_revalidated(
                        &request,
                        Some(Box::new(move |tx| {
                            Box::pin(async move {
                                deciding
                                    .check_current_runtime_capabilities(
                                        tx, scope, generation, &valid,
                                    )
                                    .await
                            })
                        })),
                    )
                    .await?;
                assert!(
                    matches!(committed, FreshAdmissionDurableOutcomeV1::Committed(_)),
                    "same nonce must still be usable after every refusal"
                );
                // An exact historical receipt is data, and does not run a new authority check.
                let invoked = Arc::new(AtomicBool::new(false));
                let mark = invoked.clone();
                let replay = store
                    .commit_revalidated(
                        &request,
                        Some(Box::new(move |_| {
                            mark.store(true, Ordering::SeqCst);
                            Box::pin(async { Ok(false) })
                        })),
                    )
                    .await?;
                assert!(matches!(
                    replay,
                    FreshAdmissionDurableOutcomeV1::ExistingCommitted(_)
                ));
                assert!(!invoked.load(Ordering::SeqCst));
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result?;
            println!("ADMIT0_PG17_6_PASS fresh_atomic_refusal");
            Ok::<(), Box<dyn std::error::Error>>(())
        })
}

#[test]
fn admit0_current_runtime_replacement_refuses_a_prepared_old_generation() -> TestResult {
    use foundation::admission_authority_publication::{
        AdmissionAuthorityGuardStateV1, AdmissionPublicationPreconditionV1,
    };
    if postgres::postgres_e2e_availability()? == postgres::PostgresE2eAvailability::NotConfigured {
        eprintln!("ADMIT0 current-owner PG NOT RUN: PostgreSQL 17.6 fixture is not configured");
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("admit0_current_owner").await?;
            let result = async {
                let url = database.database_url()?;
                let pool = sqlx::PgPool::connect(&url).await?;
                require_postgres_17_6(&pool).await?;
                durability::MigrationExecutor::connect_migration(&url)
                    .await?
                    .apply_embedded_ledger()
                    .await?;
                let now: i64 = sqlx::query_scalar(
                    "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint",
                )
                .fetch_one(&pool)
                .await?;
                let source = postgres::fresh::Source::new(now)?;
                let guards = AdmissionGuardStore::connect_runtime(&url, 8192).await?;
                guards
                    .publish(&authority_matrix::checked(
                        AdmissionAuthorityPublicationV1::prepare(&source, now),
                    )?)
                    .await?;
                let request = source.request()?;
                let scope =
                    RuntimeScopeRefV1::channel(source.current.world_id, source.current.channel_id);
                let generation = source.current.scope_ownership_generation;
                // Independently publish a new CURRENT generation after request preparation.
                // Never manufacture current facts from the prepared operation being checked.
                let mut replacement = source.rows[2].clone();
                replacement.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
                    expected_publication_revision: replacement.publication_revision,
                };
                replacement.publication_revision += 1;
                replacement.source.source_revision += 1;
                replacement.source.decision_identity = "runtime-replacement-2".into();
                if let AdmissionAuthorityGuardStateV1::Runtime {
                    ownership_generation,
                    ..
                } = &mut replacement.state
                {
                    *ownership_generation += 1;
                } else {
                    return Err("fixture runtime row missing".into());
                }
                let mut publisher = postgres::fresh::Source::new(now)?;
                publisher.rows = vec![replacement];
                assert_eq!(
                    guards
                        .publish(&authority_matrix::checked(
                            AdmissionAuthorityPublicationV1::prepare(&publisher, now)
                        )?)
                        .await?,
                    durability::admission_authority_guards::GuardPublicationDisposition::Applied
                );
                let keys: Vec<_> = source.rows.iter().map(|row| row.key.clone()).collect();
                let before_guards = guards.load(&keys).await?;
                let before = durable_counts(&pool).await?;
                let store = FreshAdmissionStore::connect_runtime(&url, 65536, 8192).await?;
                let deciding = store.clone();
                let observed = Arc::new(AtomicBool::new(false));
                let mark = observed.clone();
                let old_binding: CurrentCapabilityCheck =
                    Arc::new(move |current, current_generation, policy| {
                        mark.store(true, Ordering::SeqCst);
                        current == scope && current_generation == generation && policy == "policy-1"
                    });
                assert_eq!(
                    store
                        .commit_revalidated(
                            &request,
                            Some(Box::new(move |tx| {
                                Box::pin(async move {
                                    deciding
                                        .check_current_runtime_capabilities(
                                            tx,
                                            scope,
                                            generation,
                                            &old_binding,
                                        )
                                        .await
                                })
                            }))
                        )
                        .await?,
                    FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority
                );
                assert!(
                    !observed.load(Ordering::SeqCst),
                    "mismatched live generation must reject before the capability predicate"
                );
                assert_eq!(durable_counts(&pool).await?, before);
                assert_eq!(guards.load(&keys).await?, before_guards);
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result?;
            println!("ADMIT0_PG17_6_PASS current_runtime_replacement");
            Ok::<(), Box<dyn std::error::Error>>(())
        })
}

// Focused V2 setup retained from the existing complete-reconnect owner qualification.
#[test]
fn admit0_v2_prepare_and_commit_refusal_preserve_current_authority() -> TestResult {
    use base64::Engine;
    use durability::fresh_admission::FreshReconciliation;
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
    if postgres::postgres_e2e_availability()? == postgres::PostgresE2eAvailability::NotConfigured {
        eprintln!("ADMIT0 V2 PG NOT RUN: PostgreSQL 17.6 fixture is not configured");
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(Box::pin(async {
        let database = postgres::IsolatedPostgres::create("admit0_v2_refusal").await?;
        let result = async {
            let url = database.database_url()?;
            let pool = sqlx::PgPool::connect(&url).await?;
            require_postgres_17_6(&pool).await?;
            durability::MigrationExecutor::connect_migration(&url).await?.apply_embedded_ledger().await?;
            let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint").fetch_one(&pool).await?;
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

            let verified = verify_recovery_grant_durability_v2(&token, now,
                &RecoveryDurabilityTrustContextV2::from_owning_source(&security), &recovery)
                .map_err(|error| format!("verify: {error:?}"))?;
            let authorization = CompleteReconnectAuthorizationV1::authorize(source.as_ref(), identity,
                CompleteReconnectProofV1::V2(Box::new(verified)), now).map_err(|error| format!("authorize: {error:?}"))?;
            let mut flow = CompleteReconnectFlowV1::begin(authorization, None).map_err(|error| format!("begin: {error:?}"))?;
            let prepare = Arc::new(flow.take_request(CompleteReconnectRequestKindV1::Prepare).map_err(|error| format!("prepare: {error:?}"))?);
            let binding_scope = lost.current_runtime_scope();
            let binding_generation = lost.current_scope_generation().get();
            let binding_policy = recovery.world_policy_revision.clone();
            let valid: CurrentCapabilityCheck = Arc::new(move |scope, generation, policy| {
                scope == binding_scope && generation == binding_generation && policy == binding_policy
            });
            let before_claims = guards.load(&keys).await?;
            for phase in [CompleteReconnectRequestKindV1::Prepare, CompleteReconnectRequestKindV1::Commit] {
                if phase == CompleteReconnectRequestKindV1::Commit {
                    let prepared = store.apply_complete_reconnect_with_capabilities(prepare.clone(), source.clone(), valid.clone()).await?;
                    assert!(matches!(prepared, CompleteReconnectOutcomeV1::Prepared { .. }));
                    let budget = store.recovery_budget(lost.commit().game_session_id(), authority_matrix::checked(ControlLossEpochRefV1::new(1))?).await?;
                    {
                        let mut current = source.current.lock().map_err(|_| "owner lock")?;
                        current.snapshot.budget = budget;
                        current.prepared = Some(Box::new(flow.operation().clone()));
                    }
                    flow.accept_completion(&mut Report(Some(CompleteReconnectCompletionV1 {
                        operation: flow.operation().clone(), outcome: prepared,
                    }))).map_err(|error| format!("completion: {error:?}"))?;
                    let fresh = verify_recovery_grant_durability_v2(&token, now,
                        &RecoveryDurabilityTrustContextV2::from_owning_source(&security), &recovery)
                        .map_err(|error| format!("reverify: {error:?}"))?;
                    let authorization = CompleteReconnectAuthorizationV1::reauthorize_history(flow.operation().recovery.clone(),
                        CompleteReconnectProofV1::V2(Box::new(fresh)), source.as_ref(), now)
                        .map_err(|error| format!("reauthorize: {error:?}"))?;
                    flow.resume_prepared(authorization, source.as_ref(), now).map_err(|error| format!("resume: {error:?}"))?;
                }
                let request = if phase == CompleteReconnectRequestKindV1::Prepare { prepare.clone() }
                    else { Arc::new(flow.take_request(CompleteReconnectRequestKindV1::Commit).map_err(|error| format!("commit: {error:?}"))?) };
                let before = durable_counts(&pool).await?;
                let budget = store.recovery_budget(lost.commit().game_session_id(), authority_matrix::checked(ControlLossEpochRefV1::new(1))?).await?;
                for case in 0..7 {
                    let mut support = vec![4, 6, 18];
                    let mut expected_scope = lost.current_runtime_scope();
                    let mut expected_generation = lost.current_scope_generation().get();
                    let mut expected_policy = recovery.world_policy_revision.clone();
                    match case {
                        0..=2 => { support.remove(case); }
                        3 => { expected_scope = RuntimeScopeRefV1::channel(authority_matrix::world(30)?, lost.commit().channel_id()); }
                        4 => { expected_generation += 1; }
                        5 => { expected_generation -= 1; }
                        _ => { expected_policy = "other-policy".into(); }
                    }
                    let observed = Arc::new(AtomicBool::new(false));
                    let mark = observed.clone();
                    let refused: CurrentCapabilityCheck = Arc::new(move |scope, generation, policy| {
                        mark.store(true, Ordering::SeqCst);
                        scope == expected_scope && generation == expected_generation && policy == expected_policy
                            && [4, 6, 18].iter().all(|id| support.contains(id))
                    });
                    assert_eq!(store.apply_complete_reconnect_with_capabilities(request.clone(), source.clone(), refused).await?, CompleteReconnectOutcomeV1::Rejected);
                    assert!(observed.load(Ordering::SeqCst));
                    assert_eq!(store.current_session_at(lost.commit().game_session_id()).await?.0, lost);
                    assert_eq!(guards.load(&keys).await?, before_claims);
                    assert_eq!(store.recovery_budget(lost.commit().game_session_id(), authority_matrix::checked(ControlLossEpochRefV1::new(1))?).await?, budget);
                    assert_eq!(durable_counts(&pool).await?, before);
                }
                if phase == CompleteReconnectRequestKindV1::Commit {
                    assert!(matches!(store.apply_complete_reconnect_with_capabilities(request.clone(), source.clone(), valid.clone()).await?, CompleteReconnectOutcomeV1::Committed { .. }));
                    let (current, _) = store.current_session_at(lost.commit().game_session_id()).await?;
                    assert_eq!(current.current_connection_generation().get(), 2);
                    assert_eq!(current.current_transport(), Some(candidate.transport_ref()));
                    assert_eq!(current.current_character_lease(), lost.current_character_lease());
                    assert_eq!(durable_counts(&pool).await?.4, 1);
                }
            }
            pool.close().await;
            Ok::<(), Box<dyn std::error::Error>>(())
        }.await;
        database.cleanup().await?;
        result?;
        println!("ADMIT0_PG17_6_PASS v2_prepare_commit_refusal");
        Ok::<(), Box<dyn std::error::Error>>(())
    }))
}
