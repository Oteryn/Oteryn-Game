// Shared CHARM-3 cases (migration 0020). Any wrapper that provides the same path-loaded crate
// root as `charm_state_postgres.rs` can include this file.

use crate::character_recovery_fence::CharacterRecoveryStore;
use crate::charm_transport::CharmPortUnavailable;
use crate::charm_transport::native::{CharmConnectionContent, NativeCharmProgressionPort};
use crate::domain::charm::{
    BestiaryRaceKey, BestiaryStage, CharmCatalogue, CharmCategory, CharmCurrency, CharmDefinition,
    CharmKey, CharmRuleError, CharmSlotEntitlement, CharmStage, derive_balance,
};
use crate::domain::progression::{
    FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
};
use crate::domain::{CharacterId, CharacterRevision};
use crate::durability::admission_authority_guards::GuardPublicationDisposition;
use crate::durability::bestiary_progress::{
    BestiaryKillOccurrence, BestiaryKillOutcome, BestiaryKillRequest,
};
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, ExperienceAwardRequest, ExperienceCommitOutcome,
    ExperienceRewardOccurrence,
};
use crate::durability::charm_state::{
    BestiaryCharmEntry, BestiaryCharmFacts, CharmCommand, CharmCommandEffect,
    CharmCommandOccurrence, CharmCommandOutcome, CharmCommandRequest, CharmFacts,
    CharmProgressionReadRequest, CharmStateError,
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
        let name = format!("chs_{name}_{suffix}");
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
    let parent =
        std::env::temp_dir().join(format!("oteryn-charm-state-parent-{}", std::process::id()));
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
    let launch = LaunchBinding::new(&format!("charm-launch-{tag}")).map_err(debug)?;
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
            decision_identity: "charm-runtime-ready-1".into(),
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
    let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "charm-writer")
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

// Validated provider fixtures exercise native binding; they do not claim production Content.
fn native_content(
    races: Vec<crate::domain::bestiary::BestiaryRace>,
) -> TestResult<CharmConnectionContent> {
    use crate::combat::charm_effects::{
        CharmCatalogueRead, CharmCategory as EffectCategory, CharmDefinition as EffectDefinition,
        CharmEffect, CharmPercent, CharmStageValue,
    };
    struct Effects(BTreeMap<String, EffectDefinition>);
    impl CharmCatalogueRead for Effects {
        fn charm(&self, key: &str) -> Option<&EffectDefinition> {
            self.0.get(key)
        }
    }
    let catalogue = catalogue();
    let mut effects = Effects(BTreeMap::new());
    for definition in catalogue.definitions() {
        let category = match definition.category {
            CharmCategory::Major => EffectCategory::Major,
            CharmCategory::Minor => EffectCategory::Minor,
        };
        let effect = EffectDefinition::new(
            definition.key.as_str(),
            category,
            CharmStageValue::TriggerChancePercent,
            [100, 200, 300].map(|value| CharmPercent::from_hundredths(value).expect("percent")),
            CharmEffect::DodgeAttack,
        )
        .map_err(debug)?;
        effects.0.insert(definition.key.as_str().to_owned(), effect);
    }
    CharmConnectionContent::new("content-1".into(), catalogue, races, &effects).map_err(debug)
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

fn assign(tag: u8, name: &str, to: &str) -> CharmCommandRequest {
    CharmCommandRequest {
        occurrence: occurrence(tag),
        command: CharmCommand::Assign {
            charm: charm(name),
            race: race(to),
        },
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

async fn revision(pool: &sqlx::PgPool) -> TestResult<String> {
    Ok(
        sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots")
            .fetch_one(pool)
            .await?,
    )
}

async fn committed(
    harness: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    revision: u64,
    request: CharmCommandRequest,
    facts: &Facts,
) -> TestResult<crate::durability::charm_state::CommittedCharmCommand> {
    match harness
        .root
        .commit_charm_command(
            authority,
            &harness.node,
            fence(revision)?,
            request,
            facts.clone(),
        )
        .await
        .map_err(|error| format!("commit at revision {revision}: {error:?}"))?
    {
        CharmCommandOutcome::Committed(committed) => Ok(committed),
        outcome => Err(format!("expected a new commit, got {outcome:?}").into()),
    }
}

/// One rejected command: the given error and no durable change (revision stays).
async fn rejected(
    harness: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    fence: CurrentCharacterGameplayFence,
    request: CharmCommandRequest,
    facts: &Facts,
    expected: impl Fn(&CharmStateError) -> bool,
    case: &str,
) -> TestResult {
    let before = revision(&harness.pool).await?;
    let result = harness
        .root
        .commit_charm_command(authority, &harness.node, fence, request, facts.clone())
        .await;
    match &result {
        Err(error) if expected(error) => {}
        other => return Err(format!("{case}: unexpected outcome {other:?}").into()),
    }
    if revision(&harness.pool).await? != before {
        return Err(format!("{case}: a rejected command advanced the revision").into());
    }
    Ok(())
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

#[test]
fn unlock_assign_replay_reconcile_and_restart_readback_are_durable() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "replay", true).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let facts = Facts::new(&[15, 585], &[("rat", 3), ("wolf", 2)]);

        let first = committed(&harness, &authority, 1, unlock(60, "wound"), &facts).await?;
        assert_eq!(first.committed_character_revision.get(), 2);
        assert_eq!(first.category, CharmCategory::Major);
        assert_eq!(
            first.effect,
            CharmCommandEffect::Unlocked {
                stage_before: 0,
                stage_after: CharmStage::FIRST,
                cost: 240,
            }
        );
        // Exact replay returns the retained result, even at a stale revision.
        let replay = harness
            .root
            .commit_charm_command(
                &authority,
                &harness.node,
                fence(1)?,
                unlock(60, "wound"),
                facts.clone(),
            )
            .await
            .map_err(debug)?;
        assert_eq!(replay, CharmCommandOutcome::AlreadyCommitted(first.clone()));
        // The same occurrence with other semantics conflicts.
        rejected(
            &harness,
            &authority,
            fence(1)?,
            assign(60, "wound", "rat"),
            &facts,
            |error| matches!(error, CharmStateError::ConflictingOccurrence),
            "reused occurrence",
        )
        .await?;

        let second = committed(&harness, &authority, 2, unlock(61, "wound"), &facts).await?;
        assert_eq!(
            second.effect,
            CharmCommandEffect::Unlocked {
                stage_before: 1,
                stage_after: CharmStage::new(2).map_err(debug)?,
                cost: 360,
            }
        );
        // The 150 echoes earned by the two major stages buy the first minor stage.
        let minor = committed(&harness, &authority, 3, unlock(62, "gut"), &facts).await?;
        assert_eq!(minor.category, CharmCategory::Minor);
        // One major and one minor on the same race.
        let major_on_rat =
            committed(&harness, &authority, 4, assign(63, "wound", "rat"), &facts).await?;
        assert_eq!(
            major_on_rat.effect,
            CharmCommandEffect::Assigned { race: race("rat") }
        );
        committed(&harness, &authority, 5, assign(64, "gut", "rat"), &facts).await?;
        let reconciled = harness
            .root
            .reconcile_charm_command(&authority, occurrence(63))
            .await
            .map_err(debug)?;
        assert_eq!(reconciled, Some(major_on_rat));
        assert_eq!(
            harness
                .root
                .reconcile_charm_command(&authority, occurrence(99))
                .await
                .map_err(debug)?,
            None
        );

        // An XP award continues the same CharacterRevision chain.
        let xp = harness
            .root
            .commit_character_experience(&authority, &harness.node, fence(6)?, xp_request(70)?)
            .await
            .map_err(debug)?;
        let ExperienceCommitOutcome::Committed(xp) = xp else {
            return Err("XP award after charm commands was not committed".into());
        };
        assert_eq!(xp.committed_character_revision.get(), 7);
        committed(
            &harness,
            &authority,
            7,
            unlock(65, "poison"),
            &Facts::new(&[1000], &[]),
        )
        .await?;

        let state = harness
            .root
            .read_character_charm_state(&authority, CharacterId::from_bytes(id(41)).map_err(debug)?)
            .await
            .map_err(debug)?;
        assert_expected_state(&state)?;
        let counts: (String, i64, i64, i64, i64) = sqlx::query_as(
            "SELECT (SELECT character_revision::text FROM game_character_roots), \
                    (SELECT count(*) FROM game_character_charm_receipts), \
                    (SELECT count(*) FROM game_character_xp_receipts), \
                    (SELECT count(*) FROM game_character_charm_unlocks), \
                    (SELECT count(*) FROM game_character_charm_assignments)",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(counts, ("8".to_owned(), 6, 1, 3, 2));
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

fn assert_expected_state(
    state: &crate::durability::charm_state::CharacterCharmState,
) -> TestResult {
    assert_eq!(
        state.unlocks,
        BTreeMap::from([
            (charm("wound"), CharmStage::new(2).map_err(debug)?),
            (charm("gut"), CharmStage::FIRST),
            (charm("poison"), CharmStage::FIRST),
        ])
    );
    assert_eq!(
        state.assignments,
        BTreeMap::from([(charm("wound"), race("rat")), (charm("gut"), race("rat"))])
    );
    let balance = derive_balance(&catalogue(), &state.unlocks, [1000], false).map_err(debug)?;
    assert_eq!(balance.points_spent, 240 + 360 + 240);
    assert_eq!(balance.echoes_earned, 50 + 100 + 50);
    assert_eq!(balance.available(CharmCurrency::MinorCharmEchoes), 100);
    Ok(())
}

/// Restart readback: `open_character_authority` runs `verify_character_integrity`, whose
/// receipt chain must include the charm receipts, or no Character authority opens after the
/// first charm command.
#[test]
fn restart_readback_after_charm_commands() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "restart", true).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let facts = Facts::new(&[1000], &[("rat", 3)]);
        for (revision, request) in [
            (1, unlock(60, "wound")),
            (2, unlock(61, "wound")),
            (3, unlock(62, "gut")),
            (4, assign(63, "wound", "rat")),
            (5, assign(64, "gut", "rat")),
            (6, unlock(65, "poison")),
        ] {
            committed(&harness, &authority, revision, request, &facts).await?;
        }
        let restarted = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let restart_seal = harness.recovery.seal_current().map_err(debug)?;
        let restart_authority = restarted
            .open_character_authority(&restart_seal)
            .await
            .map_err(debug)?;
        let state = restarted
            .read_character_charm_state(
                &restart_authority,
                CharacterId::from_bytes(id(41)).map_err(debug)?,
            )
            .await
            .map_err(debug)?;
        assert_expected_state(&state)?;
        drop(restart_authority);
        drop(restart_seal);
        drop(restarted);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// The production facts over real CHARM-2 kill counters: kills and charm commands share one
/// CharacterRevision chain, a completed entry earns its charm points, and a restart reopens
/// Character authority over both receipt kinds.
#[test]
fn bestiary_kill_counters_earn_points_and_admit_assignments() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "bestiary", true).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let bestiary_race = |name: &str| {
            crate::domain::bestiary::BestiaryRace::new(
                format!("oteryn:creature.{name}"),
                "definition-r1",
                vec![1, 2, 3],
            )
            .map_err(debug)
        };
        let facts = BestiaryCharmFacts::new([
            BestiaryCharmEntry {
                race: bestiary_race("rat")?,
                charm_points: 300,
            },
            BestiaryCharmEntry {
                race: bestiary_race("wolf")?,
                charm_points: 300,
            },
        ])
        .map_err(debug)?;
        let context = xp_request(1)?.context;
        let mut kill_revision = 1;
        for (tag, name) in [
            (80, "rat"),
            (81, "rat"),
            (82, "rat"),
            (83, "wolf"),
            (84, "wolf"),
        ] {
            let outcome = harness
                .root
                .commit_bestiary_kill(
                    &authority,
                    &harness.node,
                    fence(kill_revision)?,
                    BestiaryKillRequest {
                        occurrence: BestiaryKillOccurrence::from_bytes(id(tag)).map_err(debug)?,
                        race: bestiary_race(name)?,
                        context: context.clone(),
                        policy_revision: "policy-1".into(),
                        reward_revision: "reward-1".into(),
                    },
                )
                .await
                .map_err(debug)?;
            assert!(
                matches!(outcome, BestiaryKillOutcome::Committed(_)),
                "{outcome:?}"
            );
            kill_revision += 1;
        }
        // Only the completed rat entry earns points: its 300 buy wound stage 1 (240); zap
        // (320) would then need 560.
        let unlocked = harness
            .root
            .commit_charm_command(
                &authority,
                &harness.node,
                fence(6)?,
                unlock(61, "wound"),
                facts.clone(),
            )
            .await
            .map_err(debug)?;
        assert!(matches!(unlocked, CharmCommandOutcome::Committed(_)));
        let short = harness
            .root
            .commit_charm_command(
                &authority,
                &harness.node,
                fence(7)?,
                unlock(62, "zap"),
                facts.clone(),
            )
            .await;
        assert!(
            matches!(
                short,
                Err(CharmStateError::Rule(CharmRuleError::InsufficientBalance))
            ),
            "{short:?}"
        );
        // Wolf is at stage 2 of 3: no major charm.
        let incomplete = harness
            .root
            .commit_charm_command(
                &authority,
                &harness.node,
                fence(7)?,
                assign(63, "wound", "wolf"),
                facts.clone(),
            )
            .await;
        assert!(
            matches!(
                incomplete,
                Err(CharmStateError::Rule(CharmRuleError::BestiaryStageTooLow))
            ),
            "{incomplete:?}"
        );
        let assigned = harness
            .root
            .commit_charm_command(
                &authority,
                &harness.node,
                fence(7)?,
                assign(64, "wound", "rat"),
                facts.clone(),
            )
            .await
            .map_err(debug)?;
        assert!(matches!(assigned, CharmCommandOutcome::Committed(_)));
        assert_eq!(revision(&harness.pool).await?, "8");

        let read_request = || CharmProgressionReadRequest {
            catalogue_revision: "content-1".into(),
            catalogue: catalogue(),
            races: vec![race("rat"), race("wolf")],
        };
        let view = harness
            .root
            .read_character_charm_progression(
                &authority,
                &harness.node,
                fence(8)?,
                read_request(),
                facts.clone(),
            )
            .await
            .map_err(debug)?;
        assert_eq!(view.character_revision.get(), 8);
        assert_eq!(
            view.bestiary_counts,
            BTreeMap::from([(race("rat"), 3), (race("wolf"), 2)])
        );
        assert_eq!(view.balance.available(CharmCurrency::CharmPoints), 60);
        assert_eq!(view.balance.available(CharmCurrency::MinorCharmEchoes), 50);
        assert_eq!(view.slot_entitlement, CharmSlotEntitlement::Free);
        let content = native_content(vec![bestiary_race("wolf")?, bestiary_race("rat")?])?;
        let port = NativeCharmProgressionPort::bind(
            &harness.root,
            &authority,
            &harness.node,
            fence(8)?,
            &content,
            facts.clone(),
        )
        .await
        .map_err(debug)?;
        let native_view = port.views().await.map_err(debug)?;
        assert_eq!(native_view.revision.get(), 8);
        assert_eq!(
            native_view
                .bestiary
                .iter()
                .map(|race| (race.race.get(), race.kill_count))
                .collect::<Vec<_>>(),
            [(1, 3), (2, 2)]
        );
        assert_eq!(native_view.charms.charm_points_available, 60);
        assert_eq!(native_view.charms.minor_charm_echoes_available, 50);
        let wound = &native_view.charms.charms[3];
        assert_eq!(
            (
                wound.charm.get(),
                wound.unlocked_stage,
                wound.assigned_race.map(std::num::NonZeroU32::get)
            ),
            (4, 1, Some(1))
        );
        assert!(
            native_view
                .charms
                .charms
                .iter()
                .all(|charm| !charm.effect_active)
        );
        let mut mismatched_session = fence(8)?;
        mismatched_session.game_session_id =
            crate::foundation::GameSessionId::decode(&id(51)).map_err(debug)?;
        for refused_fence in [mismatched_session, fence(7)?] {
            assert!(matches!(
                NativeCharmProgressionPort::bind(
                    &harness.root,
                    &authority,
                    &harness.node,
                    refused_fence,
                    &content,
                    facts.clone(),
                )
                .await,
                Err(CharmPortUnavailable)
            ));
        }
        let mut maximal = read_request();
        maximal
            .races
            .extend((0..1022).map(|i| race(&format!("race{i}"))));
        let maximum = harness
            .root
            .read_character_charm_progression(
                &authority,
                &harness.node,
                fence(8)?,
                maximal,
                facts.clone(),
            )
            .await
            .map_err(debug)?;
        assert_eq!(maximum, view);

        // Each negative changes one binding fact; every other live fact remains valid.
        let mut wrong_character = fence(8)?;
        wrong_character.character_id = CharacterId::from_bytes(id(99)).map_err(debug)?;
        let mut wrong_session = fence(8)?;
        wrong_session.game_session_id =
            crate::foundation::GameSessionId::decode(&id(99)).map_err(debug)?;
        let mut replaced_connection = fence(8)?;
        replaced_connection.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        let mut wrong_lease = fence(8)?;
        wrong_lease.character_lease_generation = 2;
        let mut wrong_scope = fence(8)?;
        wrong_scope.runtime_scope = RuntimeScopeRefV1::channel(
            crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
            crate::foundation::ChannelId::decode(&id(99)).map_err(debug)?,
        );
        let mut wrong_scope_generation = fence(8)?;
        wrong_scope_generation.scope_ownership_generation =
            ScopeOwnershipGeneration::new(2).map_err(debug)?;
        for invalid in [
            wrong_character,
            wrong_session,
            replaced_connection,
            wrong_lease,
            wrong_scope,
            wrong_scope_generation,
        ] {
            let refused = harness
                .root
                .read_character_charm_progression(
                    &authority,
                    &harness.node,
                    invalid,
                    read_request(),
                    facts.clone(),
                )
                .await;
            assert!(
                matches!(refused, Err(CharmStateError::AuthorityRejected)),
                "{refused:?}"
            );
        }
        let stale = harness
            .root
            .read_character_charm_progression(
                &authority,
                &harness.node,
                fence(7)?,
                read_request(),
                facts.clone(),
            )
            .await;
        assert!(matches!(
            stale,
            Err(CharmStateError::CharacterRevisionMismatch)
        ));
        let mut other_content = read_request();
        other_content.catalogue_revision = "other-content".into();
        let refused = harness
            .root
            .read_character_charm_progression(
                &authority,
                &harness.node,
                fence(8)?,
                other_content,
                facts.clone(),
            )
            .await;
        assert!(matches!(
            refused,
            Err(CharmStateError::CharmContextMismatch)
        ));
        for races in [
            vec![race("rat"), race("rat")],
            (0..1025).map(|i| race(&format!("race{i}"))).collect(),
        ] {
            let mut oversized = read_request();
            oversized.races = races;
            let refused = harness
                .root
                .read_character_charm_progression(
                    &authority,
                    &harness.node,
                    fence(8)?,
                    oversized,
                    facts.clone(),
                )
                .await;
            assert!(matches!(refused, Err(CharmStateError::InvalidInput)));
        }
        // A current-generation read omits historical keys, without deleting their counters.
        let mut current_only = read_request();
        current_only.races = vec![race("rat")];
        let filtered = harness
            .root
            .read_character_charm_progression(
                &authority,
                &harness.node,
                fence(8)?,
                current_only,
                facts.clone(),
            )
            .await
            .map_err(debug)?;
        assert_eq!(filtered.bestiary_counts, BTreeMap::from([(race("rat"), 3)]));
        let historical = harness
            .root
            .read_bestiary_progress(&authority, fence(8)?.character_id, race("wolf").as_str())
            .await
            .map_err(debug)?
            .ok_or("historical wolf counter disappeared")?;
        assert_eq!(historical.kill_count, 2);

        let restarted = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let restart_seal = harness.recovery.seal_current().map_err(debug)?;
        let restart_authority = restarted
            .open_character_authority(&restart_seal)
            .await
            .map_err(debug)?;
        let state = restarted
            .read_character_charm_state(
                &restart_authority,
                CharacterId::from_bytes(id(41)).map_err(debug)?,
            )
            .await
            .map_err(debug)?;
        assert_eq!(
            state.unlocks,
            BTreeMap::from([(charm("wound"), CharmStage::FIRST)])
        );
        assert_eq!(
            state.assignments,
            BTreeMap::from([(charm("wound"), race("rat"))])
        );
        let reloaded = restarted
            .read_character_charm_progression(
                &restart_authority,
                &harness.node,
                fence(8)?,
                read_request(),
                facts.clone(),
            )
            .await
            .map_err(debug)?;
        assert_eq!(reloaded, view);
        assert_eq!(revision(&harness.pool).await?, "8");
        drop(restart_authority);
        drop(restart_seal);
        drop(restarted);
        // Independently replace the live connection generation. The already-bound port must
        // reject its old evidence rather than reconstructing current authority from storage.
        let replaced = sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation = 2 WHERE game_session_id = encode($1,'hex')::uuid")
            .bind(id(50).as_slice()).execute(&harness.pool).await?;
        assert_eq!(replaced.rows_affected(), 1);
        assert_eq!(port.views().await, Err(CharmPortUnavailable));
        drop(port);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// A login in the runtime group (0006): what a deployed GameNode connects as.
async fn runtime_login(harness: &Harness, tag: &str) -> TestResult<(String, String)> {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let role = format!("charm_runtime_{tag}_{suffix}");
    let password = format!("{role}-secret");
    for statement in [
        format!("CREATE ROLE {role} LOGIN PASSWORD '{password}' IN ROLE oteryn_game_runtime"),
        format!(
            "GRANT CONNECT ON DATABASE {} TO {role}",
            harness.database.name
        ),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(statement))
            .execute(&harness.pool)
            .await?;
    }
    let (_, address) = harness
        .database
        .url
        .split_once('@')
        .ok_or("database URL has no authority separator")?;
    Ok((
        role.clone(),
        format!("postgresql://{role}:{password}@{address}"),
    ))
}

/// Every statement of an unlock, a replay and an assign runs as the runtime group, never as the
/// migration owner: the 0020 grants (tables and the CHECK key functions) must be sufficient.
#[test]
fn charm_commands_commit_under_the_runtime_role_grants() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "runtime", true).await?;
        let (role, runtime_url) = runtime_login(&harness, "grant").await?;
        let result = async {
            let runtime = DurabilityRoot::connect_test_runtime(&runtime_url)?;
            assert!(runtime.maintain_ready_once().await?);
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = runtime
                .open_character_authority(&seal)
                .await
                .map_err(|error| format!("runtime authority: {error:?}"))?;
            let facts = Facts::new(&[1000], &[("rat", 3)]);
            let request = unlock(60, "wound");
            let first = runtime
                .commit_charm_command(
                    &authority,
                    &harness.node,
                    fence(1)?,
                    request.clone(),
                    facts.clone(),
                )
                .await
                .map_err(|error| format!("runtime unlock: {error:?}"))?;
            let CharmCommandOutcome::Committed(first) = first else {
                return Err(format!("runtime unlock was not new: {first:?}").into());
            };
            assert_eq!(
                runtime
                    .commit_charm_command(
                        &authority,
                        &harness.node,
                        fence(1)?,
                        request,
                        facts.clone()
                    )
                    .await
                    .map_err(|error| format!("runtime replay: {error:?}"))?,
                CharmCommandOutcome::AlreadyCommitted(first)
            );
            let assigned = runtime
                .commit_charm_command(
                    &authority,
                    &harness.node,
                    fence(2)?,
                    assign(61, "wound", "rat"),
                    facts,
                )
                .await
                .map_err(|error| format!("runtime assign: {error:?}"))?;
            assert!(matches!(assigned, CharmCommandOutcome::Committed(_)));
            let state = runtime
                .read_character_charm_state(
                    &authority,
                    CharacterId::from_bytes(id(41)).map_err(debug)?,
                )
                .await
                .map_err(|error| format!("runtime read: {error:?}"))?;
            assert_eq!(
                state.assignments,
                BTreeMap::from([(charm("wound"), race("rat"))])
            );
            assert_eq!(revision(&harness.pool).await?, "3");
            drop(authority);
            drop(seal);
            drop(runtime);
            Ok::<(), Box<dyn std::error::Error>>(())
        }
        .await;
        let admin_url = harness.database.admin_url.clone();
        harness.cleanup().await?;
        let mut connection = sqlx::PgConnection::connect(&admin_url).await?;
        sqlx::query(sqlx::AssertSqlSafe(format!("DROP ROLE IF EXISTS {role}")))
            .execute(&mut connection)
            .await?;
        connection.close().await?;
        result
    })
}

#[test]
fn every_charm_rule_fails_closed_without_a_write() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "rules", true).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let rule = |expected: CharmRuleError| move |error: &CharmStateError| matches!(error, CharmStateError::Rule(found) if *found == expected);
        let facts = Facts::new(&[240], &[("rat", 3), ("wolf", 2), ("bear", 2)]);
        let cases: [(CharmCommandRequest, CharmRuleError, &str); 4] = [
            (
                unlock(60, "unknown"),
                CharmRuleError::UnknownCharm,
                "unknown charm",
            ),
            (
                unlock(61, "zap"),
                CharmRuleError::InsufficientBalance,
                "points short",
            ),
            (
                unlock(62, "gut"),
                CharmRuleError::InsufficientBalance,
                "no echoes",
            ),
            (
                assign(63, "wound", "rat"),
                CharmRuleError::CharmLocked,
                "locked charm",
            ),
        ];
        for (request, error, case) in cases {
            let before = revision(&harness.pool).await?;
            let result = harness
                .root
                .commit_charm_command(&authority, &harness.node, fence(1)?, request, facts.clone())
                .await;
            assert!(
                matches!(&result, Err(CharmStateError::Rule(found)) if *found == error),
                "{case}: {result:?}"
            );
            assert_eq!(revision(&harness.pool).await?, before, "{case}");
        }
        let mut foreign = unlock(64, "wound");
        foreign.catalogue_revision = "content-2".into();
        rejected(
            &harness,
            &authority,
            fence(1)?,
            foreign,
            &facts,
            |error| matches!(error, CharmStateError::CharmContextMismatch),
            "catalogue of another content revision",
        )
        .await?;

        committed(&harness, &authority, 1, unlock(65, "wound"), &facts).await?;
        rejected(
            &harness,
            &authority,
            fence(2)?,
            assign(66, "wound", "wolf"),
            &facts,
            rule(CharmRuleError::BestiaryStageTooLow),
            "major on an incomplete entry",
        )
        .await?;
        committed(&harness, &authority, 2, assign(67, "wound", "rat"), &facts).await?;
        rejected(
            &harness,
            &authority,
            fence(3)?,
            assign(68, "wound", "bear"),
            &facts,
            rule(CharmRuleError::CharmAlreadyAssigned),
            "re-assign without unassign",
        )
        .await?;

        let richer = Facts::new(&[480], &[("rat", 3), ("wolf", 3), ("bear", 2)]);
        committed(&harness, &authority, 3, unlock(69, "poison"), &richer).await?;
        rejected(
            &harness,
            &authority,
            fence(4)?,
            assign(70, "poison", "rat"),
            &richer,
            rule(CharmRuleError::RaceCapacityReached),
            "second major on one race",
        )
        .await?;
        committed(
            &harness,
            &authority,
            4,
            assign(71, "poison", "wolf"),
            &richer,
        )
        .await?;
        // Two major stages earned 100 echoes: the first minor stage.
        committed(&harness, &authority, 5, unlock(72, "gut"), &richer).await?;
        rejected(
            &harness,
            &authority,
            fence(6)?,
            assign(73, "gut", "bear"),
            &richer,
            rule(CharmRuleError::AssignmentSlotsFull),
            "third assignment on free slots",
        )
        .await?;
        let premium = Facts {
            slots: CharmSlotEntitlement::Premium,
            ..richer
        };
        committed(&harness, &authority, 6, assign(74, "gut", "bear"), &premium).await?;
        assert_eq!(revision(&harness.pool).await?, "7");
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn stale_fences_and_missing_state_fail_closed() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin.clone(), "stale", true).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let facts = Facts::new(&[10_000], &[("rat", 3)]);
        rejected(
            &harness,
            &authority,
            fence(2)?,
            unlock(60, "wound"),
            &facts,
            |error| matches!(error, CharmStateError::CharacterRevisionMismatch),
            "stale revision",
        )
        .await?;
        let authority_rejected =
            |error: &CharmStateError| matches!(error, CharmStateError::AuthorityRejected);
        let mut stale_connection = fence(1)?;
        stale_connection.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        let mut stale_lease = fence(1)?;
        stale_lease.character_lease_generation = 2;
        let mut stale_scope = fence(1)?;
        stale_scope.scope_ownership_generation = ScopeOwnershipGeneration::new(2).map_err(debug)?;
        let mut other_session = fence(1)?;
        other_session.game_session_id =
            crate::foundation::GameSessionId::decode(&id(51)).map_err(debug)?;
        for (tag, stale, case) in [
            (61, stale_connection, "stale connection generation"),
            (62, stale_lease, "stale lease generation"),
            (63, stale_scope, "stale scope ownership generation"),
            (64, other_session, "another game session"),
        ] {
            rejected(
                &harness,
                &authority,
                stale,
                unlock(tag, "wound"),
                &facts,
                authority_rejected,
                case,
            )
            .await?;
        }
        sqlx::query(
            "UPDATE game_durability_reconnect_sessions SET session_state = 3 \
             WHERE game_session_id = encode($1,'hex')::uuid",
        )
        .bind(id(50).as_slice())
        .execute(&harness.pool)
        .await?;
        rejected(
            &harness,
            &authority,
            fence(1)?,
            unlock(65, "wound"),
            &facts,
            authority_rejected,
            "ended session",
        )
        .await?;
        drop(authority);
        drop(seal);
        harness.cleanup().await?;

        let ended = Harness::create(admin.clone(), "ended", true).await?;
        let ended_seal = ended.recovery.seal_current().map_err(debug)?;
        let ended_authority = ended
            .root
            .open_character_authority(&ended_seal)
            .await
            .map_err(debug)?;
        ended
            .root
            .revoke_node_registration(ended.node.fact())
            .await
            .map_err(debug)?;
        rejected(
            &ended,
            &ended_authority,
            fence(1)?,
            unlock(66, "wound"),
            &facts,
            authority_rejected,
            "revoked node incarnation",
        )
        .await?;
        drop(ended_authority);
        drop(ended_seal);
        ended.cleanup().await?;

        let missing = Harness::create(admin, "missing", false).await?;
        let missing_seal = missing.recovery.seal_current().map_err(debug)?;
        let missing_authority = missing
            .root
            .open_character_authority(&missing_seal)
            .await
            .map_err(debug)?;
        rejected(
            &missing,
            &missing_authority,
            fence(1)?,
            unlock(67, "wound"),
            &facts,
            |error| matches!(error, CharmStateError::MissingProgressionState),
            "missing progression state",
        )
        .await?;
        drop(missing_authority);
        drop(missing_seal);
        missing.cleanup().await
    })
}

/// Two distinct unlocks that each fit the balance alone race on two connections: one commits,
/// the other sees the advanced revision, and at the new revision the spent balance refuses it.
#[test]
fn concurrent_unlocks_cannot_double_spend_and_a_failed_commit_writes_nothing() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "race", true).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let other_root = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
        assert!(other_root.maintain_ready_once().await?);
        let other_seal = harness.recovery.seal_current().map_err(debug)?;
        let other_authority = other_root
            .open_character_authority(&other_seal)
            .await
            .map_err(debug)?;
        // 240 points: exactly one first major stage.
        let facts = Facts::new(&[240], &[]);

        // A failed receipt insert rolls the whole command back.
        sqlx::raw_sql(
            "CREATE FUNCTION test_reject_charm_receipt() RETURNS trigger LANGUAGE plpgsql AS $$ \
             BEGIN RAISE EXCEPTION 'injected receipt failure'; END; $$; \
             CREATE TRIGGER test_reject_charm_receipt BEFORE INSERT \
               ON game_character_charm_receipts \
             FOR EACH ROW EXECUTE FUNCTION test_reject_charm_receipt();",
        )
        .execute(&harness.pool)
        .await?;
        assert!(
            harness
                .root
                .commit_charm_command(
                    &authority,
                    &harness.node,
                    fence(1)?,
                    unlock(60, "wound"),
                    facts.clone(),
                )
                .await
                .is_err()
        );
        sqlx::raw_sql(
            "DROP TRIGGER test_reject_charm_receipt ON game_character_charm_receipts; \
             DROP FUNCTION test_reject_charm_receipt();",
        )
        .execute(&harness.pool)
        .await?;
        let untouched: (String, String, i64, i64) = sqlx::query_as(
            "SELECT (SELECT character_revision::text FROM game_character_roots), \
                    (SELECT character_revision::text FROM game_character_progression_state), \
                    (SELECT count(*) FROM game_character_charm_receipts), \
                    (SELECT count(*) FROM game_character_charm_unlocks)",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(untouched, ("1".to_owned(), "1".to_owned(), 0, 0));

        let wound = unlock(61, "wound");
        let poison = unlock(62, "poison");
        let first = harness.root.commit_charm_command(
            &authority,
            &harness.node,
            fence(1)?,
            wound.clone(),
            facts.clone(),
        );
        let second = other_root.commit_charm_command(
            &other_authority,
            &harness.node,
            fence(1)?,
            poison.clone(),
            facts.clone(),
        );
        let (first, second) = join_two(first, second).await;
        let loser = match (first, second) {
            (
                Ok(CharmCommandOutcome::Committed(_)),
                Err(CharmStateError::CharacterRevisionMismatch),
            ) => poison,
            (
                Err(CharmStateError::CharacterRevisionMismatch),
                Ok(CharmCommandOutcome::Committed(_)),
            ) => wound,
            outcomes => return Err(format!("unexpected concurrent outcomes: {outcomes:?}").into()),
        };
        rejected(
            &harness,
            &authority,
            fence(2)?,
            loser,
            &facts,
            |error| {
                matches!(
                    error,
                    CharmStateError::Rule(CharmRuleError::InsufficientBalance)
                )
            },
            "second spend of the same points",
        )
        .await?;
        let receipts: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_charm_receipts")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(receipts, 1);
        assert_eq!(revision(&harness.pool).await?, "2");

        // The same occurrence raced on both connections commits once; the other replays it.
        let same = unlock(63, "wound");
        let rich = Facts::new(&[10_000], &[]);
        let first = harness.root.commit_charm_command(
            &authority,
            &harness.node,
            fence(2)?,
            same.clone(),
            rich.clone(),
        );
        let second =
            other_root.commit_charm_command(&other_authority, &harness.node, fence(2)?, same, rich);
        match join_two(first, second).await {
            (
                Ok(CharmCommandOutcome::Committed(committed)),
                Ok(CharmCommandOutcome::AlreadyCommitted(replayed)),
            )
            | (
                Ok(CharmCommandOutcome::AlreadyCommitted(replayed)),
                Ok(CharmCommandOutcome::Committed(committed)),
            ) => assert_eq!(committed, replayed),
            outcomes => return Err(format!("unexpected replay race: {outcomes:?}").into()),
        }
        assert_eq!(revision(&harness.pool).await?, "3");
        drop(other_authority);
        drop(other_seal);
        drop(other_root);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

// SQL-level guards: the exact statements a writer issues, committed as one transaction, with one
// invariant broken per case. Character 41 starts at revision one (level 50, 1000 experience).

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn uuid(seed: u8) -> String {
    format!("'{}'::uuid", hex(&id(seed)))
}

const CHARACTER: u8 = 41;

fn advance(original: u64) -> String {
    format!(
        "UPDATE game_character_roots SET character_revision = {committed} \
          WHERE character_id = {character} AND character_revision = {original}; \
         UPDATE game_character_progression_state SET character_revision = {committed} \
          WHERE character_id = {character} AND character_revision = {original};",
        committed = original + 1,
        character = uuid(CHARACTER),
    )
}

/// A charm receipt at `original`; `step` is `Some((before, after, cost))` for an unlock and
/// `race` is set for an assign.
fn receipt(
    occurrence: u8,
    original: u64,
    name: &str,
    category: i16,
    step: Option<(i16, i16, i64)>,
    race: Option<&str>,
) -> String {
    let (kind, before, after, cost) = match step {
        Some((before, after, cost)) => (1, before.to_string(), after.to_string(), cost.to_string()),
        None => (2, "NULL".into(), "NULL".into(), "NULL".into()),
    };
    let race = race.map_or_else(
        || "NULL".to_owned(),
        |race| format!("'oteryn:creature.{race}'"),
    );
    format!(
        "INSERT INTO game_character_charm_receipts(\
           charm_occurrence_id, command_binding, catalogue_digest, catalogue_revision, \
           character_id, original_character_revision, committed_character_revision, \
           level_before, level_after, experience_before, experience_after, command_kind, \
           charm_key, charm_category, stage_before, stage_after, stage_cost, race_key, \
           profile_revision, ruleset_revision, content_revision, simulation_revision, \
           evidence_revision, declaration_revision, policy_revision, reward_revision, \
           committed_at) \
         VALUES ({occurrence}, '\\x{binding}'::bytea, '\\x{digest}'::bytea, 'content-1', \
           {character}, {original}, {committed}, 50, 50, 1000, 1000, {kind}, \
           'oteryn:charm.{name}', {category}, {before}, {after}, {cost}, {race}, \
           'profile-1', 'ruleset-1', 'content-1', 'simulation-1', 'evidence-1', \
           'declaration-1', 'policy-1', 'reward-1', 1);",
        occurrence = uuid(occurrence),
        binding = hex(&[occurrence; 33]),
        digest = hex(&[occurrence; 32]),
        character = uuid(CHARACTER),
        committed = original + 1,
    )
}

fn unlock_row(name: &str, stage: i16, revision: u64, occurrence: u8) -> String {
    format!(
        "INSERT INTO game_character_charm_unlocks VALUES \
           ({character}, 'oteryn:charm.{name}', {stage}, {revision}, {occurrence}) \
         ON CONFLICT (character_id, charm_key) DO UPDATE \
           SET unlocked_stage = EXCLUDED.unlocked_stage, \
               committed_character_revision = EXCLUDED.committed_character_revision, \
               last_charm_occurrence_id = EXCLUDED.last_charm_occurrence_id;",
        character = uuid(CHARACTER),
        occurrence = uuid(occurrence),
    )
}

fn assignment_row(name: &str, race: &str, category: i16, revision: u64, occurrence: u8) -> String {
    format!(
        "INSERT INTO game_character_charm_assignments VALUES \
           ({character}, 'oteryn:charm.{name}', 'oteryn:creature.{race}', {category}, \
            {revision}, {occurrence});",
        character = uuid(CHARACTER),
        occurrence = uuid(occurrence),
    )
}

/// A complete unlock transaction of a major charm (category 1) from `before` to `before + 1`.
fn sql_unlock(occurrence: u8, original: u64, name: &str, before: i16) -> String {
    format!(
        "{}{}{}",
        advance(original),
        receipt(
            occurrence,
            original,
            name,
            1,
            Some((before, before + 1, 240)),
            None
        ),
        unlock_row(name, before + 1, original + 1, occurrence)
    )
}

/// A complete assign transaction of a major charm (category 1).
fn sql_assign(occurrence: u8, original: u64, name: &str, race: &str) -> String {
    format!(
        "{}{}{}",
        advance(original),
        receipt(occurrence, original, name, 1, None, Some(race)),
        assignment_row(name, race, 1, original + 1, occurrence)
    )
}

async fn attempt(pool: &sqlx::PgPool, script: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(script.to_owned()))
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

async fn snapshot(pool: &sqlx::PgPool) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT concat_ws('|', \
           (SELECT string_agg(character_revision::text, ',') FROM game_character_roots), \
           (SELECT string_agg(character_revision::text, ',') \
              FROM game_character_progression_state), \
           (SELECT string_agg(committed_character_revision::text, ',' \
                     ORDER BY committed_character_revision) \
              FROM game_character_charm_receipts), \
           (SELECT string_agg(concat_ws(':', charm_key, unlocked_stage, \
                     committed_character_revision), ',' ORDER BY charm_key) \
              FROM game_character_charm_unlocks), \
           (SELECT string_agg(concat_ws(':', charm_key, race_key, charm_category), ',' \
                     ORDER BY charm_key) \
              FROM game_character_charm_assignments))",
    )
    .fetch_one(pool)
    .await?)
}

async fn expect_committed(pool: &sqlx::PgPool, case: &str, script: &str) -> TestResult {
    attempt(pool, script)
        .await
        .map_err(|error| format!("{case}: expected commit, got {error}").into())
}

async fn expect_rejected(pool: &sqlx::PgPool, case: &str, script: &str, code: &str) -> TestResult {
    let before = snapshot(pool).await?;
    let result = attempt(pool, script).await;
    let observed = result
        .as_ref()
        .err()
        .and_then(|error| error.as_database_error())
        .and_then(|error| error.code())
        .map(|code| code.into_owned());
    if observed.as_deref() != Some(code) {
        return Err(format!("{case}: expected SQLSTATE {code}, got {result:?}").into());
    }
    if snapshot(pool).await? != before {
        return Err(format!("{case}: a rejected transaction changed durable state").into());
    }
    Ok(())
}

#[test]
fn the_database_binds_charm_rows_to_one_receipt_chain() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "guards", true).await?;
        let pool = &harness.pool;
        let character = uuid(CHARACTER);

        // Revision one: nothing may exist without its receipt.
        expect_rejected(
            pool,
            "receipt without the root successor",
            &format!(
                "{}{}",
                receipt(60, 1, "wound", 1, Some((0, 1, 240)), None),
                unlock_row("wound", 1, 2, 60)
            ),
            "23514",
        )
        .await?;
        expect_rejected(
            pool,
            "receipt without its unlock row",
            &format!(
                "{}{}",
                advance(1),
                receipt(60, 1, "wound", 1, Some((0, 1, 240)), None)
            ),
            "23514",
        )
        .await?;
        expect_rejected(
            pool,
            "a stage skipped",
            &format!(
                "{}{}{}",
                advance(1),
                receipt(60, 1, "wound", 1, Some((0, 2, 240)), None),
                unlock_row("wound", 2, 2, 60)
            ),
            "23514",
        )
        .await?;
        expect_rejected(
            pool,
            "a later stage without the earlier one",
            &format!(
                "{}{}{}",
                advance(1),
                receipt(60, 1, "wound", 1, Some((1, 2, 360)), None),
                unlock_row("wound", 2, 2, 60)
            ),
            "23514",
        )
        .await?;
        expect_rejected(
            pool,
            "row stage other than the receipt's",
            &format!(
                "{}{}{}",
                advance(1),
                receipt(60, 1, "wound", 1, Some((0, 1, 240)), None),
                unlock_row("wound", 2, 2, 60)
            ),
            "23514",
        )
        .await?;
        expect_rejected(
            pool,
            "experience changed by a charm receipt",
            &sql_unlock(60, 1, "wound", 0).replace("50, 50, 1000, 1000", "50, 50, 1000, 1001"),
            "23514",
        )
        .await?;
        expect_rejected(
            pool,
            "assign before the unlock revision",
            &format!(
                "{}{}{}{}{}{}",
                advance(1),
                receipt(60, 1, "wound", 1, None, Some("rat")),
                advance(2),
                receipt(61, 2, "wound", 1, Some((0, 1, 240)), None),
                unlock_row("wound", 1, 3, 61),
                assignment_row("wound", "rat", 1, 2, 60)
            ),
            "23514",
        )
        .await?;
        expect_rejected(
            pool,
            "assignment of a locked charm",
            &sql_assign(60, 1, "wound", "rat"),
            "23503",
        )
        .await?;

        expect_committed(pool, "unlock wound r2", &sql_unlock(60, 1, "wound", 0)).await?;
        expect_committed(pool, "unlock wound r3", &sql_unlock(61, 2, "wound", 1)).await?;
        expect_committed(pool, "unlock poison r4", &sql_unlock(62, 3, "poison", 0)).await?;

        expect_rejected(
            pool,
            "row-only stage advance",
            &format!(
                "UPDATE game_character_charm_unlocks SET unlocked_stage = 3 \
                  WHERE character_id = {character} AND charm_key = 'oteryn:charm.wound';"
            ),
            "23514",
        )
        .await?;
        expect_rejected(
            pool,
            "stage lowered",
            &format!(
                "UPDATE game_character_charm_unlocks SET unlocked_stage = 1 \
                  WHERE character_id = {character} AND charm_key = 'oteryn:charm.wound';"
            ),
            "23514",
        )
        .await?;
        expect_rejected(
            pool,
            "unlock deleted",
            &format!("DELETE FROM game_character_charm_unlocks WHERE character_id = {character};"),
            "23514",
        )
        .await?;
        expect_rejected(
            pool,
            "same stage bought twice",
            &format!(
                "{}{}{}",
                advance(4),
                receipt(63, 4, "poison", 1, Some((0, 1, 240)), None),
                unlock_row("poison", 1, 5, 63)
            ),
            "23505",
        )
        .await?;
        expect_rejected(
            pool,
            "assign receipt without its row",
            &format!(
                "{}{}",
                advance(4),
                receipt(63, 4, "wound", 1, None, Some("rat"))
            ),
            "23514",
        )
        .await?;
        expect_rejected(
            pool,
            "assignment row naming another race than its receipt",
            &format!(
                "{}{}{}",
                advance(4),
                receipt(63, 4, "wound", 1, None, Some("rat")),
                assignment_row("wound", "wolf", 1, 5, 63)
            ),
            "23514",
        )
        .await?;

        expect_committed(pool, "assign wound r5", &sql_assign(63, 4, "wound", "rat")).await?;
        expect_rejected(
            pool,
            "second major on one race",
            &sql_assign(64, 5, "poison", "rat"),
            "23505",
        )
        .await?;
        expect_rejected(
            pool,
            "charm assigned twice",
            &sql_assign(64, 5, "wound", "wolf")
                .replace(&assignment_row("wound", "wolf", 1, 6, 64), ""),
            "23505",
        )
        .await?;
        expect_rejected(
            pool,
            "unassign",
            &format!(
                "DELETE FROM game_character_charm_assignments WHERE character_id = {character};"
            ),
            "23514",
        )
        .await?;
        expect_rejected(
            pool,
            "reassign in place",
            &format!(
                "UPDATE game_character_charm_assignments SET race_key = 'oteryn:creature.wolf' \
                  WHERE character_id = {character};"
            ),
            "23514",
        )
        .await?;
        expect_rejected(
            pool,
            "receipt rewritten",
            &format!("UPDATE game_character_charm_receipts SET stage_cost = 1 WHERE character_id = {character};"),
            "23514",
        )
        .await?;
        for table in [
            "game_character_charm_receipts",
            "game_character_charm_unlocks",
            "game_character_charm_assignments",
        ] {
            expect_rejected(pool, table, &format!("TRUNCATE {table} CASCADE;"), "23514").await?;
        }
        expect_rejected(
            pool,
            "major charm recorded as minor to pass the race bound",
            &format!(
                "{}{}{}",
                advance(5),
                receipt(64, 5, "poison", 2, None, Some("rat")),
                assignment_row("poison", "rat", 2, 6, 64)
            ),
            "23514",
        )
        .await?;
        expect_committed(
            pool,
            "assign poison r6",
            &sql_assign(64, 5, "poison", "wolf"),
        )
        .await?;
        assert_eq!(
            snapshot(pool).await?,
            "6|6|2,3,4,5,6|oteryn:charm.poison:1:4,oteryn:charm.wound:2:3|\
             oteryn:charm.poison:oteryn:creature.wolf:1,oteryn:charm.wound:oteryn:creature.rat:1"
        );

        let privileges: Vec<(String, bool, bool, bool, bool)> = sqlx::query_as(
            "SELECT t, has_table_privilege('oteryn_game_runtime', t, 'INSERT'), \
                    has_table_privilege('oteryn_game_runtime', t, 'UPDATE'), \
                    has_table_privilege('oteryn_game_runtime', t, 'DELETE'), \
                    has_table_privilege('oteryn_game_control', t, 'INSERT') \
               FROM unnest(ARRAY['game_character_charm_receipts', \
                                 'game_character_charm_unlocks', \
                                 'game_character_charm_assignments']) AS t ORDER BY t",
        )
        .fetch_all(pool)
        .await?;
        assert_eq!(
            privileges,
            vec![
                (
                    "game_character_charm_assignments".to_owned(),
                    true,
                    false,
                    false,
                    false
                ),
                (
                    "game_character_charm_receipts".to_owned(),
                    true,
                    false,
                    false,
                    false
                ),
                (
                    "game_character_charm_unlocks".to_owned(),
                    true,
                    true,
                    false,
                    false
                ),
            ]
        );
        let public_functions: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_proc p, \
               aclexplode(coalesce(p.proacl, acldefault('f', p.proowner))) acl \
              WHERE p.proname IN ('game_character_is_charm_key', \
                                  'game_character_is_bestiary_race_key', \
                                  'game_character_charm_consistency_guard', \
                                  'game_character_charm_unlock_row_guard') \
                AND acl.grantee = 0 AND acl.privilege_type = 'EXECUTE'",
        )
        .fetch_one(pool)
        .await?;
        assert_eq!(public_functions, 0);
        // The key functions back CHECKs, which run as the writing role.
        let runtime_functions: Vec<(String, bool)> = sqlx::query_as(
            "SELECT f, has_function_privilege('oteryn_game_runtime', f, 'EXECUTE') \
               FROM unnest(ARRAY['game_character_is_charm_key(text)', \
                                 'game_character_is_bestiary_race_key(text)']) AS f ORDER BY f",
        )
        .fetch_all(pool)
        .await?;
        assert_eq!(
            runtime_functions,
            vec![
                ("game_character_is_bestiary_race_key(text)".to_owned(), true),
                ("game_character_is_charm_key(text)".to_owned(), true),
            ]
        );
        harness.cleanup().await
    })
}
