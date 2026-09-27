// Shared R7 P03 cases. Both wrappers provide the same path-loaded crate root.

use crate::character_recovery_fence::CharacterRecoveryStore;
use crate::domain::progression::{
    FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
};
use crate::domain::{CharacterId, CharacterRevision};
use crate::durability::DurabilityRoot;
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, ExperienceRewardOccurrence,
};
use crate::durability::runtime_scope_assignment::{
    BootstrapSecret, LaunchBinding, NodeIncarnationProof,
};
use crate::foundation::{ConnectionGeneration, RuntimeScopeRefV1, ScopeOwnershipGeneration};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};
use sqlx::{Connection, Executor, Row};
use std::future::Future;
use std::task::Poll;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

async fn join_two<A, B>(first: A, second: B) -> (A::Output, B::Output)
where
    A: Future,
    B: Future,
{
    let mut first = std::pin::pin!(first);
    let mut second = std::pin::pin!(second);
    let mut first_output = None;
    let mut second_output = None;
    std::future::poll_fn(move |context| {
        if first_output.is_none()
            && let Poll::Ready(output) = first.as_mut().poll(context)
        {
            first_output = Some(output);
        }
        if second_output.is_none()
            && let Poll::Ready(output) = second.as_mut().poll(context)
        {
            second_output = Some(output);
        }
        match (first_output.take(), second_output.take()) {
            (Some(first), Some(second)) => Poll::Ready((first, second)),
            (first, second) => {
                first_output = first;
                second_output = second;
                Poll::Pending
            }
        }
    })
    .await
}

fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

struct Database {
    admin_url: String,
    name: String,
    url: String,
}

impl Database {
    async fn create(admin_url: String, name: &str) -> TestResult<Self> {
        if !admin_url.starts_with("postgresql://oteryn_test_admin:")
            || !admin_url.ends_with("@127.0.0.1:5432/postgres")
        {
            return Err("unsafe PostgreSQL test admin URL".into());
        }
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let name = format!("cp_{name}_{suffix}");
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
        assert_eq!(version, "170006", "canonical target is PostgreSQL 17.6");
        sqlx::migrate!("./migrations").run(&mut connection).await?;
        connection.close().await?;
        Ok(Self {
            admin_url,
            name,
            url,
        })
    }

    async fn cleanup(self) -> TestResult {
        let mut admin = sqlx::PgConnection::connect(&self.admin_url).await?;
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "DROP DATABASE {} WITH (FORCE)",
                self.name
            ))))
            .await?;
        admin.close().await?;
        Ok(())
    }
}

fn fence_parent(tag: &str) -> TestResult<std::path::PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let parent = std::env::temp_dir().join(format!(
        "oteryn-character-progression-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&parent);
    std::fs::create_dir_all(&parent)?;
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700))?;
    Ok(parent)
}

async fn register(root: &DurabilityRoot, tag: u8) -> TestResult<NodeIncarnationProof> {
    let secret = BootstrapSecret::from_bytes([tag; 32]);
    let launch = LaunchBinding::new(&format!("progression-launch-{tag}"))
        .map_err(|error| format!("{error:?}"))?;
    let node = crate::foundation::NodeId::decode(&id(tag)).map_err(|error| format!("{error:?}"))?;
    root.issue_node_bootstrap_authorization(&secret, &launch, None)
        .await
        .map_err(|error| format!("{error:?}"))?;
    root.register_node_incarnation(&secret, &launch, node)
        .await
        .map_err(|error| format!("{error:?}").into())
}

struct Harness {
    database: Database,
    root: DurabilityRoot,
    pool: sqlx::PgPool,
    recovery: CharacterRecoveryStore,
    retained: std::path::PathBuf,
    node: NodeIncarnationProof,
}

impl Harness {
    async fn create(admin: String, tag: &str, initialized: bool) -> TestResult<Self> {
        let database = Database::create(admin, tag).await?;
        let root = DurabilityRoot::connect_test_runtime(&database.url)?;
        assert!(root.maintain_ready_once().await?);
        let pool = sqlx::PgPool::connect(&database.url).await?;
        let retained = fence_parent(tag)?;
        let recovery = CharacterRecoveryStore::open(&retained, "character-primary", "game-ops")
            .map_err(|error| format!("{error:?}"))?;
        {
            let fresh = recovery
                .authorize_fresh_store(id(10), 100)
                .map_err(|error| format!("{error:?}"))?;
            root.admit_fresh_character_recovery(&fresh)
                .await
                .map_err(|error| format!("{error:?}"))?;
        }
        let node = register(&root, 1).await?;
        seed_character(&pool, &node, initialized).await?;
        Ok(Self {
            database,
            root,
            pool,
            recovery,
            retained,
            node,
        })
    }

    async fn cleanup(self) -> TestResult {
        self.pool.close().await;
        self.database.cleanup().await?;
        std::fs::remove_dir_all(self.retained)?;
        Ok(())
    }
}

fn bootstrap_binding() -> Vec<u8> {
    let mut binding = vec![1];
    binding.extend_from_slice(&id(31));
    binding.extend_from_slice(&1_i64.to_be_bytes());
    binding.extend_from_slice(&id(30));
    binding.extend_from_slice(&id(40));
    binding.extend_from_slice(&id(42));
    binding.extend_from_slice(&1_i64.to_be_bytes());
    binding.extend_from_slice(&120_i64.to_be_bytes());
    for value in ["profile-1", "ruleset-1", "content-1", "starter-1"] {
        binding.extend_from_slice(&u16::try_from(value.len()).expect("length").to_be_bytes());
        binding.extend_from_slice(value.as_bytes());
    }
    binding
}

fn channel_scope_key() -> [u8; 33] {
    let mut key = [0_u8; 33];
    key[0] = 1;
    key[1..17].copy_from_slice(&id(42));
    key[17..].copy_from_slice(&id(43));
    key
}

async fn seed_character(
    pool: &sqlx::PgPool,
    node: &NodeIncarnationProof,
    initialized: bool,
) -> TestResult {
    sqlx::query(
        "INSERT INTO game_character_interpretations VALUES \
         (1,'profile-1','ruleset-1','content-1','starter-1',1)",
    )
    .execute(pool)
    .await?;
    sqlx::query("INSERT INTO game_character_account_guards VALUES (encode($1,'hex')::uuid)")
        .bind(id(40).as_slice())
        .execute(pool)
        .await?;
    sqlx::query(
        "INSERT INTO game_character_roots VALUES \
         (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
          1,1,'profile-1','ruleset-1','content-1','starter-1')",
    )
    .bind(id(41).as_slice())
    .bind(id(40).as_slice())
    .bind(id(42).as_slice())
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO game_character_operation_receipts(\
           operation_id,command_binding,account_id,character_id,world_id,character_revision,\
           event_id,occurred_at,server_build_id,transaction_id,issuer_decision_id,\
           intent_source_revision,issued_at_source,expires_at_source) \
         VALUES (encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,\
           encode($5,'hex')::uuid,1,encode($6,'hex')::uuid,1,'test/1',\
           encode($7,'hex')::uuid,encode($8,'hex')::uuid,1,1,120)",
    )
    .bind(id(30).as_slice())
    .bind(bootstrap_binding())
    .bind(id(40).as_slice())
    .bind(id(41).as_slice())
    .bind(id(42).as_slice())
    .bind(id(32).as_slice())
    .bind(id(33).as_slice())
    .bind(id(31).as_slice())
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO game_character_bootstrap_intent_floors VALUES \
         (1,1,encode($1,'hex')::uuid,$2)",
    )
    .bind(id(31).as_slice())
    .bind(bootstrap_binding())
    .execute(pool)
    .await?;
    if initialized {
        sqlx::query(
            "INSERT INTO game_character_progression_state VALUES \
             (encode($1,'hex')::uuid,1,50,1000,'profile-1','ruleset-1','content-1',\
              'simulation-1','evidence-1','declaration-1','policy-1','reward-1')",
        )
        .bind(id(41).as_slice())
        .execute(pool)
        .await?;
    }

    sqlx::query(
        "INSERT INTO game_durability_reconnect_sessions(\
           game_session_id,account_id,character_id,world_id,runtime_scope_kind,\
           runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,\
           original_grace_deadline,predecessor_generation,character_lease_generation,\
           scope_ownership_generation,current_generation,current_transport_ref,session_state) \
         VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
           encode($4,'hex')::uuid,1,encode($4,'hex')::uuid,encode($5,'hex')::uuid,\
           1,999999,1,1,1,1,$6,1)",
    )
    .bind(id(50).as_slice())
    .bind(id(40).as_slice())
    .bind(id(41).as_slice())
    .bind(id(42).as_slice())
    .bind(id(43).as_slice())
    .bind([9_u8; 16].as_slice())
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO game_durability_admission_account_guards VALUES \
         (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
          1,'test',1,'account-current',1,0,'{}')",
    )
    .bind(id(40).as_slice())
    .bind(id(41).as_slice())
    .bind(id(50).as_slice())
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO game_durability_admission_character_guards VALUES \
         (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
          true,1,encode($4,'hex')::uuid,1,'test',1,'character-current',1,0,'{}')",
    )
    .bind(id(41).as_slice())
    .bind(id(40).as_slice())
    .bind(id(42).as_slice())
    .bind(id(50).as_slice())
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO game_durability_admission_runtime_guards VALUES \
         ($1,1,true,1,'test',1,'runtime-current',1,0,'{}')",
    )
    .bind(channel_scope_key().as_slice())
    .execute(pool)
    .await?;

    let fact = node.fact();
    sqlx::query(
        "INSERT INTO game_runtime_scope_assignments(\
           scope_key,world_id,channel_id,ownership_generation,state,holder_node_id,\
           holder_registration_revision,source_revision,decision_identity,operation_key,decided_at) \
         VALUES ($1,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,1,\
           encode($4,'hex')::uuid,$5::text::numeric(20,0),1,'progression-fixture',$6,1)",
    )
    .bind(channel_scope_key().as_slice())
    .bind(id(42).as_slice())
    .bind(id(43).as_slice())
    .bind(fact.node_id().as_bytes().as_slice())
    .bind(fact.registration_revision().to_string())
    .bind([7_u8; 32].as_slice())
    .execute(pool)
    .await?;
    Ok(())
}

fn context() -> ProgressionRevisionContext<String> {
    ProgressionRevisionContext {
        profile: "profile-1".into(),
        ruleset: "ruleset-1".into(),
        content: "content-1".into(),
        simulation: "simulation-1".into(),
        evidence: "evidence-1".into(),
        declaration: "declaration-1".into(),
    }
}

fn request(tag: u8, amount: i64) -> TestResult<ExperienceAwardRequest<2>> {
    let context = context();
    Ok(ExperienceAwardRequest {
        occurrence: ExperienceRewardOccurrence::from_bytes(id(tag))
            .map_err(|error| format!("{error:?}"))?,
        amount: ExactI64::new(amount),
        context: context.clone(),
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
        policy: FiniteProgressionPolicy {
            context,
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
            death_policy_revision: "death-1".into(),
            declared_difference_revision: "declaration-1".into(),
            thresholds: [
                LevelThreshold {
                    level: 50,
                    minimum_experience: ExactI64::new(1000),
                },
                LevelThreshold {
                    level: 51,
                    minimum_experience: ExactI64::new(1100),
                },
            ],
            terminal_exclusive_experience: ExactI64::new(1200),
            death_loss_numerator: 1,
            death_loss_denominator: 10,
            death_loss_rounding: RoundingMode::Floor,
        },
    })
}

fn fence(revision: u64) -> TestResult<CurrentCharacterGameplayFence> {
    Ok(CurrentCharacterGameplayFence {
        character_id: CharacterId::from_bytes(id(41)).map_err(|error| format!("{error:?}"))?,
        game_session_id: crate::foundation::GameSessionId::decode(&id(50))
            .map_err(|error| format!("{error:?}"))?,
        connection_generation: ConnectionGeneration::new(1)
            .map_err(|error| format!("{error:?}"))?,
        character_lease_generation: 1,
        runtime_scope: RuntimeScopeRefV1::channel(
            crate::foundation::WorldId::decode(&id(42)).map_err(|error| format!("{error:?}"))?,
            crate::foundation::ChannelId::decode(&id(43)).map_err(|error| format!("{error:?}"))?,
        ),
        scope_ownership_generation: ScopeOwnershipGeneration::new(1)
            .map_err(|error| format!("{error:?}"))?,
        expected_character_revision: CharacterRevision::new(revision)
            .map_err(|error| format!("{error:?}"))?,
    })
}

fn configured_admin() -> Option<String> {
    match std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") {
        Ok(value) => Some(value),
        Err(_) => {
            eprintln!(
                "PRE-ROUTING / NONCANONICAL: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured"
            );
            None
        }
    }
}

#[test]
fn exact_replay_reconcile_and_restart_readback_are_durable() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let harness = Harness::create(admin, "replay", true).await?;
            let seal = harness
                .recovery
                .seal_current()
                .map_err(|error| format!("{error:?}"))?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(|error| format!("{error:?}"))?;
            let award = request(60, 5)?;
            let committed = harness
                .root
                .commit_character_experience(&authority, &harness.node, fence(1)?, award.clone())
                .await
                .map_err(|error| format!("{error:?}"))?;
            let ExperienceCommitOutcome::Committed(committed) = committed else {
                return Err("first occurrence was not committed".into());
            };
            assert_eq!(committed.experience_before.get(), 1000);
            assert_eq!(committed.experience_after.get(), 1005);
            assert_eq!(committed.committed_character_revision.get(), 2);

            sqlx::query(
                "UPDATE game_durability_reconnect_sessions SET session_state = 3 \
                 WHERE game_session_id = encode($1,'hex')::uuid",
            )
            .bind(id(50).as_slice())
            .execute(&harness.pool)
            .await?;
            assert!(matches!(
                harness
                    .root
                    .commit_character_experience(
                        &authority,
                        &harness.node,
                        fence(2)?,
                        request(61, 5)?,
                    )
                    .await,
                Err(CharacterProgressionError::AuthorityRejected)
            ));
            let replay = harness
                .root
                .commit_character_experience(&authority, &harness.node, fence(1)?, award.clone())
                .await
                .map_err(|error| format!("{error:?}"))?;
            assert!(matches!(
                replay,
                ExperienceCommitOutcome::AlreadyCommitted(_)
            ));
            assert!(matches!(
                harness
                    .root
                    .commit_character_experience(
                        &authority,
                        &harness.node,
                        fence(1)?,
                        request(60, 6)?,
                    )
                    .await,
                Err(CharacterProgressionError::ConflictingOccurrence)
            ));
            let reconciled = harness
                .root
                .reconcile_character_experience(&authority, award.occurrence)
                .await
                .map_err(|error| format!("{error:?}"))?
                .ok_or("missing reconciliation receipt")?;
            assert_eq!(reconciled, committed);

            let restarted = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
            assert!(restarted.maintain_ready_once().await?);
            let restart_seal = harness
                .recovery
                .seal_current()
                .map_err(|error| format!("{error:?}"))?;
            let restart_authority = restarted
                .open_character_authority(&restart_seal)
                .await
                .map_err(|error| format!("restart integrity: {error:?}"))?;
            let state = restarted
                .read_character_progression(
                    &restart_authority,
                    CharacterId::from_bytes(id(41)).map_err(|error| format!("{error:?}"))?,
                )
                .await
                .map_err(|error| format!("{error:?}"))?
                .ok_or("missing progression state")?;
            assert_eq!(state.character_revision.get(), 2);
            assert_eq!(state.total_experience.get(), 1005);
            drop(restart_authority);
            drop(restart_seal);
            drop(restarted);
            drop(authority);
            drop(seal);
            harness.cleanup().await
        })
}

#[test]
fn stale_fences_context_and_missing_state_fail_closed() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let harness = Harness::create(admin.clone(), "stale", true).await?;
            let seal = harness
                .recovery
                .seal_current()
                .map_err(|error| format!("{error:?}"))?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(|error| format!("{error:?}"))?;

            assert!(matches!(
                harness
                    .root
                    .commit_character_experience(
                        &authority,
                        &harness.node,
                        fence(2)?,
                        request(61, 5)?,
                    )
                    .await,
                Err(CharacterProgressionError::CharacterRevisionMismatch)
            ));
            let mut stale_connection = fence(1)?;
            stale_connection.connection_generation =
                ConnectionGeneration::new(2).map_err(|error| format!("{error:?}"))?;
            assert!(matches!(
                harness
                    .root
                    .commit_character_experience(
                        &authority,
                        &harness.node,
                        stale_connection,
                        request(62, 5)?,
                    )
                    .await,
                Err(CharacterProgressionError::AuthorityRejected)
            ));
            let mut stale_lease = fence(1)?;
            stale_lease.character_lease_generation = 2;
            assert!(matches!(
                harness
                    .root
                    .commit_character_experience(
                        &authority,
                        &harness.node,
                        stale_lease,
                        request(63, 5)?,
                    )
                    .await,
                Err(CharacterProgressionError::AuthorityRejected)
            ));
            let mut stale_scope = fence(1)?;
            stale_scope.scope_ownership_generation =
                ScopeOwnershipGeneration::new(2).map_err(|error| format!("{error:?}"))?;
            assert!(matches!(
                harness
                    .root
                    .commit_character_experience(
                        &authority,
                        &harness.node,
                        stale_scope,
                        request(64, 5)?,
                    )
                    .await,
                Err(CharacterProgressionError::AuthorityRejected)
            ));
            let mut stale_context = request(65, 5)?;
            stale_context.context.content = "content-2".into();
            stale_context.policy.context.content = "content-2".into();
            assert!(matches!(
                harness
                    .root
                    .commit_character_experience(
                        &authority,
                        &harness.node,
                        fence(1)?,
                        stale_context,
                    )
                    .await,
                Err(CharacterProgressionError::ProgressionContextMismatch)
            ));
            drop(authority);
            drop(seal);
            harness.cleanup().await?;

            let missing = Harness::create(admin, "missing", false).await?;
            let missing_seal = missing
                .recovery
                .seal_current()
                .map_err(|error| format!("{error:?}"))?;
            let missing_authority = missing
                .root
                .open_character_authority(&missing_seal)
                .await
                .map_err(|error| format!("{error:?}"))?;
            assert!(matches!(
                missing
                    .root
                    .commit_character_experience(
                        &missing_authority,
                        &missing.node,
                        fence(1)?,
                        request(66, 5)?,
                    )
                    .await,
                Err(CharacterProgressionError::MissingProgressionState)
            ));
            drop(missing_authority);
            drop(missing_seal);
            missing.cleanup().await
        })
}

#[test]
fn rollback_concurrency_and_ended_node_preserve_single_revision() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let rollback = Harness::create(admin.clone(), "rollback", true).await?;
            let seal = rollback
                .recovery
                .seal_current()
                .map_err(|error| format!("{error:?}"))?;
            let authority = rollback
                .root
                .open_character_authority(&seal)
                .await
                .map_err(|error| format!("{error:?}"))?;
            sqlx::raw_sql(
                "CREATE FUNCTION test_reject_xp_receipt() RETURNS trigger LANGUAGE plpgsql AS $$ \
                 BEGIN RAISE EXCEPTION 'injected receipt failure'; END; $$; \
                 CREATE TRIGGER test_reject_xp_receipt BEFORE INSERT ON game_character_xp_receipts \
                 FOR EACH ROW EXECUTE FUNCTION test_reject_xp_receipt();",
            )
            .execute(&rollback.pool)
            .await?;
            assert!(rollback
                .root
                .commit_character_experience(
                    &authority,
                    &rollback.node,
                    fence(1)?,
                    request(67, 5)?,
                )
                .await
                .is_err());
            sqlx::raw_sql(
                "DROP TRIGGER test_reject_xp_receipt ON game_character_xp_receipts; \
                 DROP FUNCTION test_reject_xp_receipt();",
            )
            .execute(&rollback.pool)
            .await?;
            let row = sqlx::query(
                "SELECT r.character_revision::text, s.character_revision::text AS state_revision, \
                        s.total_experience, count(x.reward_occurrence_id) AS receipts \
                   FROM game_character_roots r JOIN game_character_progression_state s USING(character_id) \
                   LEFT JOIN game_character_xp_receipts x USING(character_id) \
                  GROUP BY r.character_revision,s.character_revision,s.total_experience",
            )
            .fetch_one(&rollback.pool)
            .await?;
            assert_eq!(row.try_get::<String, _>("character_revision")?, "1");
            assert_eq!(row.try_get::<String, _>("state_revision")?, "1");
            assert_eq!(row.try_get::<i64, _>("total_experience")?, 1000);
            assert_eq!(row.try_get::<i64, _>("receipts")?, 0);

            let duplicate = request(68, 5)?;
            let first = rollback.root.commit_character_experience(
                &authority,
                &rollback.node,
                fence(1)?,
                duplicate.clone(),
            );
            let second = rollback.root.commit_character_experience(
                &authority,
                &rollback.node,
                fence(1)?,
                duplicate,
            );
            let (first, second) = join_two(first, second).await;
            let outcomes = [first, second];
            assert_eq!(
                outcomes
                    .iter()
                    .filter(|outcome| matches!(outcome, Ok(ExperienceCommitOutcome::Committed(_))))
                    .count(),
                1
            );
            assert_eq!(
                outcomes
                    .iter()
                    .filter(|outcome| {
                        matches!(outcome, Ok(ExperienceCommitOutcome::AlreadyCommitted(_)))
                    })
                    .count(),
                1
            );
            drop(authority);
            drop(seal);
            rollback.cleanup().await?;

            let ended = Harness::create(admin, "ended", true).await?;
            let ended_seal = ended
                .recovery
                .seal_current()
                .map_err(|error| format!("{error:?}"))?;
            let ended_authority = ended
                .root
                .open_character_authority(&ended_seal)
                .await
                .map_err(|error| format!("{error:?}"))?;
            ended
                .root
                .revoke_node_registration(ended.node.fact())
                .await
                .map_err(|error| format!("{error:?}"))?;
            assert!(matches!(
                ended
                    .root
                    .commit_character_experience(
                        &ended_authority,
                        &ended.node,
                        fence(1)?,
                        request(69, 5)?,
                    )
                    .await,
                Err(CharacterProgressionError::AuthorityRejected)
            ));
            drop(ended_authority);
            drop(ended_seal);
            ended.cleanup().await
        })
}

#[test]
fn distinct_occurrences_with_one_predecessor_cannot_both_commit() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let harness = Harness::create(admin, "distinct", true).await?;
            let seal = harness
                .recovery
                .seal_current()
                .map_err(|error| format!("{error:?}"))?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(|error| format!("{error:?}"))?;
            let first = harness.root.commit_character_experience(
                &authority,
                &harness.node,
                fence(1)?,
                request(70, 5)?,
            );
            let second = harness.root.commit_character_experience(
                &authority,
                &harness.node,
                fence(1)?,
                request(71, 5)?,
            );
            let (first, second) = join_two(first, second).await;
            let outcomes = [first, second];
            assert_eq!(
                outcomes
                    .iter()
                    .filter(|outcome| matches!(outcome, Ok(ExperienceCommitOutcome::Committed(_))))
                    .count(),
                1
            );
            assert_eq!(
                outcomes
                    .iter()
                    .filter(|outcome| {
                        matches!(
                            outcome,
                            Err(CharacterProgressionError::CharacterRevisionMismatch)
                        )
                    })
                    .count(),
                1
            );
            let root_revision: String =
                sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots")
                    .fetch_one(&harness.pool)
                    .await?;
            assert_eq!(root_revision, "2");
            let receipts: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_xp_receipts")
                    .fetch_one(&harness.pool)
                    .await?;
            assert_eq!(receipts, 1);
            drop(authority);
            drop(seal);
            harness.cleanup().await
        })
}
