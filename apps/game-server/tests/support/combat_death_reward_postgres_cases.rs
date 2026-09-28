// Combat D2b: creature death -> loot MINT(s) + R7 P03 XP, composed by
// `combat::settle_creature_death_rewards`. Both wrappers provide the same
// path-loaded crate root.

use crate::character_recovery_fence::CharacterRecoveryStore;
use crate::combat::{
    CombatDeathRewardLootError, CombatDeathRewardXpError, CreatureDeathRewardInput,
    DeathGroundContext, DurabilitySession, LootDefinitionRef, LootSelectionAlgorithm,
    LootTableDefinition, LootTableEntry, RewardPrincipal, RewardProgressionBinding,
    settle_creature_death_rewards,
};
use crate::domain::CharacterId;
use crate::domain::progression::{
    FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
};
use crate::durability::DurabilityRoot;
use crate::durability::admission_authority_guards::GuardPublicationDisposition;
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, ExperienceCommitOutcome,
};
use crate::durability::item_mint::ItemMintError;
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, BootstrapSecret, ControlActor,
    LaunchBinding, NodeIncarnationProof, OperationKey, RuntimeScopeAssignmentWriter,
};
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
    AdmissionAuthorityOwningPublisherV1, AdmissionAuthorityPublicationChangeV1,
    AdmissionAuthorityPublicationErrorV1, AdmissionAuthorityPublicationV1,
    AdmissionPublicationPreconditionV1, AdmissionPublicationPurposeV1,
    AdmissionPublicationSourceV1,
};
use crate::foundation::{
    ChannelId, CombatDeathFixture, RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId,
};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};
use sqlx::{Connection, Executor};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const WORLD: u8 = 42;
const CHANNEL: u8 = 43;
const RAT_XP: i64 = 5;

fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

fn debug<E: std::fmt::Debug>(error: E) -> String {
    format!("{error:?}")
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

fn runtime() -> TestResult<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?)
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
        let name = format!("cdr_{name}_{suffix}");
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
        assert!(
            version.starts_with("17"),
            "PostgreSQL 17 required: {version}"
        );
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
        "oteryn-combat-death-reward-parent-{}",
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
    let launch = LaunchBinding::new(&format!("combat-death-reward-launch-{tag}")).map_err(debug)?;
    let node = crate::foundation::NodeId::decode(&id(tag)).map_err(debug)?;
    root.issue_node_bootstrap_authorization(&secret, &launch, None)
        .await
        .map_err(debug)?;
    root.register_node_incarnation(&secret, &launch, node)
        .await
        .map_err(|error| debug(error).into())
}

fn scope() -> TestResult<RuntimeScopeRefV1> {
    Ok(RuntimeScopeRefV1::channel(
        WorldId::decode(&id(WORLD)).map_err(debug)?,
        ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
    ))
}

fn bootstrap_binding() -> Vec<u8> {
    let mut binding = vec![1];
    binding.extend_from_slice(&id(31));
    binding.extend_from_slice(&1_i64.to_be_bytes());
    binding.extend_from_slice(&id(30));
    binding.extend_from_slice(&id(40));
    binding.extend_from_slice(&id(WORLD));
    binding.extend_from_slice(&1_i64.to_be_bytes());
    binding.extend_from_slice(&120_i64.to_be_bytes());
    for value in ["profile-1", "ruleset-1", "content-1", "starter-1"] {
        binding.extend_from_slice(&u16::try_from(value.len()).expect("length").to_be_bytes());
        binding.extend_from_slice(value.as_bytes());
    }
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
    ownership_generation: u64,
) -> TestResult {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
            .fetch_one(pool)
            .await?;
    let change = AdmissionAuthorityPublicationChangeV1 {
        key: AdmissionAuthorityGuardKeyV1::Runtime(scope()?),
        source: AdmissionPublicationSourceV1 {
            authority: "game-runtime-publisher".into(),
            purpose: AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness,
            source_revision: 1,
            decision_identity: "combat-death-reward-runtime-ready-1".into(),
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

async fn seed_character(pool: &sqlx::PgPool) -> TestResult {
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
    .bind(id(WORLD).as_slice())
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
    .bind(id(WORLD).as_slice())
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
    .bind(id(WORLD).as_slice())
    .bind(id(CHANNEL).as_slice())
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
    .bind(id(WORLD).as_slice())
    .bind(id(50).as_slice())
    .execute(pool)
    .await?;
    Ok(())
}

struct Harness {
    database: Database,
    root: DurabilityRoot,
    pool: sqlx::PgPool,
    recovery: CharacterRecoveryStore,
    retained: std::path::PathBuf,
    node: NodeIncarnationProof,
    writer: RuntimeScopeAssignmentWriter,
}

impl Harness {
    async fn create(admin: String, tag: &str) -> TestResult<Self> {
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
        seed_character(&pool).await?;
        sqlx::query(
            "INSERT INTO game_control_scope_grants \
             (control_role, world_id, channel_id, operation) \
             SELECT session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, operation \
               FROM unnest(ARRAY[1, 2, 3]::SMALLINT[]) AS operation",
        )
        .bind(id(WORLD).as_slice())
        .bind(id(CHANNEL).as_slice())
        .execute(&pool)
        .await?;
        let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "combat-death-reward-writer")
            .await
            .map_err(debug)?;
        let outcome = writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([7_u8; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                command: AssignmentCommand::Assign {
                    scope: scope()?,
                    target: node.fact(),
                },
            })
            .await
            .map_err(debug)?;
        let AssignmentOutcome::Committed(assignment) = outcome else {
            return Err(format!("unexpected assignment outcome: {outcome:?}").into());
        };
        assert_eq!(assignment.assignment.ownership_generation, 1);
        publish_readiness(&pool, &root, &node, 1).await?;
        Ok(Self {
            database,
            root,
            pool,
            recovery,
            retained,
            node,
            writer,
        })
    }

    async fn count(&self, relation: &str) -> TestResult<i64> {
        Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {relation}"
        )))
        .fetch_one(&self.pool)
        .await?)
    }

    async fn cleanup(self) -> TestResult {
        drop(self.writer);
        self.pool.close().await;
        self.database.cleanup().await?;
        std::fs::remove_dir_all(self.retained)?;
        Ok(())
    }
}

fn death_fixture() -> TestResult<CombatDeathFixture> {
    CombatDeathFixture::new(
        WorldId::decode(&id(WORLD)).map_err(debug)?,
        ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
        ScopeOwnershipGeneration::new(1).map_err(debug)?,
    )
    .map_err(|error| debug(error).into())
}

fn ground() -> DeathGroundContext {
    DeathGroundContext {
        map_revision: "fixture:combat-death-reward.map.r1".into(),
        content_revision: "fixture:combat-death-reward.content.r1".into(),
        ruleset_revision: "fixture:combat-death-reward.ruleset.r1".into(),
        sim_revision: "fixture:combat-death-reward.sim.r1".into(),
        native_room_placement_context: b"fixture:combat-death-reward.room".to_vec(),
    }
}

fn rat_loot_table_ref() -> LootDefinitionRef {
    LootDefinitionRef::new(
        "LootTable",
        "fixture:combat-death-reward.loot.rat",
        "VSL_COMBAT_FIXTURE_PROFILE/v1",
    )
}

/// One guaranteed-drop entry: deterministic, no reliance on a specific RNG
/// draw outcome so the test asserts exactly one minted item.
fn rat_loot_table() -> LootTableDefinition {
    LootTableDefinition {
        algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
        entries: vec![LootTableEntry {
            item: LootDefinitionRef::new(
                "ItemType",
                "fixture:combat-death-reward.item.cheese",
                "VSL_COMBAT_FIXTURE_PROFILE/v1",
            ),
            min_count: 1,
            max_count: 1,
            probability_ppm: Some(1_000_000),
        }],
    }
}

fn unsupported_algorithm_loot_table() -> LootTableDefinition {
    LootTableDefinition {
        algorithm: LootSelectionAlgorithm::GuaranteedEntries,
        entries: vec![],
    }
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

fn progression_binding() -> RewardProgressionBinding<2> {
    let context = context();
    RewardProgressionBinding {
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
                    level: 1,
                    minimum_experience: ExactI64::new(0),
                },
                LevelThreshold {
                    level: 2,
                    minimum_experience: ExactI64::new(1000),
                },
            ],
            terminal_exclusive_experience: ExactI64::new(2000),
            death_loss_numerator: 1,
            death_loss_denominator: 10,
            death_loss_rounding: RoundingMode::Floor,
        },
    }
}

/// `expected_character_revision` must be the character's *current* root
/// revision: `commit_character_experience` advances it by one on every fresh
/// award, so a caller settling the same death twice must track and pass the
/// post-award revision on the second call (exactly as a real caller would
/// re-read it), not a value fixed at the first call.
fn gameplay_fence(
    scope_ownership_generation: u64,
    character_revision: u64,
) -> TestResult<CurrentCharacterGameplayFence> {
    Ok(CurrentCharacterGameplayFence {
        character_id: CharacterId::from_bytes(id(41)).map_err(debug)?,
        game_session_id: crate::foundation::GameSessionId::decode(&id(50)).map_err(debug)?,
        connection_generation: crate::foundation::ConnectionGeneration::new(1).map_err(debug)?,
        character_lease_generation: 1,
        runtime_scope: scope()?,
        scope_ownership_generation: ScopeOwnershipGeneration::new(scope_ownership_generation)
            .map_err(debug)?,
        expected_character_revision: crate::domain::CharacterRevision::new(character_revision)
            .map_err(debug)?,
    })
}

fn reward_principal(
    scope_ownership_generation: u64,
    character_revision: u64,
) -> TestResult<RewardPrincipal> {
    Ok(RewardPrincipal {
        character_id: CharacterId::from_bytes(id(41)).map_err(debug)?,
        gameplay_fence: gameplay_fence(scope_ownership_generation, character_revision)?,
    })
}

fn input(
    loot_table: LootTableDefinition,
    scope_ownership_generation: u64,
    character_revision: u64,
) -> TestResult<CreatureDeathRewardInput<2>> {
    Ok(CreatureDeathRewardInput {
        loot_table_ref: rat_loot_table_ref(),
        loot_table,
        ground: ground(),
        inflight_loot_mints_before_this_death: 0,
        reward_principals: vec![reward_principal(
            scope_ownership_generation,
            character_revision,
        )?],
        xp_amount: ExactI64::new(RAT_XP),
        progression: progression_binding(),
    })
}

#[test]
fn one_creature_death_mints_the_plan_and_awards_xp_once() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "once").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let session = DurabilitySession {
            root: &harness.root,
            authority: &authority,
            node: &harness.node,
        };

        let mut fixture = death_fixture()?;
        fixture
            .strike("fixture:reward.strike.lethal", CombatDeathFixture::HEALTH)
            .map_err(debug)?;
        let (death, corpse) = fixture.project_death().map_err(debug)?;
        let actor = fixture.actor();

        let outcome = settle_creature_death_rewards(
            death,
            corpse,
            actor,
            &mut fixture.borrow_combat_death(),
            &session,
            input(rat_loot_table(), 1, 1)?,
        )
        .await
        .map_err(debug)?;

        let minted = outcome.loot.map_err(debug)?;
        assert_eq!(minted.len(), 1);
        let ExperienceCommitOutcome::Committed(award) = outcome.xp.map_err(debug)? else {
            return Err("first XP award must be freshly committed".into());
        };
        assert_eq!(award.experience_before.get(), 0);
        assert_eq!(award.experience_after.get(), RAT_XP);

        harness.count("game_item_instances").await.map(|count| {
            assert_eq!(count, 1);
        })?;
        assert_eq!(harness.count("game_character_progression_state").await?, 1);
        assert_eq!(harness.count("game_character_xp_receipts").await?, 1);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn replay_is_idempotent_with_no_duplicate_mint_or_xp() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "replay").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let session = DurabilitySession {
            root: &harness.root,
            authority: &authority,
            node: &harness.node,
        };

        let mut fixture = death_fixture()?;
        fixture
            .strike("fixture:reward.strike.lethal", CombatDeathFixture::HEALTH)
            .map_err(debug)?;
        let actor = fixture.actor();

        // A genuine replay resends the exact same request, including the
        // fence's `expected_character_revision` (1, still current for the
        // first attempt): `commit_character_experience`'s occurrence-keyed
        // replay resolves without re-checking that fence, and
        // `settle_experience` skips the redundant `initialize_character_
        // progression` call once the occurrence is already memoized, so the
        // stale revision on the retry never needs re-validating.
        for _ in 0..2 {
            let (death, corpse) = fixture.project_death().map_err(debug)?;
            let outcome = settle_creature_death_rewards(
                death,
                corpse,
                actor,
                &mut fixture.borrow_combat_death(),
                &session,
                input(rat_loot_table(), 1, 1)?,
            )
            .await
            .map_err(debug)?;
            outcome.loot.map_err(debug)?;
            outcome.xp.map_err(debug)?;
        }

        assert_eq!(harness.count("game_item_instances").await?, 1);
        assert_eq!(harness.count("game_item_mint_receipts").await?, 1);
        assert_eq!(harness.count("game_character_xp_receipts").await?, 1);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn generation_change_leaves_a_stale_death_rejected_with_no_write() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "gen_change").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let session = DurabilitySession {
            root: &harness.root,
            authority: &authority,
            node: &harness.node,
        };

        let mut fixture = death_fixture()?;
        fixture
            .strike("fixture:reward.strike.lethal", CombatDeathFixture::HEALTH)
            .map_err(debug)?;
        let (death, corpse) = fixture.project_death().map_err(debug)?;
        let actor = fixture.actor();

        // D52: the scope moves to another node before any reward is settled.
        // The predecessor is re-read fresh (not `harness.assignment`'s, which
        // was captured before `publish_readiness` advanced the fenced
        // publication revision) to isolate this test to the generation move.
        let predecessor = harness
            .root
            .read_runtime_scope_predecessor(scope()?)
            .await
            .map_err(debug)?
            .ok_or("expected a live assignment predecessor")?;
        let node2 = register(&harness.root, 2).await?;
        let moved = harness
            .writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([8_u8; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                command: AssignmentCommand::Replace {
                    scope: scope()?,
                    predecessor,
                    target: node2.fact(),
                },
            })
            .await
            .map_err(debug)?;
        let AssignmentOutcome::Committed(moved) = moved else {
            return Err(format!("unexpected replacement outcome: {moved:?}").into());
        };
        assert_eq!(moved.assignment.ownership_generation, 2);

        let outcome = settle_creature_death_rewards(
            death,
            corpse,
            actor,
            &mut fixture.borrow_combat_death(),
            &session,
            input(rat_loot_table(), 1, 1)?,
        )
        .await
        .map_err(debug)?;

        assert!(matches!(
            outcome.loot,
            Err(CombatDeathRewardLootError::Mint(
                ItemMintError::AuthorityRejected
            ))
        ));
        assert!(matches!(
            outcome.xp,
            Err(CombatDeathRewardXpError::Progression(
                CharacterProgressionError::AuthorityRejected
            ))
        ));
        assert_eq!(harness.count("game_item_instances").await?, 0);
        assert_eq!(harness.count("game_character_progression_state").await?, 0);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn a_stale_xp_fence_rejects_xp_without_blocking_loot() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "stale_xp_fence").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let session = DurabilitySession {
            root: &harness.root,
            authority: &authority,
            node: &harness.node,
        };

        // The reward principal's session is no longer live; the runtime scope
        // assignment loot minting relies on is untouched.
        sqlx::query(
            "UPDATE game_durability_reconnect_sessions SET session_state = 3 \
             WHERE game_session_id = encode($1,'hex')::uuid",
        )
        .bind(id(50).as_slice())
        .execute(&harness.pool)
        .await?;

        let mut fixture = death_fixture()?;
        fixture
            .strike("fixture:reward.strike.lethal", CombatDeathFixture::HEALTH)
            .map_err(debug)?;
        let (death, corpse) = fixture.project_death().map_err(debug)?;
        let actor = fixture.actor();

        let outcome = settle_creature_death_rewards(
            death,
            corpse,
            actor,
            &mut fixture.borrow_combat_death(),
            &session,
            input(rat_loot_table(), 1, 1)?,
        )
        .await
        .map_err(debug)?;

        assert_eq!(outcome.loot.map_err(debug)?.len(), 1);
        assert!(matches!(
            outcome.xp,
            Err(CombatDeathRewardXpError::Progression(
                CharacterProgressionError::AuthorityRejected
            ))
        ));
        assert_eq!(harness.count("game_item_instances").await?, 1);
        assert_eq!(harness.count("game_character_progression_state").await?, 0);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn an_unsupported_loot_table_rejects_loot_without_blocking_xp() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "loot_fail").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let session = DurabilitySession {
            root: &harness.root,
            authority: &authority,
            node: &harness.node,
        };

        let mut fixture = death_fixture()?;
        fixture
            .strike("fixture:reward.strike.lethal", CombatDeathFixture::HEALTH)
            .map_err(debug)?;
        let (death, corpse) = fixture.project_death().map_err(debug)?;
        let actor = fixture.actor();

        let outcome = settle_creature_death_rewards(
            death,
            corpse,
            actor,
            &mut fixture.borrow_combat_death(),
            &session,
            input(unsupported_algorithm_loot_table(), 1, 1)?,
        )
        .await
        .map_err(debug)?;

        assert!(matches!(
            outcome.loot,
            Err(CombatDeathRewardLootError::Plan(_))
        ));
        let ExperienceCommitOutcome::Committed(award) = outcome.xp.map_err(debug)? else {
            return Err("XP award must commit despite the loot-plan failure".into());
        };
        assert_eq!(award.experience_after.get(), RAT_XP);
        assert_eq!(harness.count("game_item_instances").await?, 0);
        assert_eq!(harness.count("game_character_progression_state").await?, 1);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
