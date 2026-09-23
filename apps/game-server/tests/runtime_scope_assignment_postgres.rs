// Dedicated PostgreSQL target for GameNode process-incarnation registration and
// the Channel-only runtime-scope assignment writer. Ordinary workspace runs skip
// when the routed PostgreSQL service is absent; only configured PostgreSQL 17.6
// runs count as qualification evidence.
extern crate self as oteryn_game_server;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_recovery_fence.rs"]
pub mod character_recovery_fence;
#[allow(dead_code, unused_imports)]
#[path = "../src/domain/mod.rs"]
pub mod domain;
#[allow(dead_code, unused_imports)]
#[path = "../src/durability/mod.rs"]
mod durability;
#[allow(dead_code, unused_imports)]
#[path = "../src/foundation/mod.rs"]
pub mod foundation;

use durability::admission_authority_guards::{AdmissionGuardStore, GuardPublicationDisposition};
use durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentError, AssignmentOutcome, AssignmentPredecessor,
    AssignmentReceipt, AssignmentRejection, AssignmentRequest, AssignmentState, BootstrapSecret,
    ControlActor, LaunchBinding, MAX_IDENTITY_BYTES, MAX_PENDING_COMMANDS, NodeIncarnationProof,
    NodeRegistrationFact, OperationKey, ReconcileOutcome, RegistrationError,
    RuntimeScopeAssignmentWriter,
};
use durability::{DurabilityError, DurabilityRoot};
use foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
    AdmissionAuthorityOwningPublisherV1, AdmissionAuthorityPublicationChangeV1,
    AdmissionAuthorityPublicationErrorV1, AdmissionAuthorityPublicationV1,
    AdmissionPublicationPreconditionV1, AdmissionPublicationPurposeV1,
    AdmissionPublicationSourceV1,
};
use foundation::{ChannelId, NodeId, RuntimeScopeRefV1, WorldId};
use sqlx::{Connection, Executor};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn configured() -> bool {
    env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_some()
}

fn skipped() {
    eprintln!("PRE-ROUTING / NONCANONICAL: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured");
}

struct Database {
    admin_url: String,
    name: String,
    url: String,
}

impl Database {
    async fn create(test_name: &str) -> TestResult<Self> {
        let admin_url = env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
        if !admin_url.starts_with("postgresql://oteryn_test_admin:")
            || !admin_url.ends_with("@127.0.0.1:5432/postgres")
        {
            return Err("unsafe PostgreSQL test admin URL".into());
        }
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let name = format!("rsa_{test_name}_{suffix}");
        let mut admin = sqlx::PgConnection::connect(&admin_url).await?;
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "CREATE DATABASE {name}"
            ))))
            .await?;
        admin.close().await?;
        let prefix = admin_url
            .strip_suffix("/postgres")
            .ok_or("invalid admin URL")?;
        let url = format!("{prefix}/{name}");
        let mut connection = sqlx::PgConnection::connect(&url).await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&mut connection)
            .await?;
        assert_eq!(
            version, "170006",
            "canonical target requires PostgreSQL 17.6"
        );
        sqlx::migrate!("./migrations").run(&mut connection).await?;
        connection.close().await?;
        Ok(Self {
            admin_url,
            name,
            url,
        })
    }

    async fn cleanup(self, roles: &[&str]) -> TestResult {
        let mut admin = sqlx::PgConnection::connect(&self.admin_url).await?;
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "DROP DATABASE {} WITH (FORCE)",
                self.name
            ))))
            .await?;
        for role in roles {
            admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "DROP ROLE IF EXISTS {role}"
                ))))
                .await?;
        }
        admin.close().await?;
        Ok(())
    }

    async fn pool(&self) -> TestResult<sqlx::PgPool> {
        Ok(sqlx::PgPool::connect(&self.url).await?)
    }
}

fn run<F>(test_name: &'static str, body: F) -> TestResult
where
    F: for<'a> FnOnce(
        &'a Database,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = TestResult> + 'a>>,
{
    if !configured() {
        skipped();
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let database = Database::create(test_name).await?;
            let result = body(&database).await;
            database.cleanup(&[]).await?;
            result
        })
}

async fn ready_root(url: &str) -> Result<DurabilityRoot, DurabilityError> {
    let root = DurabilityRoot::connect_test_runtime(url)?;
    assert!(root.maintain_ready_once().await?);
    Ok(root)
}

async fn ensure_ready(root: &DurabilityRoot) -> TestResult {
    if !root.is_ready() {
        root.request_ready();
        assert!(root.maintain_ready_once().await?);
    }
    Ok(())
}

fn v7(tag: u8) -> [u8; 16] {
    [
        0x01, 0x89, 0x0f, 0x4c, 0x3b, 0x2a, 0x7c, tag, 0x8d, 0x11, 0x9a, 0x32, 0x1b, 0x7c, 0x00,
        tag,
    ]
}

fn node(tag: u8) -> TestResult<NodeId> {
    NodeId::decode(&v7(tag)).map_err(|error| format!("{error:?}").into())
}

fn scope(channel: u8) -> TestResult<RuntimeScopeRefV1> {
    let world = WorldId::decode(&v7(200)).map_err(|error| format!("{error:?}"))?;
    let channel = ChannelId::decode(&v7(channel)).map_err(|error| format!("{error:?}"))?;
    Ok(RuntimeScopeRefV1::channel(world, channel))
}

fn secret(tag: u8) -> BootstrapSecret {
    BootstrapSecret::from_bytes([tag; 32])
}

fn launch(tag: u8) -> TestResult<LaunchBinding> {
    LaunchBinding::new(&format!("launch-{tag}")).map_err(|error| format!("{error:?}").into())
}

fn key(tag: u8) -> OperationKey {
    OperationKey::from_bytes([tag; 32])
}

fn actor() -> TestResult<ControlActor> {
    ControlActor::new("operator.control-plane").map_err(|error| format!("{error:?}").into())
}

async fn register(
    root: &DurabilityRoot,
    tag: u8,
    supersedes: Option<NodeId>,
) -> TestResult<NodeRegistrationFact> {
    Ok(register_proof(root, tag, supersedes).await?.fact())
}

async fn register_proof(
    root: &DurabilityRoot,
    tag: u8,
    supersedes: Option<NodeId>,
) -> TestResult<NodeIncarnationProof> {
    root.issue_node_bootstrap_authorization(&secret(tag), &launch(tag)?, supersedes)
        .await
        .map_err(|error| format!("issue {tag}: {error:?}"))?;
    Ok(root
        .register_node_incarnation(&secret(tag), &launch(tag)?, node(tag)?)
        .await
        .map_err(|error| format!("register {tag}: {error:?}"))?)
}

fn request(tag: u8, command: AssignmentCommand) -> TestResult<AssignmentRequest> {
    Ok(AssignmentRequest {
        operation_key: key(tag),
        actor: actor()?,
        command,
    })
}

fn committed(outcome: Result<AssignmentOutcome, AssignmentError>) -> TestResult<AssignmentReceipt> {
    match outcome {
        Ok(AssignmentOutcome::Committed(receipt)) => Ok(receipt),
        other => Err(format!("expected committed assignment, got {other:?}").into()),
    }
}

fn rejected(
    outcome: Result<AssignmentOutcome, AssignmentError>,
) -> TestResult<AssignmentRejection> {
    match outcome {
        Ok(AssignmentOutcome::Rejected(rejection)) => Ok(rejection),
        other => Err(format!("expected rejected assignment, got {other:?}").into()),
    }
}

async fn high_water(pool: &sqlx::PgPool) -> TestResult<(String, String)> {
    Ok(sqlx::query_as(
        "SELECT (SELECT source_revision_high_water::text FROM game_runtime_scope_assignment_writer), \
                (SELECT registration_revision_high_water::text FROM game_node_registration_writer)",
    )
    .fetch_one(pool)
    .await?)
}

fn sql_state(error: &sqlx::Error) -> Option<String> {
    error
        .as_database_error()
        .and_then(|database| database.code())
        .map(|code| code.into_owned())
}

async fn expect_sql_state(
    result: Result<sqlx::postgres::PgQueryResult, sqlx::Error>,
    expected: &str,
) -> TestResult {
    match result {
        Err(error) if sql_state(&error).as_deref() == Some(expected) => Ok(()),
        other => Err(format!("expected SQLSTATE {expected}, got {other:?}").into()),
    }
}

async fn db_now(pool: &sqlx::PgPool) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(pool)
            .await?,
    )
}

// Test-only independently controlled Runtime publication owner.
struct RuntimePublisher(AdmissionAuthorityPublicationChangeV1);
impl foundation::fnd04_verifier::fresh_source_sealed::Sealed for RuntimePublisher {}
impl AdmissionAuthorityOwningPublisherV1 for RuntimePublisher {
    fn resolve_publication(
        &self,
        _now: i64,
    ) -> Result<Vec<AdmissionAuthorityPublicationChangeV1>, AdmissionAuthorityPublicationErrorV1>
    {
        Ok(vec![self.0.clone()])
    }
}

fn runtime_change(
    scope: RuntimeScopeRefV1,
    predecessor: Option<&AdmissionAuthorityPublicationChangeV1>,
    ownership_generation: u64,
    ready: bool,
    now: i64,
) -> AdmissionAuthorityPublicationChangeV1 {
    let (precondition, publication_revision, source_revision) = match predecessor {
        None => (
            AdmissionPublicationPreconditionV1::Bootstrap {
                restored_publication_high_water: Some(0),
            },
            1,
            1,
        ),
        Some(prior) => (
            AdmissionPublicationPreconditionV1::CompareAndSet {
                expected_publication_revision: prior.publication_revision,
            },
            prior.publication_revision + 1,
            prior.source.source_revision + 1,
        ),
    };
    AdmissionAuthorityPublicationChangeV1 {
        key: AdmissionAuthorityGuardKeyV1::Runtime(scope),
        source: AdmissionPublicationSourceV1 {
            authority: "game-runtime-publisher".into(),
            purpose: AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness,
            source_revision,
            decision_identity: format!("runtime-observation-{source_revision}"),
            source_observed_at: now,
            clock_uncertainty_seconds: 0,
        },
        precondition,
        publication_revision,
        state: AdmissionAuthorityGuardStateV1::Runtime {
            ownership_generation,
            ready,
            route_revision: "route-1".into(),
            runtime_observation_revision: format!("runtime-{source_revision}"),
            protocol_major: 1,
            transport_profile: 1,
            ruleset_revision: "rules-1".into(),
            content_revision: "content-1".into(),
            map_revision: "map-1".into(),
            world_policy_revision: "policy-1".into(),
            offer_revision: "offer-1".into(),
        },
    }
}

async fn publish_runtime(
    guards: &AdmissionGuardStore,
    change: AdmissionAuthorityPublicationChangeV1,
    now: i64,
) -> Result<GuardPublicationDisposition, DurabilityError> {
    let publication = AdmissionAuthorityPublicationV1::prepare(&RuntimePublisher(change), now)
        .map_err(|_| DurabilityError::Unavailable)?;
    guards.publish(&publication).await
}

async fn current_runtime(
    guards: &AdmissionGuardStore,
    scope: RuntimeScopeRefV1,
) -> TestResult<AdmissionAuthorityPublicationChangeV1> {
    guards
        .load(&[AdmissionAuthorityGuardKeyV1::Runtime(scope)])
        .await?
        .pop()
        .flatten()
        .ok_or_else(|| "runtime guard absent".into())
}

fn runtime_ready(change: &AdmissionAuthorityPublicationChangeV1) -> Option<(u64, bool)> {
    match &change.state {
        AdmissionAuthorityGuardStateV1::Runtime {
            ownership_generation,
            ready,
            ..
        } => Some((*ownership_generation, *ready)),
        _ => None,
    }
}

#[test]
fn registration_consumes_one_launch_authorization_and_tracks_current_incarnation() -> TestResult {
    run("registration", |database| {
        Box::pin(async move {
            let root = ready_root(&database.url).await?;
            let pool = database.pool().await?;

            // Fresh authorization + fresh UUIDv7 NodeId registration succeeds.
            let first = register_proof(&root, 1, None).await?;
            assert_eq!(first.fact().node_id(), node(1)?);
            assert_eq!(first.fact().registration_revision(), 1);
            root.require_current_node_registration(&first)
                .await
                .map_err(|e| format!("{e:?}"))?;

            // Lost response: the identical request reconciles the original result
            // without consuming or allocating anything new.
            let replay = root
                .register_node_incarnation(&secret(1), &launch(1)?, node(1)?)
                .await
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(replay, first);
            assert_eq!(high_water(&pool).await?.1, "1");

            // Reused authorization with a changed NodeId rejects.
            assert!(matches!(
                root.register_node_incarnation(&secret(1), &launch(1)?, node(2)?)
                    .await,
                Err(RegistrationError::Rejected)
            ));
            // Same secret with a changed launch binding rejects.
            root.issue_node_bootstrap_authorization(&secret(3), &launch(3)?, None)
                .await
                .map_err(|e| format!("{e:?}"))?;
            assert!(matches!(
                root.register_node_incarnation(&secret(3), &launch(4)?, node(3)?)
                    .await,
                Err(RegistrationError::Rejected)
            ));
            // Unknown/forged authorization rejects.
            assert!(matches!(
                root.register_node_incarnation(&secret(9), &launch(9)?, node(9)?)
                    .await,
                Err(RegistrationError::Rejected)
            ));
            // Duplicate/colliding NodeId under a fresh authorization rejects.
            assert!(matches!(
                root.register_node_incarnation(&secret(3), &launch(3)?, node(1)?)
                    .await,
                Err(RegistrationError::Rejected)
            ));
            // A launch binding is one-launch only.
            assert!(matches!(
                root.issue_node_bootstrap_authorization(&secret(5), &launch(3)?, None)
                    .await,
                Err(RegistrationError::Rejected)
            ));
            assert_eq!(high_water(&pool).await?.1, "1");

            // Nil/malformed/non-UUIDv7 NodeIds reject at the SQL boundary as well.
            for bad in [
                "00000000-0000-0000-0000-000000000000",
                "01890f4c-3b2a-4c03-8d11-9a321b7c0003",
            ] {
                let result = sqlx::query("SELECT game_node_register($1, 'launch-3', $2::uuid)")
                    .bind([3_u8; 32].as_slice())
                    .bind(bad)
                    .execute(&pool)
                    .await;
                expect_sql_state(result, "OTN01").await?;
            }

            // Wrong-incarnation fact is not current.
            let wrong = NodeRegistrationFact::new(
                first.fact().node_id(),
                first.fact().registration_revision() + 1,
            );
            assert!(matches!(
                root.require_current_node_registration(&NodeIncarnationProof::new(
                    wrong,
                    secret(1)
                ))
                .await,
                Err(RegistrationError::NotCurrent)
            ));

            // Public NodeId/revision without the incarnation's own secret proves nothing.
            assert!(matches!(
                root.require_current_node_registration(&NodeIncarnationProof::new(
                    first.fact(),
                    secret(2)
                ))
                .await,
                Err(RegistrationError::NotCurrent)
            ));

            // Restart: a new process gets a new NodeId under an explicit relaunch
            // authorization that supersedes the prior incarnation atomically.
            let second = register_proof(&root, 2, Some(first.fact().node_id())).await?;
            assert_eq!(second.fact().registration_revision(), 2);
            assert!(matches!(
                root.require_current_node_registration(&first).await,
                Err(RegistrationError::NotCurrent)
            ));
            root.require_current_node_registration(&second)
                .await
                .map_err(|e| format!("{e:?}"))?;

            // Explicit revoke; idempotent; superseded cannot be revoked back.
            root.revoke_node_registration(second.fact())
                .await
                .map_err(|e| format!("{e:?}"))?;
            root.revoke_node_registration(second.fact())
                .await
                .map_err(|e| format!("{e:?}"))?;
            assert!(matches!(
                root.require_current_node_registration(&second).await,
                Err(RegistrationError::NotCurrent)
            ));
            assert!(matches!(
                root.revoke_node_registration(first.fact()).await,
                Err(RegistrationError::Rejected)
            ));

            // Stored state cannot be rolled back to CURRENT, deleted or reused.
            expect_sql_state(
                sqlx::query("UPDATE game_node_registrations SET state = 1, ended_at = NULL, superseded_by = NULL WHERE registration_revision = 1")
                    .execute(&pool)
                    .await,
                "23514",
            )
            .await?;
            expect_sql_state(
                sqlx::query("DELETE FROM game_node_registrations")
                    .execute(&pool)
                    .await,
                "23514",
            )
            .await?;
            expect_sql_state(
                sqlx::query("UPDATE game_node_bootstrap_authorizations SET consumed_node_id = NULL, consumed_at = NULL")
                    .execute(&pool)
                    .await,
                "23514",
            )
            .await?;
            expect_sql_state(
                sqlx::query(
                    "UPDATE game_node_registration_writer SET registration_revision_high_water = 1",
                )
                .execute(&pool)
                .await,
                "23514",
            )
            .await?;
            // Only the digest of the launch secret is retained.
            let retained: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_node_bootstrap_authorizations WHERE authorization_digest = $1",
            )
            .bind([1_u8; 32].as_slice())
            .fetch_one(&pool)
            .await?;
            assert_eq!(retained, 0);

            pool.close().await;
            Ok(())
        })
    })
}

#[test]
fn channel_assignment_uses_exact_cas_replay_and_writer_owned_revisions() -> TestResult {
    run("assignment_cas", |database| {
        Box::pin(async move {
            let root = ready_root(&database.url).await?;
            let pool = database.pool().await?;
            let node1 = register(&root, 1, None).await?;
            let node2 = register(&root, 2, None).await?;
            let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "writer-a")
                .await
                .map_err(|e| format!("{e:?}"))?;
            let channel = scope(1)?;

            let assign = request(
                1,
                AssignmentCommand::Assign {
                    scope: channel,
                    target: node1,
                },
            )?;
            let first = committed(writer.submit(&assign).await)?;
            assert_eq!(first.assignment.ownership_generation, 1);
            assert_eq!(first.assignment.source_revision, 1);
            assert_eq!(
                first.assignment.decision_identity,
                "runtime-scope-assignment:1"
            );
            assert_eq!(first.assignment.state, AssignmentState::Assigned);
            assert_eq!(first.assignment.holder, Some(node1));
            assert_eq!(first.fenced_publication_revision, None);

            // Exact replay returns the original receipt without advancing anything.
            assert_eq!(committed(writer.submit(&assign).await)?, first);
            assert_eq!(high_water(&pool).await?.0, "1");
            // Changed command under the same identity conflicts.
            let changed = request(
                1,
                AssignmentCommand::Assign {
                    scope: channel,
                    target: node2,
                },
            )?;
            assert_eq!(
                rejected(writer.submit(&changed).await)?,
                AssignmentRejection::OperationConflict
            );
            // Initial assign over an existing record requires the replace CAS.
            let again = request(
                2,
                AssignmentCommand::Assign {
                    scope: channel,
                    target: node2,
                },
            )?;
            assert_eq!(
                rejected(writer.submit(&again).await)?,
                AssignmentRejection::PredecessorMismatch
            );
            // A caller cannot jump or choose the writer source revision/generation.
            for predecessor in [
                AssignmentPredecessor {
                    ownership_generation: 1,
                    source_revision: 5,
                    runtime_guard_publication_revision: None,
                },
                AssignmentPredecessor {
                    ownership_generation: 2,
                    source_revision: 1,
                    runtime_guard_publication_revision: None,
                },
            ] {
                let jumped = request(
                    3,
                    AssignmentCommand::Replace {
                        scope: channel,
                        predecessor,
                        target: node2,
                    },
                )?;
                assert_eq!(
                    rejected(writer.submit(&jumped).await)?,
                    AssignmentRejection::PredecessorMismatch
                );
            }
            assert_eq!(high_water(&pool).await?.0, "1");

            let replace = request(
                4,
                AssignmentCommand::Replace {
                    scope: channel,
                    predecessor: first.predecessor(),
                    target: node2,
                },
            )?;
            let second = committed(writer.submit(&replace).await)?;
            assert_eq!(
                (
                    second.assignment.ownership_generation,
                    second.assignment.source_revision
                ),
                (2, 2)
            );
            assert_eq!(second.assignment.holder, Some(node2));

            let revoke = request(
                5,
                AssignmentCommand::Revoke {
                    scope: channel,
                    predecessor: second.predecessor(),
                },
            )?;
            let third = committed(writer.submit(&revoke).await)?;
            assert_eq!(
                (
                    third.assignment.ownership_generation,
                    third.assignment.state
                ),
                (3, AssignmentState::Revoked)
            );
            assert_eq!(third.assignment.holder, None);
            let revoke_again = request(
                6,
                AssignmentCommand::Revoke {
                    scope: channel,
                    predecessor: third.predecessor(),
                },
            )?;
            assert_eq!(
                rejected(writer.submit(&revoke_again).await)?,
                AssignmentRejection::NotAssigned
            );
            // REVOKED is distinct from absence: it is re-assigned only through replace.
            let reassign = request(
                7,
                AssignmentCommand::Replace {
                    scope: channel,
                    predecessor: third.predecessor(),
                    target: node1,
                },
            )?;
            let fourth = committed(writer.submit(&reassign).await)?;
            assert_eq!(
                (
                    fourth.assignment.ownership_generation,
                    fourth.assignment.source_revision
                ),
                (4, 4)
            );
            assert_eq!(
                root.read_runtime_scope_assignment(channel)
                    .await
                    .map_err(|e| format!("{e:?}"))?,
                Some(fourth.assignment.clone())
            );
            assert_eq!(
                root.read_runtime_scope_assignment(scope(2)?)
                    .await
                    .map_err(|e| format!("{e:?}"))?,
                None
            );

            // A second Channel shares the writer namespace but has its own generation.
            let other = committed(
                writer
                    .submit(&request(
                        8,
                        AssignmentCommand::Assign {
                            scope: scope(2)?,
                            target: node2,
                        },
                    )?)
                    .await,
            )?;
            assert_eq!(
                (
                    other.assignment.ownership_generation,
                    other.assignment.source_revision
                ),
                (1, 5)
            );

            // Instance scope is unsupported for assign/replace/revoke/read.
            let instance = RuntimeScopeRefV1::instance(
                WorldId::decode(&v7(200)).map_err(|e| format!("{e:?}"))?,
                v7(99),
            )
            .map_err(|e| format!("{e:?}"))?;
            for command in [
                AssignmentCommand::Assign {
                    scope: instance,
                    target: node1,
                },
                AssignmentCommand::Replace {
                    scope: instance,
                    predecessor: fourth.predecessor(),
                    target: node1,
                },
                AssignmentCommand::Revoke {
                    scope: instance,
                    predecessor: fourth.predecessor(),
                },
            ] {
                assert!(matches!(
                    writer.submit(&request(9, command)?).await,
                    Err(AssignmentError::Unsupported)
                ));
            }
            assert!(matches!(
                root.read_runtime_scope_assignment(instance).await,
                Err(AssignmentError::Unsupported)
            ));
            assert_eq!(high_water(&pool).await?.0, "5");

            // Stored authority cannot regress, reuse a generation or be deleted.
            for sql in [
                "UPDATE game_runtime_scope_assignments SET ownership_generation = 1, source_revision = source_revision + 100",
                "UPDATE game_runtime_scope_assignments SET source_revision = 1, ownership_generation = ownership_generation + 1",
                "DELETE FROM game_runtime_scope_assignments",
                "UPDATE game_runtime_scope_assignment_writer SET source_revision_high_water = 1",
                "DELETE FROM game_runtime_scope_assignment_writer",
                "UPDATE game_runtime_scope_assignment_receipts SET state = 2",
                "DELETE FROM game_runtime_scope_assignment_receipts",
            ] {
                expect_sql_state(
                    sqlx::query(sqlx::AssertSqlSafe(sql)).execute(&pool).await,
                    "23514",
                )
                .await?;
            }
            pool.close().await;
            Ok(())
        })
    })
}

#[test]
fn initial_and_replacement_targets_must_be_current_registered_incarnations() -> TestResult {
    run("assignment_targets", |database| {
        Box::pin(async move {
            let root = ready_root(&database.url).await?;
            let pool = database.pool().await?;
            let old = register(&root, 1, None).await?;
            let current = register(&root, 2, Some(old.node_id())).await?;
            let revoked = register(&root, 3, None).await?;
            root.revoke_node_registration(revoked)
                .await
                .map_err(|e| format!("{e:?}"))?;
            let unregistered = NodeRegistrationFact::new(node(9)?, 1);
            let wrong_incarnation =
                NodeRegistrationFact::new(current.node_id(), current.registration_revision() + 1);
            let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "writer-a")
                .await
                .map_err(|e| format!("{e:?}"))?;
            let channel = scope(1)?;

            let mut tag = 10;
            for target in [unregistered, old, revoked, wrong_incarnation] {
                tag += 1;
                let initial = request(
                    tag,
                    AssignmentCommand::Assign {
                        scope: channel,
                        target,
                    },
                )?;
                assert_eq!(
                    rejected(writer.submit(&initial).await)?,
                    AssignmentRejection::TargetNotCurrent
                );
            }
            // Rejected before authority mutation: no record, no namespace advance.
            assert_eq!(high_water(&pool).await?.0, "0");
            assert_eq!(
                root.read_runtime_scope_assignment(channel)
                    .await
                    .map_err(|e| format!("{e:?}"))?,
                None
            );

            let assigned = committed(
                writer
                    .submit(&request(
                        20,
                        AssignmentCommand::Assign {
                            scope: channel,
                            target: current,
                        },
                    )?)
                    .await,
            )?;
            let other_current = register(&root, 4, None).await?;
            for target in [unregistered, old, revoked, wrong_incarnation] {
                tag += 1;
                let replacement = request(
                    tag,
                    AssignmentCommand::Replace {
                        scope: channel,
                        predecessor: assigned.predecessor(),
                        target,
                    },
                )?;
                assert_eq!(
                    rejected(writer.submit(&replacement).await)?,
                    AssignmentRejection::TargetNotCurrent
                );
            }
            assert_eq!(high_water(&pool).await?.0, "1");
            let replaced = committed(
                writer
                    .submit(&request(
                        30,
                        AssignmentCommand::Replace {
                            scope: channel,
                            predecessor: assigned.predecessor(),
                            target: other_current,
                        },
                    )?)
                    .await,
            )?;
            assert_eq!(replaced.assignment.holder, Some(other_current));
            assert_eq!(replaced.assignment.ownership_generation, 2);
            pool.close().await;
            Ok(())
        })
    })
}

async fn publish_ready(
    root: &DurabilityRoot,
    proof: &NodeIncarnationProof,
    change: AdmissionAuthorityPublicationChangeV1,
    now: i64,
) -> Result<GuardPublicationDisposition, AssignmentError> {
    let publication = AdmissionAuthorityPublicationV1::prepare(&RuntimePublisher(change), now)
        .map_err(|_| AssignmentError::InvalidInput)?;
    root.publish_runtime_readiness(proof, &publication).await
}

#[test]
fn assignment_mutations_atomically_fence_readiness_and_reject_stale_publication() -> TestResult {
    run("readiness_fence", |database| {
        Box::pin(async move {
            let root = ready_root(&database.url).await?;
            let pool = database.pool().await?;
            let guards = AdmissionGuardStore::from_root(root.clone());
            let channel = scope(1)?;
            let node1 = register_proof(&root, 1, None).await?;
            let node2 = register_proof(&root, 2, None).await?;
            let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "writer-a")
                .await
                .map_err(|e| format!("{e:?}"))?;

            // Pre-existing ready guard without an assignment record.
            let now = db_now(&pool).await?;
            assert_eq!(
                publish_runtime(&guards, runtime_change(channel, None, 1, true, now), now).await?,
                GuardPublicationDisposition::Applied
            );

            // Initial assignment atomically publishes ready=false.
            let first = committed(
                writer
                    .submit(&request(
                        1,
                        AssignmentCommand::Assign {
                            scope: channel,
                            target: node1.fact(),
                        },
                    )?)
                    .await,
            )?;
            assert_eq!(first.fenced_publication_revision, Some(2));
            let fenced = current_runtime(&guards, channel).await?;
            assert_eq!(runtime_ready(&fenced), Some((1, false)));
            assert_eq!(fenced.source.authority, "game-runtime-publisher");
            assert_eq!(fenced.source.source_revision, 2);
            assert_eq!(
                fenced.source.decision_identity,
                "runtime-scope-assignment:1"
            );

            // Once assigned, readiness needs the attested current holder: an
            // unattested publication, a current non-holder and a caller that knows
            // only the holder's public NodeId/revision are all refused.
            let now = db_now(&pool).await?;
            assert!(matches!(
                publish_runtime(
                    &guards,
                    runtime_change(channel, Some(&fenced), 1, true, now),
                    now
                )
                .await,
                Err(DurabilityError::Database(_))
            ));
            for impostor in [
                node2.clone(),
                NodeIncarnationProof::new(node1.fact(), secret(2)),
            ] {
                assert!(matches!(
                    publish_ready(
                        &root,
                        &impostor,
                        runtime_change(channel, Some(&fenced), 1, true, now),
                        now
                    )
                    .await,
                    Err(AssignmentError::NotCurrentHolder)
                ));
            }
            // The holder cannot publish a generation other than the current one.
            assert!(matches!(
                publish_ready(
                    &root,
                    &node1,
                    runtime_change(channel, Some(&fenced), 2, true, now),
                    now
                )
                .await,
                Err(AssignmentError::Unavailable(_))
            ));
            assert_eq!(
                publish_ready(
                    &root,
                    &node1,
                    runtime_change(channel, Some(&fenced), 1, true, now),
                    now
                )
                .await
                .map_err(|e| format!("{e:?}"))?,
                GuardPublicationDisposition::Applied
            );
            let ready = current_runtime(&guards, channel).await?;
            assert_eq!(runtime_ready(&ready), Some((1, true)));

            // The predecessor CAS binds the Runtime guard publication: a replace
            // authorized against the fenced publication cannot apply after the
            // guard advanced, and an unknown binding is rejected too.
            for stale in [
                first.predecessor(),
                AssignmentPredecessor {
                    runtime_guard_publication_revision: None,
                    ..first.predecessor()
                },
            ] {
                assert_eq!(
                    rejected(
                        writer
                            .submit(&request(
                                20,
                                AssignmentCommand::Replace {
                                    scope: channel,
                                    predecessor: stale,
                                    target: node2.fact(),
                                },
                            )?)
                            .await
                    )?,
                    AssignmentRejection::PredecessorMismatch
                );
            }
            let current = root
                .read_runtime_scope_predecessor(channel)
                .await
                .map_err(|e| format!("{e:?}"))?
                .ok_or("absent predecessor")?;
            assert_eq!(
                current.runtime_guard_publication_revision,
                Some(ready.publication_revision)
            );

            // Replace advances generation and moves the guard to the new
            // generation with readiness false in the same commit.
            let second = committed(
                writer
                    .submit(&request(
                        2,
                        AssignmentCommand::Replace {
                            scope: channel,
                            predecessor: current,
                            target: node2.fact(),
                        },
                    )?)
                    .await,
            )?;
            assert_eq!(
                second.fenced_publication_revision,
                Some(ready.publication_revision + 1)
            );
            let fenced = current_runtime(&guards, channel).await?;
            assert_eq!(runtime_ready(&fenced), Some((2, false)));
            // The replaced process keeps running with its genuine proof and learns
            // the new generation and holder's public identity: it still cannot
            // restore readiness for either generation.
            let now = db_now(&pool).await?;
            for (proof, generation) in [
                (node1.clone(), 1),
                (node1.clone(), 2),
                (NodeIncarnationProof::new(node2.fact(), secret(1)), 2),
            ] {
                assert!(matches!(
                    publish_ready(
                        &root,
                        &proof,
                        runtime_change(channel, Some(&fenced), generation, true, now),
                        now
                    )
                    .await,
                    Err(AssignmentError::NotCurrentHolder)
                ));
            }
            assert_eq!(
                runtime_ready(&current_runtime(&guards, channel).await?),
                Some((2, false))
            );
            // The replaced process cannot keep advancing the guard chain with
            // ready = false: its old generation is below the fenced guard
            // (typed CAS: Stale) and any generation other than the current
            // assignment generation is refused by the database fence.
            assert_eq!(
                publish_runtime(
                    &guards,
                    runtime_change(channel, Some(&fenced), 1, false, now),
                    now
                )
                .await?,
                GuardPublicationDisposition::Stale
            );
            assert!(matches!(
                publish_runtime(
                    &guards,
                    runtime_change(channel, Some(&fenced), 3, false, now),
                    now
                )
                .await,
                Err(DurabilityError::Database(_))
            ));
            // A non-ready observation for the current generation is permitted.
            assert_eq!(
                publish_runtime(
                    &guards,
                    runtime_change(channel, Some(&fenced), 2, false, now),
                    now
                )
                .await?,
                GuardPublicationDisposition::Applied
            );
            let closed = current_runtime(&guards, channel).await?;
            // A revoked holder registration cannot publish readiness either.
            root.revoke_node_registration(node2.fact())
                .await
                .map_err(|e| format!("{e:?}"))?;
            assert!(matches!(
                publish_ready(
                    &root,
                    &node2,
                    runtime_change(channel, Some(&closed), 2, true, now),
                    now
                )
                .await,
                Err(AssignmentError::NotCurrentHolder)
            ));
            // Revoke also advances generation; no readiness survives it.
            let node3 = register_proof(&root, 3, None).await?;
            let predecessor = root
                .read_runtime_scope_predecessor(channel)
                .await
                .map_err(|e| format!("{e:?}"))?
                .ok_or("absent predecessor")?;
            let third = committed(
                writer
                    .submit(&request(
                        3,
                        AssignmentCommand::Replace {
                            scope: channel,
                            predecessor,
                            target: node3.fact(),
                        },
                    )?)
                    .await,
            )?;
            assert_eq!(
                third.fenced_publication_revision,
                Some(closed.publication_revision + 1)
            );
            let fenced = current_runtime(&guards, channel).await?;
            assert_eq!(runtime_ready(&fenced), Some((3, false)));
            let now = db_now(&pool).await?;
            assert_eq!(
                publish_ready(
                    &root,
                    &node3,
                    runtime_change(channel, Some(&fenced), 3, true, now),
                    now
                )
                .await
                .map_err(|e| format!("{e:?}"))?,
                GuardPublicationDisposition::Applied
            );
            let ready = current_runtime(&guards, channel).await?;
            let predecessor = root
                .read_runtime_scope_predecessor(channel)
                .await
                .map_err(|e| format!("{e:?}"))?
                .ok_or("absent predecessor")?;
            let fourth = committed(
                writer
                    .submit(&request(
                        4,
                        AssignmentCommand::Revoke {
                            scope: channel,
                            predecessor,
                        },
                    )?)
                    .await,
            )?;
            assert_eq!(
                fourth.fenced_publication_revision,
                Some(ready.publication_revision + 1)
            );
            let revoked = current_runtime(&guards, channel).await?;
            assert_eq!(runtime_ready(&revoked), Some((4, false)));
            let now = db_now(&pool).await?;
            for generation in [3, 4] {
                assert!(matches!(
                    publish_ready(
                        &root,
                        &node3,
                        runtime_change(channel, Some(&revoked), generation, true, now),
                        now
                    )
                    .await,
                    Err(AssignmentError::NotCurrentHolder)
                ));
            }
            // Direct SQL cannot bypass the fence, and no attestation outlives its transaction.
            expect_sql_state(
                sqlx::query("UPDATE game_durability_admission_runtime_guards SET ready = TRUE")
                    .execute(&pool)
                    .await,
                "23514",
            )
            .await?;
            let attestations: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_runtime_readiness_attestations")
                    .fetch_one(&pool)
                    .await?;
            assert_eq!(attestations, 0);
            pool.close().await;
            Ok(())
        })
    })
}

#[test]
fn nasg_bounds_queue_inflight_and_operation_key_limits() -> TestResult {
    // Pure input bounds need no database.
    let text = key(7).to_text();
    assert_eq!(text.len(), 43);
    assert_eq!(OperationKey::from_text(&text).ok(), Some(key(7)));
    for bad in [
        &text[..42],
        &format!("{text}A"),
        &format!("{text}="),
        "++++++++++++++++++++++++++++++++++++++++++A",
    ] {
        assert!(matches!(
            OperationKey::from_text(bad),
            Err(AssignmentError::InvalidInput)
        ));
    }
    // Non-canonical trailing bits of the final character reject.
    let mut noncanonical = text.clone();
    noncanonical.pop();
    noncanonical.push('B');
    if noncanonical != text {
        assert!(matches!(
            OperationKey::from_text(&noncanonical),
            Err(AssignmentError::InvalidInput)
        ));
    }
    assert!(ControlActor::new(&"a".repeat(MAX_IDENTITY_BYTES)).is_ok());
    assert!(matches!(
        ControlActor::new(&"a".repeat(MAX_IDENTITY_BYTES + 1)),
        Err(AssignmentError::InvalidInput)
    ));
    assert!(matches!(
        ControlActor::new("bad actor"),
        Err(AssignmentError::InvalidInput)
    ));
    let max_actor = AssignmentRequest {
        operation_key: key(1),
        actor: ControlActor::new(&"a".repeat(MAX_IDENTITY_BYTES)).map_err(|e| format!("{e:?}"))?,
        command: AssignmentCommand::Replace {
            scope: scope(1)?,
            predecessor: AssignmentPredecessor {
                ownership_generation: u64::MAX,
                source_revision: u64::MAX,
                runtime_guard_publication_revision: Some(u64::MAX),
            },
            target: NodeRegistrationFact::new(node(1)?, u64::MAX),
        },
    };
    let encoded = max_actor.encode().map_err(|e| format!("{e:?}"))?;
    assert!(encoded.len() <= 1024);
    let zero = AssignmentRequest {
        command: AssignmentCommand::Revoke {
            scope: scope(1)?,
            predecessor: AssignmentPredecessor {
                ownership_generation: 0,
                source_revision: 1,
                runtime_guard_publication_revision: None,
            },
        },
        ..max_actor.clone()
    };
    assert!(matches!(zero.encode(), Err(AssignmentError::InvalidInput)));

    run("nasg_queue", |database| {
        Box::pin(async move {
            let root = ready_root(&database.url).await?;
            let pool = database.pool().await?;
            let target = register(&root, 1, None).await?;
            let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "writer-a")
                .await
                .map_err(|e| format!("{e:?}"))?;

            // Hold the writer namespace so the first dispatched operation blocks
            // inside PostgreSQL while it owns the single in-flight slot.
            let mut blocker = pool.begin().await?;
            sqlx::query("LOCK TABLE game_runtime_scope_assignment_writer IN EXCLUSIVE MODE")
                .execute(&mut *blocker)
                .await?;
            let first_writer = writer.clone();
            let first_request = request(
                1,
                AssignmentCommand::Assign {
                    scope: scope(1)?,
                    target,
                },
            )?;
            let first = tokio::spawn(async move { first_writer.submit(&first_request).await });
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            // Eight pending commands are retained; the ninth rejects before retention.
            let mut pending = Vec::new();
            for tag in 0..MAX_PENDING_COMMANDS {
                let waiting = writer.clone();
                let command = request(
                    u8::try_from(10 + tag)?,
                    AssignmentCommand::Assign {
                        scope: scope(u8::try_from(10 + tag)?)?,
                        target,
                    },
                )?;
                pending.push(tokio::spawn(async move { waiting.submit(&command).await }));
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            let ninth = request(
                30,
                AssignmentCommand::Assign {
                    scope: scope(30)?,
                    target,
                },
            )?;
            assert!(matches!(
                writer.submit(&ninth).await,
                Err(AssignmentError::QueueFull)
            ));
            // Pending commands time out after the bounded queue wait: no hidden waiter.
            for waiting in pending {
                assert!(matches!(waiting.await?, Err(AssignmentError::QueueTimeout)));
            }
            // The dispatched operation exceeds its budget: ambiguous, slot retained.
            assert!(matches!(first.await?, Err(AssignmentError::Ambiguous)));
            blocker.rollback().await?;
            assert_eq!(writer.unreconciled(), Some(key(1)));
            let slot: Option<Vec<u8>> = sqlx::query_scalar(
                "SELECT operation_key FROM game_runtime_scope_assignment_slots WHERE writer_registration = 'writer-a'",
            )
            .fetch_one(&pool)
            .await?;
            assert_eq!(slot.as_deref(), Some([1_u8; 32].as_slice()));

            // No new work, and no retry or replacement key, until reconciliation.
            ensure_ready(&root).await?;
            let blocked = request(
                2,
                AssignmentCommand::Assign {
                    scope: scope(2)?,
                    target,
                },
            )?;
            assert!(matches!(
                writer.submit(&blocked).await,
                Err(AssignmentError::ReconcileRequired)
            ));
            assert!(matches!(
                writer.reconcile(&blocked).await,
                Err(AssignmentError::ReconcileRequired)
            ));
            // A restarted writer restores the same slot custody.
            let restarted = RuntimeScopeAssignmentWriter::open(root.clone(), "writer-a")
                .await
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(restarted.unreconciled(), Some(key(1)));
            assert!(matches!(
                restarted.submit(&blocked).await,
                Err(AssignmentError::ReconcileRequired)
            ));

            // Under the slot lock an occupied matching slot proves non-commit.
            assert_eq!(
                restarted
                    .reconcile(&request(
                        1,
                        AssignmentCommand::Assign {
                            scope: scope(1)?,
                            target
                        }
                    )?)
                    .await
                    .map_err(|e| format!("{e:?}"))?,
                ReconcileOutcome::Absent
            );
            assert_eq!(restarted.unreconciled(), None);
            assert_eq!(high_water(&pool).await?.0, "0");
            // The same exact operation may now be resubmitted under its own key.
            let retried = committed(
                restarted
                    .submit(&request(
                        1,
                        AssignmentCommand::Assign {
                            scope: scope(1)?,
                            target,
                        },
                    )?)
                    .await,
            )?;
            assert_eq!(retried.assignment.source_revision, 1);
            assert_eq!(
                restarted
                    .reconcile(&request(
                        1,
                        AssignmentCommand::Assign {
                            scope: scope(1)?,
                            target
                        }
                    )?)
                    .await
                    .map_err(|e| format!("{e:?}"))?,
                ReconcileOutcome::Committed(retried)
            );
            pool.close().await;
            Ok(())
        })
    })
}

#[test]
fn concurrent_authorized_actors_from_one_predecessor_have_one_cas_winner() -> TestResult {
    run("assignment_race", |database| {
        Box::pin(async move {
            let root = ready_root(&database.url).await?;
            let node1 = register(&root, 1, None).await?;
            let node2 = register(&root, 2, None).await?;
            let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "writer-a")
                .await
                .map_err(|e| format!("{e:?}"))?;
            let first = committed(
                writer
                    .submit(&request(
                        1,
                        AssignmentCommand::Assign {
                            scope: scope(1)?,
                            target: node1,
                        },
                    )?)
                    .await,
            )?;

            let writer_b =
                RuntimeScopeAssignmentWriter::open(ready_root(&database.url).await?, "writer-b")
                    .await
                    .map_err(|e| format!("{e:?}"))?;
            let writer_c =
                RuntimeScopeAssignmentWriter::open(ready_root(&database.url).await?, "writer-c")
                    .await
                    .map_err(|e| format!("{e:?}"))?;
            let predecessor = first.predecessor();
            let request_b = request(
                2,
                AssignmentCommand::Replace {
                    scope: scope(1)?,
                    predecessor,
                    target: node2,
                },
            )?;
            let request_c = request(
                3,
                AssignmentCommand::Revoke {
                    scope: scope(1)?,
                    predecessor,
                },
            )?;
            let b = tokio::spawn(async move { writer_b.submit(&request_b).await });
            let c = tokio::spawn(async move { writer_c.submit(&request_c).await });
            let outcomes = [
                b.await?.map_err(|e| format!("{e:?}"))?,
                c.await?.map_err(|e| format!("{e:?}"))?,
            ];
            let winners = outcomes
                .iter()
                .filter(|outcome| matches!(outcome, AssignmentOutcome::Committed(_)))
                .count();
            let losers = outcomes
                .iter()
                .filter(|outcome| {
                    matches!(
                        outcome,
                        AssignmentOutcome::Rejected(AssignmentRejection::PredecessorMismatch)
                    )
                })
                .count();
            assert_eq!((winners, losers), (1, 1));
            let current = root
                .read_runtime_scope_assignment(scope(1)?)
                .await
                .map_err(|e| format!("{e:?}"))?
                .ok_or("absent")?;
            assert_eq!(
                (current.ownership_generation, current.source_revision),
                (2, 2)
            );
            Ok(())
        })
    })
}

#[test]
fn rollback_after_each_tentative_effect_and_lost_commit_reconcile_exactly() -> TestResult {
    run("assignment_rollback", |database| {
        Box::pin(async move {
            let pool = database.pool().await?;
            let root = ready_root(&database.url).await?;
            let guards = AdmissionGuardStore::from_root(root.clone());
            let node1 = register(&root, 1, None).await?;
            let channel = scope(1)?;
            let now = db_now(&pool).await?;
            publish_runtime(&guards, runtime_change(channel, None, 1, true, now), now).await?;
            let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "writer-a")
                .await
                .map_err(|e| format!("{e:?}"))?;
            sqlx::raw_sql(
                "CREATE FUNCTION inject_assignment_failure() RETURNS trigger LANGUAGE plpgsql AS $$ \
                 BEGIN RAISE EXCEPTION 'injected assignment failure' USING ERRCODE = 'P0001'; END $$;",
            )
            .execute(&pool)
            .await?;
            for table in [
                "game_durability_admission_runtime_guards",
                "game_durability_admission_guard_history",
                "game_runtime_scope_assignments",
                "game_runtime_scope_assignment_writer",
                "game_runtime_scope_assignment_receipts",
            ] {
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "CREATE TRIGGER injected_assignment_failure BEFORE INSERT OR UPDATE ON {table} \
                     FOR EACH ROW EXECUTE FUNCTION inject_assignment_failure()"
                )))
                .execute(&pool)
                .await?;
                let command = request(
                    1,
                    AssignmentCommand::Assign {
                        scope: channel,
                        target: node1,
                    },
                )?;
                assert!(
                    matches!(
                        writer.submit(&command).await,
                        Err(AssignmentError::Ambiguous)
                    ),
                    "{table}"
                );
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "DROP TRIGGER injected_assignment_failure ON {table}"
                )))
                .execute(&pool)
                .await?;
                ensure_ready(&root).await?;
                // Nothing tentative became authority; the slot proves non-commit.
                assert_eq!(high_water(&pool).await?.0, "0", "{table}");
                assert_eq!(
                    root.read_runtime_scope_assignment(channel)
                        .await
                        .map_err(|e| format!("{e:?}"))?,
                    None
                );
                assert_eq!(
                    runtime_ready(&current_runtime(&guards, channel).await?),
                    Some((1, true)),
                    "{table}"
                );
                assert_eq!(
                    writer
                        .reconcile(&command)
                        .await
                        .map_err(|e| format!("{e:?}"))?,
                    ReconcileOutcome::Absent
                );
            }

            // Commit-time failure after every effect is tentative: deferred trigger.
            sqlx::raw_sql(
                "CREATE CONSTRAINT TRIGGER injected_commit_failure AFTER INSERT ON game_runtime_scope_assignment_receipts \
                 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION inject_assignment_failure();",
            )
            .execute(&pool)
            .await?;
            let command = request(
                1,
                AssignmentCommand::Assign {
                    scope: channel,
                    target: node1,
                },
            )?;
            assert!(matches!(
                writer.submit(&command).await,
                Err(AssignmentError::Ambiguous)
            ));
            sqlx::raw_sql(
                "DROP TRIGGER injected_commit_failure ON game_runtime_scope_assignment_receipts;",
            )
            .execute(&pool)
            .await?;
            ensure_ready(&root).await?;
            assert_eq!(
                writer
                    .reconcile(&command)
                    .await
                    .map_err(|e| format!("{e:?}"))?,
                ReconcileOutcome::Absent
            );

            // Lost response after a real commit reconciles to the exact receipt.
            let receipt = committed(writer.submit(&command).await)?;
            assert_eq!(
                writer
                    .reconcile(&command)
                    .await
                    .map_err(|e| format!("{e:?}"))?,
                ReconcileOutcome::Committed(receipt.clone())
            );
            assert_eq!(
                runtime_ready(&current_runtime(&guards, channel).await?),
                Some((1, false))
            );
            assert_eq!(high_water(&pool).await?.0, "1");
            pool.close().await;
            Ok(())
        })
    })
}

#[test]
fn restart_preserves_high_water_and_fails_closed_on_regression_or_overflow() -> TestResult {
    run("assignment_restart", |database| {
        Box::pin(async move {
            let pool = database.pool().await?;
            let channel = scope(1)?;
            let (node1, node2, first) = {
                let root = ready_root(&database.url).await?;
                let node1 = register(&root, 1, None).await?;
                let node2 = register(&root, 2, None).await?;
                let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "writer-a")
                    .await
                    .map_err(|e| format!("{e:?}"))?;
                let first = committed(
                    writer
                        .submit(&request(
                            1,
                            AssignmentCommand::Assign {
                                scope: channel,
                                target: node1,
                            },
                        )?)
                        .await,
                )?;
                (node1, node2, first)
            };

            // New process/root: current assignment, receipts and writer high-water survive.
            let root = ready_root(&database.url).await?;
            let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "writer-a")
                .await
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(writer.unreconciled(), None);
            assert_eq!(
                root.read_runtime_scope_assignment(channel)
                    .await
                    .map_err(|e| format!("{e:?}"))?,
                Some(first.assignment.clone())
            );
            assert_eq!(
                committed(
                    writer
                        .submit(&request(
                            1,
                            AssignmentCommand::Assign {
                                scope: channel,
                                target: node1
                            }
                        )?)
                        .await
                )?,
                first
            );
            // A registration of a former NodeId cannot be re-created by restart.
            assert!(matches!(
                root.register_node_incarnation(&secret(5), &launch(5)?, node1.node_id())
                    .await,
                Err(RegistrationError::Rejected)
            ));

            // Injected regression of the writer namespace fails closed.
            let mut corrupt = pool.begin().await?;
            sqlx::query("ALTER TABLE game_runtime_scope_assignment_writer DISABLE TRIGGER game_runtime_scope_assignment_writer_guard")
                .execute(&mut *corrupt)
                .await?;
            sqlx::query(
                "UPDATE game_runtime_scope_assignment_writer SET source_revision_high_water = 0",
            )
            .execute(&mut *corrupt)
            .await?;
            sqlx::query("ALTER TABLE game_runtime_scope_assignment_writer ENABLE TRIGGER game_runtime_scope_assignment_writer_guard")
                .execute(&mut *corrupt)
                .await?;
            corrupt.commit().await?;
            assert!(matches!(
                root.read_runtime_scope_assignment(channel).await,
                Err(AssignmentError::Unavailable(
                    DurabilityError::InvalidStoredState
                ))
            ));
            let replace = request(
                2,
                AssignmentCommand::Replace {
                    scope: channel,
                    predecessor: first.predecessor(),
                    target: node2,
                },
            )?;
            assert!(matches!(
                writer.submit(&replace).await,
                Err(AssignmentError::Ambiguous)
            ));
            ensure_ready(&root).await?;
            // Regressed history cannot prove non-commit: reconciliation fails
            // closed until the retained maximum is restored.
            assert!(matches!(
                writer.reconcile(&replace).await,
                Err(AssignmentError::Ambiguous)
            ));
            ensure_ready(&root).await?;
            // Restore the retained maximum through the ordinary monotonic guard.
            sqlx::query(
                "UPDATE game_runtime_scope_assignment_writer SET source_revision_high_water = 1",
            )
            .execute(&pool)
            .await?;
            assert_eq!(
                writer
                    .reconcile(&replace)
                    .await
                    .map_err(|e| format!("{e:?}"))?,
                ReconcileOutcome::Absent
            );

            // A sparse history (receipt 1 plus a receipt at the maximum, with
            // the high-water at the maximum) is invalid retained history, not a
            // valid overflow fixture: the continuity check fails closed before
            // any successor revision is allocated. Checked successor overflow
            // itself is covered by `source_revision_successor_is_checked`.
            sqlx::query(
                "INSERT INTO game_runtime_scope_assignment_receipts \
                 (operation_key, command, scope_key, ownership_generation, state, holder_node_id, \
                  holder_registration_revision, source_revision, decision_identity, decided_at) \
                 SELECT '\\x99'::bytea || substring(operation_key FROM 2), command, scope_key, ownership_generation, \
                        state, holder_node_id, holder_registration_revision, 18446744073709551615, \
                        'runtime-scope-assignment:max', decided_at \
                 FROM game_runtime_scope_assignment_receipts WHERE source_revision = 1",
            )
            .execute(&pool)
            .await?;
            sqlx::query("UPDATE game_runtime_scope_assignment_writer SET source_revision_high_water = 18446744073709551615")
                .execute(&pool)
                .await?;
            assert!(matches!(
                writer.submit(&replace).await,
                Err(AssignmentError::Ambiguous)
            ));
            assert_eq!(high_water(&pool).await?.0, "18446744073709551615");
            assert!(matches!(
                root.read_runtime_scope_assignment(channel).await,
                Err(AssignmentError::Unavailable(
                    DurabilityError::InvalidStoredState
                ))
            ));
            let current: (String, String) = sqlx::query_as(
                "SELECT ownership_generation::text, source_revision::text \
                 FROM game_runtime_scope_assignments",
            )
            .fetch_one(&pool)
            .await?;
            assert_eq!(current, ("1".to_owned(), "1".to_owned()));

            let database2 = Database::create("generation_overflow").await?;
            let overflow = async {
                let root = ready_root(&database2.url).await?;
                let pool2 = database2.pool().await?;
                let node = register(&root, 1, None).await?;
                let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "writer-a")
                    .await
                    .map_err(|e| format!("{e:?}"))?;
                let first = committed(writer.submit(&request(1, AssignmentCommand::Assign { scope: channel, target: node })?).await)?;
                sqlx::query("UPDATE game_runtime_scope_assignments SET ownership_generation = 18446744073709551615, source_revision = 2, decision_identity = 'runtime-scope-assignment:2', operation_key = '\\x98'::bytea || substring(operation_key FROM 2)")
                    .execute(&pool2)
                    .await?;
                sqlx::query(
                    "INSERT INTO game_runtime_scope_assignment_receipts \
                     (operation_key, command, scope_key, ownership_generation, state, holder_node_id, \
                      holder_registration_revision, source_revision, decision_identity, decided_at) \
                     SELECT '\\x98'::bytea || substring(operation_key FROM 2), command, scope_key, 18446744073709551615, \
                            state, holder_node_id, holder_registration_revision, 2, \
                            'runtime-scope-assignment:2', decided_at \
                     FROM game_runtime_scope_assignment_receipts WHERE source_revision = 1",
                )
                .execute(&pool2)
                .await?;
                sqlx::query("UPDATE game_runtime_scope_assignment_writer SET source_revision_high_water = 2")
                    .execute(&pool2)
                    .await?;
                let predecessor = AssignmentPredecessor { ownership_generation: u64::MAX, source_revision: 2, runtime_guard_publication_revision: None };
                assert_eq!(
                    rejected(writer.submit(&request(2, AssignmentCommand::Revoke { scope: channel, predecessor })?).await)?,
                    AssignmentRejection::GenerationExhausted
                );
                assert_eq!(high_water(&pool2).await?.0, "2");
                let _ = first;
                pool2.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database2.cleanup(&[]).await?;
            overflow?;
            pool.close().await;
            Ok(())
        })
    })
}

#[test]
fn ordinary_gamenode_role_cannot_mutate_assignment_or_registration_authority() -> TestResult {
    if !configured() {
        skipped();
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let database = Database::create("privileges").await?;
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let runtime_role = format!("rsa_gamenode_{suffix}");
        let result = async {
            let pool = database.pool().await?;
            let root = ready_root(&database.url).await?;
            root.issue_node_bootstrap_authorization(&secret(1), &launch(1)?, None)
                .await
                .map_err(|e| format!("{e:?}"))?;
            let function_privileges: Vec<(String, bool, bool)> = sqlx::query_as(
                "WITH expected(signature) AS (VALUES \
                    ('game_node_is_uuid_v7(uuid)'), \
                    ('game_node_registration_writer_guard()'), \
                    ('game_node_bootstrap_authorization_guard()'), \
                    ('game_node_registration_guard()'), \
                    ('game_node_registration_ending_guard()'), \
                    ('game_node_registration_history_valid()'), \
                    ('game_node_end_registration(uuid,numeric,smallint,uuid)'), \
                    ('game_node_register(bytea,text,uuid)'), \
                    ('game_node_lock_current_registration(uuid,numeric)'), \
                    ('game_node_prove_current_incarnation(uuid,numeric,bytea)'), \
                    ('game_node_require_current(uuid,numeric,bytea)'), \
                    ('game_runtime_scope_assignment_writer_guard()'), \
                    ('game_runtime_scope_assignment_guard()'), \
                    ('game_runtime_scope_assignment_history_valid()'), \
                    ('game_runtime_scope_assignment_slot_guard()'), \
                    ('game_runtime_attest_readiness(bytea,uuid,numeric,bytea)'), \
                    ('game_runtime_guard_requires_current_assignment()')) \
                 SELECT signature, p.oid IS NOT NULL AS function_exists, EXISTS ( \
                     SELECT 1 \
                     FROM aclexplode(coalesce(p.proacl, acldefault('f', p.proowner))) acl \
                     WHERE acl.grantee = 0 AND acl.privilege_type = 'EXECUTE' \
                 ) AS public_can_execute \
                 FROM expected \
                 LEFT JOIN pg_proc p ON p.oid = to_regprocedure(signature) \
                 ORDER BY signature",
            )
            .fetch_all(&pool)
            .await?;
            assert_eq!(function_privileges.len(), 17);
            for (signature, function_exists, public_can_execute) in function_privileges {
                assert!(function_exists, "migration function is missing: {signature}");
                assert!(
                    !public_can_execute,
                    "PUBLIC retains EXECUTE on migration function: {signature}"
                );
            }
            sqlx::query(sqlx::AssertSqlSafe(format!("CREATE ROLE {runtime_role} NOLOGIN"))).execute(&pool).await?;
            // Least-privilege GameNode consumer: read current assignment, register
            // itself and check currentness through definer functions only.
            for statement in [
                format!("GRANT SELECT ON game_runtime_scope_assignments TO {runtime_role}"),
                format!("GRANT EXECUTE ON FUNCTION game_node_register(BYTEA, TEXT, UUID) TO {runtime_role}"),
                format!("GRANT EXECUTE ON FUNCTION game_node_require_current(UUID, NUMERIC, BYTEA) TO {runtime_role}"),
                format!("GRANT EXECUTE ON FUNCTION game_runtime_attest_readiness(BYTEA, UUID, NUMERIC, BYTEA) TO {runtime_role}"),
            ] {
                sqlx::query(sqlx::AssertSqlSafe(statement)).execute(&pool).await?;
            }
            let mut connection = pool.acquire().await?;
            sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE {runtime_role}"))).execute(&mut *connection).await?;
            // Registration through the definer boundary works for the GameNode.
            let revision: String = sqlx::query_scalar("SELECT game_node_register($1, 'launch-1', $2::uuid)::text")
                .bind([1_u8; 32].as_slice())
                .bind("01890f4c-3b2a-7c01-8d11-9a321b7c0001")
                .fetch_one(&mut *connection)
                .await?;
            assert_eq!(revision, "1");
            sqlx::query(
                "SELECT game_node_require_current('01890f4c-3b2a-7c01-8d11-9a321b7c0001'::uuid, 1, $1)",
            )
            .bind([1_u8; 32].as_slice())
                .execute(&mut *connection)
                .await?;
            // But it cannot write, allocate or self-grant authority.
            for sql in [
                "INSERT INTO game_runtime_scope_assignments (scope_key, world_id, channel_id, ownership_generation, state, holder_node_id, holder_registration_revision, source_revision, decision_identity, operation_key, decided_at) VALUES ('\\x01'::bytea, gen_random_uuid(), gen_random_uuid(), 1, 1, NULL, NULL, 1, 'x', '\\x00'::bytea, 0)",
                "UPDATE game_runtime_scope_assignments SET state = 2",
                "DELETE FROM game_runtime_scope_assignments",
                "UPDATE game_runtime_scope_assignment_writer SET source_revision_high_water = 99",
                "INSERT INTO game_runtime_scope_assignment_receipts (operation_key) VALUES ('\\x00'::bytea)",
                "UPDATE game_runtime_scope_assignment_slots SET operation_key = NULL",
                "UPDATE game_node_registrations SET state = 2",
                "INSERT INTO game_node_bootstrap_authorizations (authorization_digest, launch_binding, issued_at) VALUES (sha256('\\x02'::bytea), 'self', 0)",
                "UPDATE game_node_registration_writer SET registration_revision_high_water = 99",
            ] {
                expect_sql_state(sqlx::query(sqlx::AssertSqlSafe(sql)).execute(&mut *connection).await, "42501").await?;
            }
            // The internal lock primitive is not executable by PUBLIC.
            expect_sql_state(
                sqlx::query("SELECT game_node_lock_current_registration('01890f4c-3b2a-7c01-8d11-9a321b7c0001'::uuid, 1)")
                    .execute(&mut *connection)
                    .await,
                "42501",
            )
            .await?;
            expect_sql_state(
                sqlx::query("SELECT game_runtime_scope_assignment_history_valid()")
                    .execute(&mut *connection)
                    .await,
                "42501",
            )
            .await?;
            sqlx::query("RESET ROLE").execute(&mut *connection).await?;
            drop(connection);
            pool.close().await;
            Ok::<(), Box<dyn std::error::Error>>(())
        }
        .await;
        database.cleanup(&[runtime_role.as_str()]).await?;
        result
    })
}

#[test]
fn restricted_assignment_writer_can_mutate_and_read_authoritative_state() -> TestResult {
    if !configured() {
        skipped();
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = Database::create("assignment_writer_privileges").await?;
            let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
            let writer_role = format!("rsa_assignment_writer_{suffix}");
            let writer_password = format!("assignment-writer-{suffix}");
            let result = async {
            let pool = database.pool().await?;
            let owner_root = ready_root(&database.url).await?;
            let target = register(&owner_root, 1, None).await?;
            let channel = scope(1)?;
            let now = db_now(&pool).await?;
            let guards = AdmissionGuardStore::from_root(owner_root.clone());
            publish_runtime(&guards, runtime_change(channel, None, 1, true, now), now).await?;

            sqlx::query(sqlx::AssertSqlSafe(format!(
                "CREATE ROLE {writer_role} LOGIN PASSWORD '{writer_password}'"
            )))
            .execute(&pool)
            .await?;

            // This is the minimum deployment grant contract for the existing
            // assignment writer: schema/ledger inspection, relation locks,
            // assignment state mutation, the Runtime readiness fence, and the
            // two deliberately non-PUBLIC internal function boundaries. The
            // migration creates no production role or credential.
            for statement in [
                format!("GRANT USAGE ON SCHEMA public TO {writer_role}"),
                format!("GRANT SELECT ON _sqlx_migrations TO {writer_role}"),
                format!(
                    "GRANT MAINTAIN ON TABLE \
                     game_durability_admission_account_guards, \
                     game_durability_admission_character_guards, \
                     game_durability_admission_guard_history, \
                     game_durability_admission_lifecycle_receipts, \
                     game_durability_admission_signing_trust_guards, \
                     game_durability_control_loss_continuity, \
                     game_durability_executor_custody, \
                     game_durability_fresh_admission_receipts, \
                     game_durability_reconnect_attempts, \
                     game_durability_reconnect_pending_commands, \
                     game_durability_reconnect_sessions, \
                     game_durability_recovery_grant_consumptions, \
                     game_durability_session_replacements, \
                     game_durability_session_use_ledgers, \
                     game_durability_session_use_memberships, \
                     game_durability_transport_ref_reservations TO {writer_role}"
                ),
                format!(
                    "GRANT SELECT, INSERT, UPDATE ON \
                     game_runtime_scope_assignment_slots, \
                     game_runtime_scope_assignments TO {writer_role}"
                ),
                format!(
                    "GRANT SELECT, UPDATE ON game_runtime_scope_assignment_writer TO {writer_role}"
                ),
                format!(
                    "GRANT SELECT, INSERT ON \
                     game_runtime_scope_assignment_receipts, \
                     game_durability_admission_guard_history TO {writer_role}"
                ),
                format!(
                    "GRANT SELECT, INSERT, UPDATE ON \
                     game_durability_admission_runtime_guards TO {writer_role}"
                ),
                format!(
                    "GRANT EXECUTE ON FUNCTION \
                     game_node_is_uuid_v7(UUID), \
                     game_node_lock_current_registration(UUID, NUMERIC), \
                     game_runtime_scope_assignment_history_valid() TO {writer_role}"
                ),
            ] {
                sqlx::query(sqlx::AssertSqlSafe(statement))
                    .execute(&pool)
                    .await?;
            }

            let (_, address) = database
                .url
                .split_once('@')
                .ok_or("database URL has no authority separator")?;
            let writer_url = format!(
                "postgresql://{writer_role}:{writer_password}@{address}"
            );
            let writer_root = ready_root(&writer_url).await?;
            let writer = RuntimeScopeAssignmentWriter::open(writer_root.clone(), "writer-a")
                .await
                .map_err(|error| format!("restricted writer open: {error:?}"))?;
            let receipt = committed(
                writer
                    .submit(&request(
                        1,
                        AssignmentCommand::Assign {
                            scope: channel,
                            target,
                        },
                    )?)
                    .await,
            )?;
            assert_eq!(receipt.assignment.source_revision, 1);

            let current = writer_root
                .read_runtime_scope_assignment(channel)
                .await
                .map_err(|error| format!("restricted authoritative read: {error:?}"))?
                .ok_or("restricted authoritative read returned no assignment")?;
            assert_eq!(current, receipt.assignment);
            let ready: bool = sqlx::query_scalar(
                "SELECT ready FROM game_durability_admission_runtime_guards",
            )
            .fetch_one(&pool)
            .await?;
            assert!(!ready, "restricted writer did not apply the readiness fence");
            drop(writer);
            drop(writer_root);
            pool.close().await;
            Ok::<(), Box<dyn std::error::Error>>(())
        }
        .await;
            database.cleanup(&[writer_role.as_str()]).await?;
            result
        })
}

#[test]
fn database_outage_cannot_make_cached_state_sufficient() -> TestResult {
    run("assignment_outage", |database| {
        Box::pin(async move {
            let root = ready_root(&database.url).await?;
            let node1 = register_proof(&root, 1, None).await?;
            let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "writer-a")
                .await
                .map_err(|e| format!("{e:?}"))?;
            committed(
                writer
                    .submit(&request(
                        1,
                        AssignmentCommand::Assign {
                            scope: scope(1)?,
                            target: node1.fact(),
                        },
                    )?)
                    .await,
            )?;

            // Terminate the writer's only database session: no cached authority
            // permits a new mutation, read or currentness proof without the database.
            let admin = sqlx::PgPool::connect(&database.admin_url).await?;
            sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = $1 AND pid <> pg_backend_pid()")
                .bind(&database.name)
                .execute(&admin)
                .await?;
            admin.close().await;
            let node2 = NodeRegistrationFact::new(node(2)?, 2);
            let outcome = writer
                .submit(&request(
                    2,
                    AssignmentCommand::Replace {
                        scope: scope(1)?,
                        predecessor: AssignmentPredecessor {
                            ownership_generation: 1,
                            source_revision: 1,
                            runtime_guard_publication_revision: None,
                        },
                        target: node2,
                    },
                )?)
                .await;
            assert!(
                matches!(
                    outcome,
                    Err(AssignmentError::Ambiguous | AssignmentError::Unavailable(_))
                ),
                "{outcome:?}"
            );
            assert!(root.read_runtime_scope_assignment(scope(1)?).await.is_err());
            assert!(matches!(
                root.require_current_node_registration(&node1).await,
                Err(RegistrationError::Unavailable(_))
            ));
            Ok(())
        })
    })
}

#[test]
fn history_trailing_the_high_water_fails_closed() -> TestResult {
    run("history_trailing", |database| {
        Box::pin(async move {
            let root = ready_root(&database.url).await?;
            let pool = database.pool().await?;
            let node1 = register(&root, 1, None).await?;
            let node2 = register(&root, 2, None).await?;
            let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "writer-a")
                .await
                .map_err(|e| format!("{e:?}"))?;
            let first = committed(
                writer
                    .submit(&request(
                        1,
                        AssignmentCommand::Assign {
                            scope: scope(1)?,
                            target: node1,
                        },
                    )?)
                    .await,
            )?;
            committed(
                writer
                    .submit(&request(
                        2,
                        AssignmentCommand::Assign {
                            scope: scope(2)?,
                            target: node2,
                        },
                    )?)
                    .await,
            )?;
            // A partial restore rolls back the newest decision's evidence while the
            // writer high-water survives: authority must not resume across it.
            let mut restore = pool.begin().await?;
            for sql in [
                "ALTER TABLE game_runtime_scope_assignment_receipts DISABLE TRIGGER game_runtime_scope_assignment_receipt_immutable",
                "ALTER TABLE game_runtime_scope_assignments DISABLE TRIGGER game_runtime_scope_assignment_guard",
                "DELETE FROM game_runtime_scope_assignment_receipts WHERE source_revision = 2",
                "DELETE FROM game_runtime_scope_assignments WHERE source_revision = 2",
                "ALTER TABLE game_runtime_scope_assignment_receipts ENABLE TRIGGER game_runtime_scope_assignment_receipt_immutable",
                "ALTER TABLE game_runtime_scope_assignments ENABLE TRIGGER game_runtime_scope_assignment_guard",
            ] {
                sqlx::query(sqlx::AssertSqlSafe(sql))
                    .execute(&mut *restore)
                    .await?;
            }
            restore.commit().await?;
            assert_eq!(high_water(&pool).await?.0, "2");
            assert!(matches!(
                root.read_runtime_scope_assignment(scope(1)?).await,
                Err(AssignmentError::Unavailable(
                    DurabilityError::InvalidStoredState
                ))
            ));
            let replace = request(
                3,
                AssignmentCommand::Replace {
                    scope: scope(1)?,
                    predecessor: first.predecessor(),
                    target: node2,
                },
            )?;
            assert!(matches!(
                writer.submit(&replace).await,
                Err(AssignmentError::Ambiguous)
            ));
            ensure_ready(&root).await?;
            // Trailing history cannot prove non-commit: reconciliation fails closed.
            assert!(matches!(
                writer.reconcile(&replace).await,
                Err(AssignmentError::Ambiguous)
            ));
            ensure_ready(&root).await?;
            assert_eq!(high_water(&pool).await?.0, "2");

            // The same exact-equality rule protects the registration namespace.
            let mut restore = pool.begin().await?;
            for sql in [
                "ALTER TABLE game_node_registrations DISABLE TRIGGER game_node_registration_guard",
                "ALTER TABLE game_node_bootstrap_authorizations DISABLE TRIGGER game_node_bootstrap_authorization_guard",
                "UPDATE game_node_bootstrap_authorizations SET consumed_node_id = NULL, consumed_at = NULL WHERE launch_binding = 'launch-2'",
                "DELETE FROM game_node_registrations WHERE registration_revision = 2",
                "ALTER TABLE game_node_registrations ENABLE TRIGGER game_node_registration_guard",
                "ALTER TABLE game_node_bootstrap_authorizations ENABLE TRIGGER game_node_bootstrap_authorization_guard",
            ] {
                sqlx::query(sqlx::AssertSqlSafe(sql))
                    .execute(&mut *restore)
                    .await?;
            }
            restore.commit().await?;
            root.issue_node_bootstrap_authorization(&secret(5), &launch(5)?, None)
                .await
                .map_err(|e| format!("{e:?}"))?;
            assert!(matches!(
                root.register_node_incarnation(&secret(5), &launch(5)?, node(5)?)
                    .await,
                Err(RegistrationError::Unavailable(_))
            ));
            assert_eq!(high_water(&pool).await?.1, "2");
            pool.close().await;
            Ok(())
        })
    })
}

#[test]
fn per_scope_restore_fails_reads_mutations_and_readiness() -> TestResult {
    run("per_scope_history", |database| {
        Box::pin(async move {
            let root = ready_root(&database.url).await?;
            let pool = database.pool().await?;
            let guards = AdmissionGuardStore::from_root(root.clone());
            let a = scope(1)?;
            let b = scope(2)?;
            let node1 = register_proof(&root, 1, None).await?;
            let node2 = register_proof(&root, 2, None).await?;
            let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "writer-a")
                .await
                .map_err(|e| format!("{e:?}"))?;

            let now = db_now(&pool).await?;
            assert_eq!(
                publish_runtime(&guards, runtime_change(a, None, 1, true, now), now).await?,
                GuardPublicationDisposition::Applied
            );
            let first = committed(
                writer
                    .submit(&request(
                        1,
                        AssignmentCommand::Assign {
                            scope: a,
                            target: node1.fact(),
                        },
                    )?)
                    .await,
            )?;
            committed(
                writer
                    .submit(&request(
                        2,
                        AssignmentCommand::Replace {
                            scope: a,
                            predecessor: first.predecessor(),
                            target: node2.fact(),
                        },
                    )?)
                    .await,
            )?;
            committed(
                writer
                    .submit(&request(
                        3,
                        AssignmentCommand::Assign {
                            scope: b,
                            target: node2.fact(),
                        },
                    )?)
                    .await,
            )?;
            assert_eq!(high_water(&pool).await?.0, "3");

            // Restore only A's current row to its revision-1 receipt. All three
            // receipts and the namespace high-water deliberately survive.
            let mut restore = pool.begin().await?;
            sqlx::query("ALTER TABLE game_runtime_scope_assignments DISABLE TRIGGER game_runtime_scope_assignment_guard")
                .execute(&mut *restore).await?;
            sqlx::query(
                "UPDATE game_runtime_scope_assignments a SET \
                 ownership_generation=r.ownership_generation, state=r.state, holder_node_id=r.holder_node_id, \
                 holder_registration_revision=r.holder_registration_revision, source_revision=r.source_revision, \
                 decision_identity=r.decision_identity, operation_key=r.operation_key, decided_at=r.decided_at \
                 FROM game_runtime_scope_assignment_receipts r \
                 WHERE a.scope_key=r.scope_key AND r.source_revision=1",
            ).execute(&mut *restore).await?;
            sqlx::query("ALTER TABLE game_runtime_scope_assignments ENABLE TRIGGER game_runtime_scope_assignment_guard")
                .execute(&mut *restore).await?;
            restore.commit().await?;

            assert!(root.read_runtime_scope_assignment(a).await.is_err());
            let mutation = request(
                4,
                AssignmentCommand::Replace {
                    scope: a,
                    predecessor: first.predecessor(),
                    target: node2.fact(),
                },
            )?;
            assert!(matches!(
                writer.submit(&mutation).await,
                Err(AssignmentError::Ambiguous)
            ));
            assert_eq!(high_water(&pool).await?.0, "3");

            let a_key: Vec<u8> = sqlx::query_scalar(
                "SELECT scope_key FROM game_runtime_scope_assignment_receipts WHERE source_revision=1",
            ).fetch_one(&pool).await?;
            let attested: bool = sqlx::query_scalar(
                "SELECT game_runtime_attest_readiness($1, encode($2, 'hex')::uuid, $3::numeric, $4)",
            )
            .bind(a_key)
            .bind(node1.fact().node_id().as_bytes().as_slice())
            .bind(node1.fact().registration_revision().to_string())
            .bind([1_u8; 32].as_slice())
            .fetch_one(&pool).await?;
            assert!(!attested);
            let fenced = current_runtime(&guards, a).await?;
            assert!(matches!(
                publish_ready(
                    &root,
                    &node1,
                    runtime_change(a, Some(&fenced), 1, true, now),
                    now
                )
                .await,
                Err(AssignmentError::NotCurrentHolder)
            ));
            // The replacement fence already moved A's guard to generation 2.
            assert_eq!(
                runtime_ready(&current_runtime(&guards, a).await?),
                Some((2, false))
            );

            Ok(())
        })
    })
}

#[test]
fn intermediate_receipt_hole_cannot_be_crossed() -> TestResult {
    run("receipt_hole", |database| {
        Box::pin(async move {
            let root = ready_root(&database.url).await?;
            let pool = database.pool().await?;
            let node1 = register(&root, 1, None).await?;
            let node2 = register(&root, 2, None).await?;
            let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "writer-a")
                .await
                .map_err(|e| format!("{e:?}"))?;
            let first = committed(
                writer
                    .submit(&request(
                        1,
                        AssignmentCommand::Assign {
                            scope: scope(1)?,
                            target: node1,
                        },
                    )?)
                    .await,
            )?;
            let second = committed(
                writer
                    .submit(&request(
                        2,
                        AssignmentCommand::Replace {
                            scope: scope(1)?,
                            predecessor: first.predecessor(),
                            target: node2,
                        },
                    )?)
                    .await,
            )?;
            committed(
                writer
                    .submit(&request(
                        3,
                        AssignmentCommand::Replace {
                            scope: scope(1)?,
                            predecessor: second.predecessor(),
                            target: node1,
                        },
                    )?)
                    .await,
            )?;
            committed(
                writer
                    .submit(&request(
                        4,
                        AssignmentCommand::Assign {
                            scope: scope(2)?,
                            target: node2,
                        },
                    )?)
                    .await,
            )?;

            // Revision 2 is neither the namespace maximum nor the latest receipt
            // for its scope, so only the explicit continuity proof catches it.
            sqlx::query("ALTER TABLE game_runtime_scope_assignment_receipts DISABLE TRIGGER game_runtime_scope_assignment_receipt_immutable")
                .execute(&pool).await?;
            sqlx::query(
                "DELETE FROM game_runtime_scope_assignment_receipts WHERE source_revision=2",
            )
            .execute(&pool)
            .await?;
            sqlx::query("ALTER TABLE game_runtime_scope_assignment_receipts ENABLE TRIGGER game_runtime_scope_assignment_receipt_immutable")
                .execute(&pool).await?;
            assert!(root.read_runtime_scope_assignment(scope(1)?).await.is_err());
            assert!(matches!(
                writer
                    .submit(&request(
                        5,
                        AssignmentCommand::Assign {
                            scope: scope(3)?,
                            target: node1,
                        }
                    )?)
                    .await,
                Err(AssignmentError::Ambiguous)
            ));
            assert_eq!(high_water(&pool).await?.0, "4");
            Ok(())
        })
    })
}

#[test]
fn duplicate_writer_handles_share_one_nasg_queue() -> TestResult {
    run("shared_queue", |database| {
        Box::pin(async move {
            let root = ready_root(&database.url).await?;
            let pool = database.pool().await?;
            let target = register(&root, 1, None).await?;
            let first_handle = RuntimeScopeAssignmentWriter::open(root.clone(), "writer-a")
                .await
                .map_err(|e| format!("{e:?}"))?;
            let second_handle = RuntimeScopeAssignmentWriter::open(root.clone(), "writer-a")
                .await
                .map_err(|e| format!("{e:?}"))?;
            let mut blocker = pool.begin().await?;
            sqlx::query("LOCK TABLE game_runtime_scope_assignment_writer IN EXCLUSIVE MODE")
                .execute(&mut *blocker)
                .await?;
            let inflight_handle = first_handle.clone();
            let inflight_request = request(
                1,
                AssignmentCommand::Assign {
                    scope: scope(1)?,
                    target,
                },
            )?;
            let inflight =
                tokio::spawn(async move { inflight_handle.submit(&inflight_request).await });
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            // Eight pending commands split across both handles fill the one queue.
            let mut pending = Vec::new();
            for index in 0..MAX_PENDING_COMMANDS {
                let handle = if index % 2 == 0 {
                    first_handle.clone()
                } else {
                    second_handle.clone()
                };
                let tag = u8::try_from(10 + index)?;
                let command = request(
                    tag,
                    AssignmentCommand::Assign {
                        scope: scope(tag)?,
                        target,
                    },
                )?;
                pending.push(tokio::spawn(async move { handle.submit(&command).await }));
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            let ninth = request(
                30,
                AssignmentCommand::Assign {
                    scope: scope(30)?,
                    target,
                },
            )?;
            assert!(matches!(
                second_handle.submit(&ninth).await,
                Err(AssignmentError::QueueFull)
            ));
            for waiting in pending {
                assert!(matches!(waiting.await?, Err(AssignmentError::QueueTimeout)));
            }
            assert!(matches!(inflight.await?, Err(AssignmentError::Ambiguous)));
            blocker.rollback().await?;
            // The ambiguity is visible through every handle of the registration.
            assert_eq!(second_handle.unreconciled(), Some(key(1)));
            ensure_ready(&root).await?;
            assert_eq!(
                second_handle
                    .reconcile(&request(
                        1,
                        AssignmentCommand::Assign {
                            scope: scope(1)?,
                            target
                        }
                    )?)
                    .await
                    .map_err(|e| format!("{e:?}"))?,
                ReconcileOutcome::Absent
            );
            assert_eq!(first_handle.unreconciled(), None);
            pool.close().await;
            Ok(())
        })
    })
}

#[test]
fn registration_state_rollback_cannot_restore_currentness() -> TestResult {
    run("registration_rollback", |database| {
        Box::pin(async move {
            let root = ready_root(&database.url).await?;
            let pool = database.pool().await?;
            let channel = scope(1)?;
            let a = register_proof(&root, 1, None).await?;
            // B supersedes A: registration revision 2, A's ending revision 3.
            let b = register_proof(&root, 2, Some(a.fact().node_id())).await?;
            // C is registered then revoked: registration 4, ending 5.
            let c = register_proof(&root, 3, None).await?;
            root.revoke_node_registration(c.fact())
                .await
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(high_water(&pool).await?.1, "5");
            let endings: Vec<(String, i16)> = sqlx::query_as(
                "SELECT ending_revision::text, state FROM game_node_registration_endings \
                 ORDER BY ending_revision",
            )
            .fetch_all(&pool)
            .await?;
            assert_eq!(endings, vec![("3".into(), 3), ("5".into(), 2)]);
            root.require_current_node_registration(&b)
                .await
                .map_err(|e| format!("{e:?}"))?;
            // Endings are immutable.
            expect_sql_state(
                sqlx::query("DELETE FROM game_node_registration_endings")
                    .execute(&pool)
                    .await,
                "23514",
            )
            .await?;

            // Partial restore rolls only A's mutable row back to CURRENT while
            // the high-water, B and A's immutable ending survive.
            let mut restore = pool.begin().await?;
            sqlx::query(
                "ALTER TABLE game_node_registrations DISABLE TRIGGER game_node_registration_guard",
            )
            .execute(&mut *restore)
            .await?;
            sqlx::query(
                "UPDATE game_node_registrations SET state = 1, ended_at = NULL, superseded_by = NULL \
                 WHERE registration_revision = 1",
            )
            .execute(&mut *restore)
            .await?;
            sqlx::query(
                "ALTER TABLE game_node_registrations ENABLE TRIGGER game_node_registration_guard",
            )
            .execute(&mut *restore)
            .await?;
            restore.commit().await?;
            // A's still-held secret no longer proves currentness, and the
            // contradictory history also fails every other currentness check.
            for proof in [&a, &b] {
                assert!(
                    root.require_current_node_registration(proof).await.is_err(),
                    "currentness accepted over contradictory registration history"
                );
            }
            let target_current: bool = sqlx::query_scalar(
                "SELECT game_node_lock_current_registration(encode($1, 'hex')::uuid, 1)",
            )
            .bind(a.fact().node_id().as_bytes().as_slice())
            .fetch_one(&pool)
            .await?;
            assert!(!target_current);
            let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "writer-a")
                .await
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(
                rejected(
                    writer
                        .submit(&request(
                            1,
                            AssignmentCommand::Assign {
                                scope: channel,
                                target: a.fact(),
                            },
                        )?)
                        .await
                )?,
                AssignmentRejection::TargetNotCurrent
            );
            // New registration also refuses to allocate over the contradiction.
            assert!(register_proof(&root, 4, None).await.is_err());
            assert_eq!(high_water(&pool).await?.1, "5");

            // A deeper restore that also drops the ending leaves a revision hole.
            let mut restore = pool.begin().await?;
            sqlx::query("ALTER TABLE game_node_registration_endings DISABLE TRIGGER game_node_registration_ending_guard")
                .execute(&mut *restore)
                .await?;
            sqlx::query("DELETE FROM game_node_registration_endings WHERE ending_revision = 3")
                .execute(&mut *restore)
                .await?;
            sqlx::query("ALTER TABLE game_node_registration_endings ENABLE TRIGGER game_node_registration_ending_guard")
                .execute(&mut *restore)
                .await?;
            restore.commit().await?;
            assert!(root.require_current_node_registration(&a).await.is_err());
            assert!(root.require_current_node_registration(&b).await.is_err());
            pool.close().await;
            Ok(())
        })
    })
}

#[test]
fn restored_guard_behind_latest_fence_and_stale_row_fail_closed() -> TestResult {
    run("guard_fence_restore", |database| {
        Box::pin(async move {
            let root = ready_root(&database.url).await?;
            let pool = database.pool().await?;
            let guards = AdmissionGuardStore::from_root(root.clone());
            let channel = scope(1)?;
            let a = register(&root, 1, None).await?;
            let b = register(&root, 2, None).await?;
            let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "writer-a")
                .await
                .map_err(|e| format!("{e:?}"))?;
            let now = db_now(&pool).await?;
            publish_runtime(&guards, runtime_change(channel, None, 1, true, now), now).await?;
            let first = committed(
                writer
                    .submit(&request(
                        1,
                        AssignmentCommand::Assign {
                            scope: channel,
                            target: a,
                        },
                    )?)
                    .await,
            )?;
            assert_eq!(first.fenced_publication_revision, Some(2));
            // Snapshot the guard as fenced by the first decision.
            sqlx::query("CREATE TABLE snap_runtime_guard AS SELECT * FROM game_durability_admission_runtime_guards")
                .execute(&pool)
                .await?;
            let second = committed(
                writer
                    .submit(&request(
                        2,
                        AssignmentCommand::Replace {
                            scope: channel,
                            predecessor: first.predecessor(),
                            target: b,
                        },
                    )?)
                    .await,
            )?;
            assert_eq!(second.fenced_publication_revision, Some(3));

            // Partial restore: the Runtime guard and its history roll back to
            // the first fence while the assignment row, receipts and
            // high-water retain the replacement decision.
            let mut restore = pool.begin().await?;
            for statement in [
                "ALTER TABLE game_durability_admission_runtime_guards DISABLE TRIGGER USER",
                "ALTER TABLE game_durability_admission_guard_history DISABLE TRIGGER USER",
                "DELETE FROM game_durability_admission_runtime_guards",
                "INSERT INTO game_durability_admission_runtime_guards SELECT * FROM snap_runtime_guard",
                "DELETE FROM game_durability_admission_guard_history WHERE publication_revision = 3",
                "ALTER TABLE game_durability_admission_runtime_guards ENABLE TRIGGER USER",
                "ALTER TABLE game_durability_admission_guard_history ENABLE TRIGGER USER",
            ] {
                sqlx::query(statement).execute(&mut *restore).await?;
            }
            restore.commit().await?;
            assert!(root.read_runtime_scope_predecessor(channel).await.is_err());
            // A predecessor built on the rolled-back guard cannot cross the
            // missing fence and reuse its publication position.
            let stale = AssignmentPredecessor {
                runtime_guard_publication_revision: Some(2),
                ..second.predecessor()
            };
            let revoke = request(
                3,
                AssignmentCommand::Revoke {
                    scope: channel,
                    predecessor: stale,
                },
            )?;
            let outcome = writer.submit(&revoke).await;
            assert!(
                !matches!(outcome, Ok(AssignmentOutcome::Committed(_))),
                "revoke committed across a missing guard fence: {outcome:?}"
            );
            ensure_ready(&root).await?;
            let _ = writer.reconcile(&revoke).await;
            assert_eq!(high_water(&pool).await?.0, "2");

            // Also roll the assignment row back to the first decision: guard and
            // row are internally consistent again, but retained receipts prove
            // a newer decision, so no standalone publication may advance it.
            let mut restore = pool.begin().await?;
            sqlx::query("ALTER TABLE game_runtime_scope_assignments DISABLE TRIGGER game_runtime_scope_assignment_guard")
                .execute(&mut *restore)
                .await?;
            sqlx::query(
                "UPDATE game_runtime_scope_assignments a SET \
                 ownership_generation=r.ownership_generation, state=r.state, holder_node_id=r.holder_node_id, \
                 holder_registration_revision=r.holder_registration_revision, source_revision=r.source_revision, \
                 decision_identity=r.decision_identity, operation_key=r.operation_key, decided_at=r.decided_at \
                 FROM game_runtime_scope_assignment_receipts r \
                 WHERE a.scope_key=r.scope_key AND r.source_revision=1",
            )
            .execute(&mut *restore)
            .await?;
            sqlx::query("ALTER TABLE game_runtime_scope_assignments ENABLE TRIGGER game_runtime_scope_assignment_guard")
                .execute(&mut *restore)
                .await?;
            restore.commit().await?;
            let restored = current_runtime(&guards, channel).await?;
            assert_eq!(runtime_ready(&restored), Some((1, false)));
            let now = db_now(&pool).await?;
            assert!(matches!(
                publish_runtime(
                    &guards,
                    runtime_change(channel, Some(&restored), 1, false, now),
                    now
                )
                .await,
                Err(DurabilityError::Database(_))
            ));
            pool.close().await;
            Ok(())
        })
    })
}

#[test]
fn dropped_or_substituted_fence_history_fails_closed() -> TestResult {
    run("fence_history_restore", |database| {
        Box::pin(async move {
            let root = ready_root(&database.url).await?;
            let pool = database.pool().await?;
            let guards = AdmissionGuardStore::from_root(root.clone());
            let channel = scope(1)?;
            let a = register(&root, 1, None).await?;
            let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "writer-a")
                .await
                .map_err(|e| format!("{e:?}"))?;
            let now = db_now(&pool).await?;
            publish_runtime(&guards, runtime_change(channel, None, 1, true, now), now).await?;
            let first = committed(
                writer
                    .submit(&request(
                        1,
                        AssignmentCommand::Assign {
                            scope: channel,
                            target: a,
                        },
                    )?)
                    .await,
            )?;
            assert_eq!(first.fenced_publication_revision, Some(2));
            // A later standalone non-ready publication moves the guard beyond
            // the fence; the retained fence revision still proves the decision.
            let fenced = current_runtime(&guards, channel).await?;
            let now = db_now(&pool).await?;
            publish_runtime(
                &guards,
                runtime_change(channel, Some(&fenced), 1, false, now),
                now,
            )
            .await?;
            root.read_runtime_scope_predecessor(channel)
                .await
                .map_err(|e| format!("{e:?}"))?;

            // Substituted provenance at the fence revision fails closed.
            let mut restore = pool.begin().await?;
            for statement in [
                "ALTER TABLE game_durability_admission_guard_history DISABLE TRIGGER USER",
                "CREATE TABLE snap_fence AS SELECT * FROM game_durability_admission_guard_history WHERE publication_revision = 2",
                "UPDATE game_durability_admission_guard_history SET decision_identity = 'substituted-decision' WHERE publication_revision = 2",
                "ALTER TABLE game_durability_admission_guard_history ENABLE TRIGGER USER",
            ] {
                sqlx::query(statement).execute(&mut *restore).await?;
            }
            restore.commit().await?;
            assert!(root.read_runtime_scope_predecessor(channel).await.is_err());

            // A dropped fence revision behind a consistent newer guard fails closed.
            let mut restore = pool.begin().await?;
            for statement in [
                "ALTER TABLE game_durability_admission_guard_history DISABLE TRIGGER USER",
                "DELETE FROM game_durability_admission_guard_history WHERE publication_revision = 2",
                "ALTER TABLE game_durability_admission_guard_history ENABLE TRIGGER USER",
            ] {
                sqlx::query(statement).execute(&mut *restore).await?;
            }
            restore.commit().await?;
            assert!(root.read_runtime_scope_predecessor(channel).await.is_err());
            let revoke = request(
                2,
                AssignmentCommand::Revoke {
                    scope: channel,
                    predecessor: AssignmentPredecessor {
                        runtime_guard_publication_revision: Some(3),
                        ..first.predecessor()
                    },
                },
            )?;
            let outcome = writer.submit(&revoke).await;
            assert!(
                !matches!(outcome, Ok(AssignmentOutcome::Committed(_))),
                "revoke committed across a missing fence revision: {outcome:?}"
            );
            ensure_ready(&root).await?;
            let _ = writer.reconcile(&revoke).await;
            assert_eq!(high_water(&pool).await?.0, "1");

            // Restoring the exact fence row restores the proof.
            let mut restore = pool.begin().await?;
            for statement in [
                "ALTER TABLE game_durability_admission_guard_history DISABLE TRIGGER USER",
                "INSERT INTO game_durability_admission_guard_history SELECT * FROM snap_fence",
                "ALTER TABLE game_durability_admission_guard_history ENABLE TRIGGER USER",
            ] {
                sqlx::query(statement).execute(&mut *restore).await?;
            }
            restore.commit().await?;
            root.read_runtime_scope_predecessor(channel)
                .await
                .map_err(|e| format!("{e:?}"))?;

            // The exact fence payload moved under another guard key, or with a
            // disagreeing SQL mirror, is not this guard's fence.
            for tamper in [
                "UPDATE game_durability_admission_guard_history SET guard_key = guard_key || '\\x00'::bytea \
                 WHERE change_json IN (SELECT change_json FROM snap_fence)",
                "UPDATE game_durability_admission_guard_history SET source_revision = source_revision + 100 \
                 WHERE change_json IN (SELECT change_json FROM snap_fence)",
            ] {
                let mut restore = pool.begin().await?;
                for statement in [
                    "ALTER TABLE game_durability_admission_guard_history DISABLE TRIGGER USER",
                    tamper,
                    "ALTER TABLE game_durability_admission_guard_history ENABLE TRIGGER USER",
                ] {
                    sqlx::query(statement).execute(&mut *restore).await?;
                }
                restore.commit().await?;
                assert!(
                    root.read_runtime_scope_predecessor(channel).await.is_err(),
                    "accepted tampered fence row: {tamper}"
                );
                let mut restore = pool.begin().await?;
                for statement in [
                    "ALTER TABLE game_durability_admission_guard_history DISABLE TRIGGER USER",
                    "DELETE FROM game_durability_admission_guard_history WHERE change_json IN (SELECT change_json FROM snap_fence)",
                    "INSERT INTO game_durability_admission_guard_history SELECT * FROM snap_fence",
                    "ALTER TABLE game_durability_admission_guard_history ENABLE TRIGGER USER",
                ] {
                    sqlx::query(statement).execute(&mut *restore).await?;
                }
                restore.commit().await?;
                root.read_runtime_scope_predecessor(channel)
                    .await
                    .map_err(|e| format!("{e:?}"))?;
            }
            pool.close().await;
            Ok(())
        })
    })
}

#[test]
fn restored_occupied_slot_reconciles_to_its_committed_receipt() -> TestResult {
    run("slot_restore_reconcile", |database| {
        Box::pin(async move {
            let root = ready_root(&database.url).await?;
            let pool = database.pool().await?;
            let a = register(&root, 1, None).await?;
            let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "writer-a")
                .await
                .map_err(|e| format!("{e:?}"))?;
            let assign = request(
                1,
                AssignmentCommand::Assign {
                    scope: scope(1)?,
                    target: a,
                },
            )?;
            let receipt = committed(writer.submit(&assign).await)?;
            // Partial restore: the slot returns to its pre-commit occupied
            // checkpoint while the receipt and assignment survive.
            sqlx::query(
                "UPDATE game_runtime_scope_assignment_slots SET operation_key = $1, command = $2, checkpointed_at = 0 \
                 WHERE writer_registration = 'writer-a'",
            )
            .bind(key(1).as_bytes().as_slice())
            .bind(assign.encode().map_err(|e| format!("{e:?}"))?)
            .execute(&pool)
            .await?;
            assert_eq!(
                writer
                    .reconcile(&assign)
                    .await
                    .map_err(|e| format!("{e:?}"))?,
                ReconcileOutcome::Committed(receipt.clone())
            );
            // A restored slot whose command differs from the retained receipt
            // for the same operation key: a caller asking about another command
            // cannot clear that custody; the slot's own exact command reconciles
            // deterministically to a conflict, again after custody is cleared.
            let foreign = request(
                1,
                AssignmentCommand::Assign {
                    scope: scope(2)?,
                    target: a,
                },
            )?;
            sqlx::query(
                "UPDATE game_runtime_scope_assignment_slots SET operation_key = $1, command = $2, checkpointed_at = 0 \
                 WHERE writer_registration = 'writer-a'",
            )
            .bind(key(1).as_bytes().as_slice())
            .bind(foreign.encode().map_err(|e| format!("{e:?}"))?)
            .execute(&pool)
            .await?;
            assert!(matches!(
                writer.reconcile(&assign).await,
                Err(AssignmentError::ReconcileRequired)
            ));
            let retained: Option<Vec<u8>> = sqlx::query_scalar(
                "SELECT operation_key FROM game_runtime_scope_assignment_slots WHERE writer_registration = 'writer-a'",
            )
            .fetch_one(&pool)
            .await?;
            assert_eq!(retained.as_deref(), Some(key(1).as_bytes().as_slice()));
            for _ in 0..2 {
                assert_eq!(
                    writer
                        .reconcile(&foreign)
                        .await
                        .map_err(|e| format!("{e:?}"))?,
                    ReconcileOutcome::Conflict
                );
            }
            // A changed-command replay is a conflict whose lost response still
            // reconciles to that conflict; the original command stays committed.
            assert_eq!(
                writer
                    .submit(&foreign)
                    .await
                    .map_err(|e| format!("{e:?}"))?,
                AssignmentOutcome::Rejected(AssignmentRejection::OperationConflict)
            );
            assert_eq!(
                writer
                    .reconcile(&foreign)
                    .await
                    .map_err(|e| format!("{e:?}"))?,
                ReconcileOutcome::Conflict
            );
            assert_eq!(
                writer
                    .reconcile(&assign)
                    .await
                    .map_err(|e| format!("{e:?}"))?,
                ReconcileOutcome::Committed(receipt.clone())
            );
            // A partial restore that drops the committed receipt while the
            // high-water and assignment survive cannot be classified as a
            // non-commit: reconciliation fails closed and keeps custody.
            sqlx::query(
                "UPDATE game_runtime_scope_assignment_slots SET operation_key = $1, command = $2, checkpointed_at = 0 \
                 WHERE writer_registration = 'writer-a'",
            )
            .bind(key(1).as_bytes().as_slice())
            .bind(assign.encode().map_err(|e| format!("{e:?}"))?)
            .execute(&pool)
            .await?;
            let mut restore = pool.begin().await?;
            for statement in [
                "CREATE TABLE snap_receipt AS SELECT * FROM game_runtime_scope_assignment_receipts",
                "SET LOCAL session_replication_role = replica",
                "DELETE FROM game_runtime_scope_assignment_receipts",
            ] {
                sqlx::query(statement).execute(&mut *restore).await?;
            }
            restore.commit().await?;
            assert!(!matches!(
                writer.reconcile(&assign).await,
                Ok(ReconcileOutcome::Absent)
            ));
            ensure_ready(&root).await?;
            let retained: Option<Vec<u8>> = sqlx::query_scalar(
                "SELECT operation_key FROM game_runtime_scope_assignment_slots WHERE writer_registration = 'writer-a'",
            )
            .fetch_one(&pool)
            .await?;
            assert_eq!(retained.as_deref(), Some(key(1).as_bytes().as_slice()));
            let mut restore = pool.begin().await?;
            for statement in [
                "SET LOCAL session_replication_role = replica",
                "INSERT INTO game_runtime_scope_assignment_receipts SELECT * FROM snap_receipt",
            ] {
                sqlx::query(statement).execute(&mut *restore).await?;
            }
            restore.commit().await?;
            assert_eq!(
                writer
                    .reconcile(&assign)
                    .await
                    .map_err(|e| format!("{e:?}"))?,
                ReconcileOutcome::Committed(receipt.clone())
            );
            // One assignment-writer registration per process root, for the
            // root's lifetime: dropping every handle does not release it.
            assert!(matches!(
                RuntimeScopeAssignmentWriter::open(root.clone(), "writer-b").await,
                Err(AssignmentError::Unsupported)
            ));
            drop(writer);
            assert!(matches!(
                RuntimeScopeAssignmentWriter::open(root.clone(), "writer-b").await,
                Err(AssignmentError::Unsupported)
            ));
            RuntimeScopeAssignmentWriter::open(root.clone(), "writer-a")
                .await
                .map_err(|e| format!("{e:?}"))?;
            pool.close().await;
            Ok(())
        })
    })
}
