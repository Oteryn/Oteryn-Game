#![allow(clippy::expect_used)]
// CHAR-REV-SEQ-1 cases: revision-advancing writers run one at a time per Character through the
// channel runtime's revision sequencer (QUEST-STATE-0 §5.2). Any wrapper that path-loads the
// crate root of `durability_postgres.rs` can include this file. The harness follows the CHARM-3
// cases (one live session, lease, scope assignment and readiness for Character 41).

use crate::character_recovery_fence::CharacterRecoveryStore;
use crate::domain::charm::{
    BestiaryRaceKey, BestiaryStage, CharmCatalogue, CharmCategory, CharmDefinition, CharmKey,
    CharmSlotEntitlement,
};
use crate::domain::progression::{
    FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
};
use crate::domain::{CharacterId, CharacterRevision};
use crate::durability::admission_authority_guards::GuardPublicationDisposition;
use crate::durability::bestiary_progress::{
    BestiaryKillOccurrence, BestiaryKillOutcome, BestiaryKillRequest,
};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_death::{CharacterDeathRequest, DeathCell, PlayerDeathOccurrence};
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, ExperienceRewardOccurrence,
};
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::durability::charm_state::{
    CharmCommand, CharmCommandOccurrence, CharmCommandOutcome, CharmCommandRequest, CharmFacts,
    CharmStateError,
};
use crate::durability::monk_state::{
    DurableMonkState, MonkStateSaveOccurrence, MonkStateSaveOutcome, MonkStateSaveRequest,
};
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, BootstrapSecret, ControlActor,
    LaunchBinding, NodeIncarnationProof, OperationKey, RuntimeScopeAssignmentWriter,
};
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
    AdmissionAuthorityOwningPublisherV1, AdmissionAuthorityPublicationChangeV1,
    AdmissionAuthorityPublicationErrorV1, AdmissionAuthorityPublicationV1,
    AdmissionPublicationPreconditionV1, AdmissionPublicationPurposeV1,
    AdmissionPublicationSourceV1,
};
use crate::foundation::{ConnectionGeneration, RuntimeScopeRefV1, ScopeOwnershipGeneration};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};
use sqlx::postgres::PgConnection;
use sqlx::{Connection, Executor};
use std::collections::BTreeMap;
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

fn debug(error: impl std::fmt::Debug) -> Box<dyn std::error::Error> {
    format!("{error:?}").into()
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
        let name = format!("crs_{name}_{suffix}");
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
        "oteryn-revision-sequencer-parent-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&parent)?;
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700))?;
    let retained = parent.join(tag);
    let _ = std::fs::remove_dir_all(&retained);
    std::fs::create_dir(&retained)?;
    std::fs::set_permissions(&retained, std::fs::Permissions::from_mode(0o700))?;
    Ok(retained)
}

async fn register(root: &DurabilityRoot, tag: u8) -> TestResult<NodeIncarnationProof> {
    let secret = BootstrapSecret::from_bytes([tag; 32]);
    let launch = LaunchBinding::new(&format!("sequencer-launch-{tag}")).map_err(debug)?;
    let node = crate::foundation::NodeId::decode(&id(tag)).map_err(debug)?;
    root.issue_node_bootstrap_authorization(&secret, &launch, None)
        .await
        .map_err(debug)?;
    root.register_node_incarnation(&secret, &launch, node)
        .await
        .map_err(debug)
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
            .map_err(debug)?;
        {
            let fresh = recovery.authorize_fresh_store(id(10), 100).map_err(debug)?;
            root.admit_fresh_character_recovery(&fresh)
                .await
                .map_err(debug)?;
        }
        let node = register(&root, 1).await?;
        seed_character(&pool, &root, &node, initialized).await?;
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
    let mut binding = vec![2];
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
    // Contract version 2 binds the requested name last (CHAR-NAME-1).
    binding.extend_from_slice(&12_u16.to_be_bytes());
    binding.extend_from_slice(b"Fixture Hero");
    binding
}

struct RuntimeReadiness(AdmissionAuthorityPublicationChangeV1);

impl crate::foundation::fnd04_verifier::fresh_source_sealed::Sealed for RuntimeReadiness {}

impl AdmissionAuthorityOwningPublisherV1 for RuntimeReadiness {
    fn resolve_publication(
        &self,
        _now: i64,
    ) -> Result<Vec<AdmissionAuthorityPublicationChangeV1>, AdmissionAuthorityPublicationErrorV1>
    {
        Ok(vec![self.0.clone()])
    }
}

async fn publish_readiness(
    pool: &sqlx::PgPool,
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    scope: RuntimeScopeRefV1,
    ownership_generation: u64,
) -> TestResult {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
            .fetch_one(pool)
            .await?;
    let change = AdmissionAuthorityPublicationChangeV1 {
        key: AdmissionAuthorityGuardKeyV1::Runtime(scope),
        source: AdmissionPublicationSourceV1 {
            authority: "game-runtime-publisher".into(),
            purpose: AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness,
            source_revision: 1,
            decision_identity: "sequencer-runtime-ready-1".into(),
            source_observed_at: now,
            clock_uncertainty_seconds: 0,
        },
        precondition: AdmissionPublicationPreconditionV1::Bootstrap {
            restored_publication_high_water: Some(0),
        },
        publication_revision: 1,
        state: AdmissionAuthorityGuardStateV1::Runtime {
            ownership_generation,
            ready: true,
            route_revision: "route-1".into(),
            runtime_observation_revision: "runtime-1".into(),
            protocol_major: 1,
            transport_profile: 1,
            ruleset_revision: "ruleset-1".into(),
            content_revision: "content-1".into(),
            map_revision: "map-1".into(),
            world_policy_revision: "world-policy-1".into(),
            offer_revision: "offer-1".into(),
        },
    };
    let publication =
        AdmissionAuthorityPublicationV1::prepare(&RuntimeReadiness(change), now).map_err(debug)?;
    let disposition = root
        .publish_runtime_readiness(node, &publication)
        .await
        .map_err(debug)?;
    if disposition != GuardPublicationDisposition::Applied {
        return Err(format!("unexpected readiness outcome: {disposition:?}").into());
    }
    Ok(())
}

/// One Character (id 41) with a live session, lease, scope assignment and readiness, and, when
/// `initialized`, progression at revision one (level 50, 1000 experience).
async fn seed_character(
    pool: &sqlx::PgPool,
    root: &DurabilityRoot,
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
          1,1,'profile-1','ruleset-1','content-1','starter-1','Fixture Hero')",
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
    let fact = node.fact();
    sqlx::query(
        "INSERT INTO game_control_scope_grants \
         (control_role, world_id, channel_id, operation) \
         VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1)",
    )
    .bind(id(42).as_slice())
    .bind(id(43).as_slice())
    .execute(pool)
    .await?;
    let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "sequencer-writer")
        .await
        .map_err(debug)?;
    let scope = RuntimeScopeRefV1::channel(
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
    );
    let assignment = writer
        .submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([7_u8; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            command: AssignmentCommand::Assign {
                scope,
                target: fact,
            },
        })
        .await
        .map_err(debug)?;
    let AssignmentOutcome::Committed(receipt) = assignment else {
        return Err(format!("unexpected assignment outcome: {assignment:?}").into());
    };
    publish_readiness(
        pool,
        root,
        node,
        scope,
        receipt.assignment.ownership_generation,
    )
    .await?;
    Ok(())
}

fn fence(revision: u64) -> TestResult<CurrentCharacterGameplayFence> {
    Ok(CurrentCharacterGameplayFence {
        character_id: CharacterId::from_bytes(id(41)).map_err(debug)?,
        game_session_id: crate::foundation::GameSessionId::decode(&id(50)).map_err(debug)?,
        connection_generation: ConnectionGeneration::new(1).map_err(debug)?,
        character_lease_generation: 1,
        runtime_scope: RuntimeScopeRefV1::channel(
            crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
            crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        ),
        scope_ownership_generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
        expected_character_revision: CharacterRevision::new(revision).map_err(debug)?,
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

fn charm(name: &str) -> CharmKey {
    CharmKey::new(format!("oteryn:charm.{name}")).expect("charm key")
}

fn race(name: &str) -> BestiaryRaceKey {
    BestiaryRaceKey::new(format!("oteryn:creature.{name}")).expect("race key")
}

/// Candidate-catalogue costs (#1293) for three majors and two minors.
fn catalogue() -> CharmCatalogue {
    let major = |name: &str, costs| CharmDefinition {
        key: charm(name),
        category: CharmCategory::Major,
        stage_costs: costs,
    };
    let minor = |name: &str| CharmDefinition {
        key: charm(name),
        category: CharmCategory::Minor,
        stage_costs: [100, 150, 225],
    };
    CharmCatalogue::new([
        major("wound", [240, 360, 1200]),
        major("poison", [240, 360, 1200]),
        major("zap", [320, 480, 1600]),
        minor("gut"),
        minor("bless"),
    ])
    .expect("catalogue")
}

fn occurrence(tag: u8) -> CharmCommandOccurrence {
    CharmCommandOccurrence::from_bytes(id(tag)).expect("occurrence")
}

fn unlock(tag: u8, name: &str) -> CharmCommandRequest {
    CharmCommandRequest {
        occurrence: occurrence(tag),
        command: CharmCommand::UnlockNextStage { charm: charm(name) },
        catalogue_revision: "content-1".into(),
        catalogue: catalogue(),
    }
}

/// Test double for the facts CHARM-2 and later slices own. It reads nothing from the
/// transaction except a liveness probe on it, so a closed connection still fails the command.
#[derive(Clone)]
struct Facts {
    stages: BTreeMap<BestiaryRaceKey, u8>,
    points: Vec<u32>,
    promoted: bool,
    slots: CharmSlotEntitlement,
}

impl Facts {
    fn new(points: &[u32], stages: &[(&str, u8)]) -> Self {
        Self {
            stages: stages
                .iter()
                .map(|(name, stage)| (race(name), *stage))
                .collect(),
            points: points.to_vec(),
            promoted: false,
            slots: CharmSlotEntitlement::Free,
        }
    }
}

impl CharmFacts for Facts {
    async fn completed_stage(
        &self,
        connection: &mut PgConnection,
        _character: CharacterId,
        race: &BestiaryRaceKey,
    ) -> Result<BestiaryStage, DurabilityError> {
        sqlx::query("SELECT 1").execute(&mut *connection).await?;
        BestiaryStage::new(self.stages.get(race).copied().unwrap_or(0))
            .map_err(|_| DurabilityError::InvalidStoredState)
    }

    async fn completed_entry_charm_points(
        &self,
        connection: &mut PgConnection,
        _character: CharacterId,
    ) -> Result<Vec<u32>, DurabilityError> {
        sqlx::query("SELECT 1").execute(&mut *connection).await?;
        Ok(self.points.clone())
    }

    async fn promoted(
        &self,
        _connection: &mut PgConnection,
        _character: CharacterId,
    ) -> Result<bool, DurabilityError> {
        Ok(self.promoted)
    }

    async fn slot_entitlement(
        &self,
        _connection: &mut PgConnection,
        _character: CharacterId,
    ) -> Result<CharmSlotEntitlement, DurabilityError> {
        Ok(self.slots)
    }
}

fn xp_request(tag: u8) -> TestResult<ExperienceAwardRequest<2>> {
    let context = ProgressionRevisionContext {
        profile: "profile-1".into(),
        ruleset: "ruleset-1".into(),
        content: "content-1".into(),
        simulation: "simulation-1".into(),
        evidence: "evidence-1".into(),
        declaration: "declaration-1".into(),
    };
    Ok(ExperienceAwardRequest {
        occurrence: ExperienceRewardOccurrence::from_bytes(id(tag)).map_err(debug)?,
        amount: ExactI64::new(5),
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

fn run<F>(body: F) -> TestResult
where
    F: AsyncFnOnce(String) -> TestResult,
{
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(body(admin))
}

async fn revision(pool: &sqlx::PgPool) -> TestResult<u64> {
    let text: String =
        sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots")
            .fetch_one(pool)
            .await?;
    Ok(text.parse()?)
}

fn kill(tag: u8) -> TestResult<BestiaryKillRequest> {
    Ok(BestiaryKillRequest {
        occurrence: BestiaryKillOccurrence::from_bytes(id(tag)).map_err(debug)?,
        race: crate::domain::bestiary::BestiaryRace::new(
            "oteryn:creature.rat",
            "definition-r1",
            vec![5, 10, 20],
        )
        .map_err(debug)?,
        context: xp_request(tag)?.context,
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
    })
}

fn death(tag: u8) -> TestResult<CharacterDeathRequest<2>> {
    let xp = xp_request(tag)?;
    Ok(CharacterDeathRequest {
        occurrence: PlayerDeathOccurrence::from_bytes(id(tag)).map_err(debug)?,
        context: xp.context,
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
        policy: xp.policy,
        held_blessings: Vec::new(),
        death_cell: DeathCell {
            world_id: crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
            channel_id: crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
            spatial_position: vec![1, 2, 3, 7],
            map_revision: "map-1".into(),
        },
        respawn_position: b"temple:fixture".to_vec(),
    })
}

fn monk(tag: u8) -> TestResult<MonkStateSaveRequest> {
    Ok(MonkStateSaveRequest {
        occurrence: MonkStateSaveOccurrence::from_bytes(id(tag)).map_err(debug)?,
        state: DurableMonkState::new(3, 0).map_err(debug)?,
    })
}

/// A writer that bypasses the sequencer: one XP award straight at the current revision.
async fn bypass(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    tag: u8,
) -> TestResult {
    let current = revision(&harness.pool).await?;
    let outcome = harness
        .root
        .commit_character_experience(authority, &harness.node, fence(current)?, xp_request(tag)?)
        .await
        .map_err(debug)?;
    assert!(
        matches!(outcome, ExperienceCommitOutcome::Committed(_)),
        "{outcome:?}"
    );
    Ok(())
}

#[test]
fn concurrent_xp_and_charm_on_one_character_commit_in_sequence_without_mismatch() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "concurrent", true).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let sequencer = CharacterRevisionSequencer::new();
        let facts = Facts::new(&[300], &[]);
        // Both requests carry the same stale revision: the slot, not the caller, picks it.
        let xp = async {
            let mut slot = sequencer.acquire(fence(1)?.character_id).await;
            slot.commit_experience(
                &harness.root,
                &authority,
                &harness.node,
                fence(1)?,
                xp_request(70)?,
                None,
            )
            .await
            .map_err(debug)
        };
        let charm = async {
            let mut slot = sequencer.acquire(fence(1)?.character_id).await;
            slot.commit_charm(
                &harness.root,
                &authority,
                &harness.node,
                fence(1)?,
                unlock(71, "wound"),
                facts.clone(),
                None,
            )
            .await
            .map_err(debug)
        };
        let (xp, charm) = join_two(xp, charm).await;
        let (xp, charm) = (xp?, charm?);
        let ExperienceCommitOutcome::Committed(xp) = xp else {
            return Err(format!("XP did not commit: {xp:?}").into());
        };
        let CharmCommandOutcome::Committed(charm) = charm else {
            return Err(format!("charm did not commit: {charm:?}").into());
        };
        let mut chain = [
            (
                xp.original_character_revision.get(),
                xp.committed_character_revision.get(),
            ),
            (
                charm.original_character_revision.get(),
                charm.committed_character_revision.get(),
            ),
        ];
        chain.sort_unstable();
        assert_eq!(chain, [(1, 2), (2, 3)]);
        assert_eq!(revision(&harness.pool).await?, 3);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn a_death_chain_holds_the_slot_and_bestiary_takes_the_revision_xp_committed() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "chain", true).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let sequencer = CharacterRevisionSequencer::new();
        let character = fence(1)?.character_id;
        let chain = async {
            let mut slot = sequencer.acquire(character).await;
            assert_eq!(slot.character_id(), character);
            let xp = slot
                .commit_experience(
                    &harness.root,
                    &authority,
                    &harness.node,
                    fence(1)?,
                    xp_request(72)?,
                    None,
                )
                .await
                .map_err(debug)?;
            // Let the waiting writer run if the slot did not hold it back.
            tokio::task::yield_now().await;
            let bestiary = slot
                .commit_bestiary(
                    &harness.root,
                    &authority,
                    &harness.node,
                    fence(1)?,
                    kill(72)?,
                )
                .await
                .map_err(debug)?;
            TestResult::Ok((xp, bestiary))
        };
        let waiting = async {
            tokio::task::yield_now().await;
            let mut slot = sequencer.acquire(character).await;
            slot.commit_monk_state_save(
                &harness.root,
                &authority,
                &harness.node,
                fence(1)?,
                monk(73)?,
            )
            .await
            .map_err(debug)
        };
        let (chain, waiting) = join_two(chain, waiting).await;
        let (xp, bestiary) = chain?;
        let ExperienceCommitOutcome::Committed(xp) = xp else {
            return Err(format!("XP did not commit: {xp:?}").into());
        };
        let BestiaryKillOutcome::Committed(bestiary) = bestiary else {
            return Err(format!("Bestiary did not commit: {bestiary:?}").into());
        };
        assert_eq!(xp.committed_character_revision.get(), 2);
        assert_eq!(
            bestiary.original_character_revision,
            xp.committed_character_revision
        );
        assert_eq!(bestiary.committed_character_revision.get(), 3);
        let MonkStateSaveOutcome::Committed(save) = waiting? else {
            return Err("the waiting monk save did not commit".into());
        };
        assert_eq!(
            save.original_character_revision.get(),
            3,
            "it ran after the chain"
        );
        assert_eq!(revision(&harness.pool).await?, 4);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn a_bestiary_mismatch_reloads_the_cursor_and_retries_once() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "bestiary_retry", true).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let sequencer = CharacterRevisionSequencer::new();
        let mut slot = sequencer.acquire(fence(1)?.character_id).await;
        assert_eq!(
            slot.cursor(&harness.root, &authority)
                .await
                .map_err(debug)?
                .get(),
            1
        );
        bypass(&harness, &authority, 74).await?;
        let outcome = slot
            .commit_bestiary(
                &harness.root,
                &authority,
                &harness.node,
                fence(1)?,
                kill(75)?,
            )
            .await
            .map_err(debug)?;
        let BestiaryKillOutcome::Committed(credited) = outcome else {
            return Err(format!("Bestiary did not commit: {outcome:?}").into());
        };
        assert_eq!(
            credited.original_character_revision.get(),
            2,
            "the reloaded cursor"
        );
        assert_eq!(revision(&harness.pool).await?, 3);

        // The retry is per request: a later request under the same slot gets its own one retry.
        bypass(&harness, &authority, 76).await?;
        bypass(&harness, &authority, 77).await?;
        let result = slot
            .commit_bestiary(
                &harness.root,
                &authority,
                &harness.node,
                fence(1)?,
                kill(78)?,
            )
            .await;
        assert!(result.is_ok(), "{result:?}");
        drop(slot);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn revision_bound_writers_fail_closed_without_retry() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "fail_closed", true).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let sequencer = CharacterRevisionSequencer::new();
        let facts = Facts::new(&[300], &[]);
        for (case, tag) in [("xp", 80), ("death", 82), ("charm", 84), ("monk", 86)] {
            let mut slot = sequencer.acquire(fence(1)?.character_id).await;
            slot.cursor(&harness.root, &authority)
                .await
                .map_err(debug)?;
            bypass(&harness, &authority, tag).await?;
            let before = revision(&harness.pool).await?;
            let base = fence(1)?;
            let mismatch = match case {
                "xp" => matches!(
                    slot.commit_experience(
                        &harness.root,
                        &authority,
                        &harness.node,
                        base,
                        xp_request(tag + 1)?,
                        None
                    )
                    .await,
                    Err(CharacterProgressionError::CharacterRevisionMismatch)
                ),
                "death" => matches!(
                    slot.commit_death(
                        &harness.root,
                        &authority,
                        &harness.node,
                        base,
                        death(tag + 1)?,
                        None
                    )
                    .await,
                    Err(CharacterProgressionError::CharacterRevisionMismatch)
                ),
                "charm" => matches!(
                    slot.commit_charm(
                        &harness.root,
                        &authority,
                        &harness.node,
                        base,
                        unlock(tag + 1, "wound"),
                        facts.clone(),
                        None,
                    )
                    .await,
                    Err(CharmStateError::CharacterRevisionMismatch)
                ),
                _ => matches!(
                    slot.commit_monk_state_save(
                        &harness.root,
                        &authority,
                        &harness.node,
                        base,
                        monk(tag + 1)?
                    )
                    .await,
                    Err(CharacterProgressionError::CharacterRevisionMismatch)
                ),
            };
            assert!(mismatch, "{case}: a stale cursor must fail closed");
            assert_eq!(
                revision(&harness.pool).await?,
                before,
                "{case}: no retry committed"
            );
        }
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
