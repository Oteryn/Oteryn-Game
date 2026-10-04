// ATTACK-1b: a creature killed by auto-attack swings settles loot and XP
// through the same VSL-COMBAT-01 / DEATH-2 composition
// (`combat::settle_creature_death_rewards`). The harness mirrors
// `combat_death_reward_postgres_cases.rs`; only the damage source differs:
// every hit is one swing `(lineage command, swing_ordinal)` of a bound
// player attacker, committed exactly as the Channel owner's auto-attack
// drain commits it.

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
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::durability::item_mint::{
    CORPSE_MATERIALIZATION_PURPOSE_KEY, GroundPlacement, ItemMintCause, ItemMintError,
    ItemMintRequest, TypedDefinitionRef,
};
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
        let name = format!("akr_{name}_{suffix}");
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
        "oteryn-attack-kill-reward-parent-{}",
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
    let launch = LaunchBinding::new(&format!("attack-kill-reward-launch-{tag}")).map_err(debug)?;
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
    let mut binding = vec![2];
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
          1,1,'profile-1','ruleset-1','content-1','starter-1','Fixture Hero')",
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
        let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "attack-kill-reward-writer")
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

/// The corpse item definition is caller-supplied (Content binding of
/// `i00005801` waits on the Content revision, D3-7).
fn corpse_item_ref() -> LootDefinitionRef {
    LootDefinitionRef::new(
        "ItemType",
        "fixture:combat-death-reward.item.rat_corpse",
        "VSL_COMBAT_FIXTURE_PROFILE/v1",
    )
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
        gameplay_fence: gameplay_fence(scope_ownership_generation, character_revision)?,
    })
}

fn input(
    loot_table: LootTableDefinition,
    scope_ownership_generation: u64,
    character_revision: u64,
) -> TestResult<CreatureDeathRewardInput<2>> {
    Ok(CreatureDeathRewardInput {
        corpse_item: corpse_item_ref(),
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

fn uuid_text(bytes: [u8; 16]) -> String {
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

impl Harness {
    /// `corpse_top_damage_character_id::text` and whether `materialized_at`
    /// was set, for the corpse receipt of `corpse`.
    async fn corpse_receipt(&self, corpse: [u8; 16]) -> TestResult<(String, bool)> {
        use sqlx::Row;
        let row = sqlx::query(
            "SELECT corpse_top_damage_character_id::text AS winner, \
                    materialized_at IS NOT NULL AS materialized \
               FROM game_item_mint_receipts \
              WHERE item_instance_id = encode($1,'hex')::uuid AND loot_purpose_key = $2",
        )
        .bind(corpse.as_slice())
        .bind(CORPSE_MATERIALIZATION_PURPOSE_KEY)
        .fetch_one(&self.pool)
        .await?;
        Ok((row.try_get("winner")?, row.try_get("materialized")?))
    }
}

/// The fixture creature (health 20) dies to in-order swings of one lineage;
/// a replayed swing returns its retained receipt without a second write.
#[test]
fn an_auto_attack_kill_mints_the_loot_and_awards_the_xp() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "kill").await?;
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

        let attacker = crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?;
        let mut fixture = CombatDeathFixture::new_with_bound_attacker(
            WorldId::decode(&id(WORLD)).map_err(debug)?,
            ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
            ScopeOwnershipGeneration::new(1).map_err(debug)?,
            attacker,
        )
        .map_err(debug)?;
        let first = fixture.swing(1, 0, 7).map_err(debug)?;
        assert!(first.applied);
        assert_eq!(first.health_after, CombatDeathFixture::HEALTH - 7);
        let second = fixture.swing(1, 1, 7).map_err(debug)?;
        assert_eq!(second.health_after, CombatDeathFixture::HEALTH - 14);
        // A replayed swing returns its retained receipt without a second write.
        let replay = fixture.swing(1, 1, 7).map_err(debug)?;
        assert!(!replay.applied);
        assert_eq!(replay.health_after, second.health_after);
        let lethal = fixture.swing(1, 2, 7).map_err(debug)?;
        assert!(lethal.applied && lethal.health_after <= 0);
        assert_eq!(
            fixture.top_damage_character().map_err(debug)?,
            Some(attacker)
        );
        fixture.project_death().map_err(debug)?;
        let actor = fixture.actor();

        let mut slot = CharacterRevisionSequencer::new()
            .acquire(CharacterId::from_bytes(id(41)).map_err(debug)?)
            .await;
        let outcome = settle_creature_death_rewards(
            actor,
            &mut fixture.borrow_combat_death(),
            &session,
            &mut slot,
            input(rat_loot_table(), 1, 1)?,
        )
        .await
        .map_err(debug)?;
        let minted = outcome.loot.map_err(debug)?;
        assert_eq!(minted.entries.len(), 1);
        let ExperienceCommitOutcome::Committed(award) = outcome.xp.map_err(debug)? else {
            return Err("the kill's XP award must be freshly committed".into());
        };
        assert_eq!(award.experience_before.get(), 0);
        assert_eq!(award.experience_after.get(), RAT_XP);
        let (winner, materialized) = harness
            .corpse_receipt(minted.corpse.item_instance_id)
            .await?;
        assert_eq!(winner, uuid_text(id(41)));
        assert!(materialized);
        assert_eq!(harness.count("game_character_xp_receipts").await?, 1);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
