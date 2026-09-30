// Shared CHEST-1 DUR-03 reward-claim MINT cases (a `once` RewardClaim mints
// one fresh item into a new direct entry of the equipped main backpack). Both
// wrappers provide the same path-loaded crate root. The backpack is equipped
// through the real B3-1 TRANSFER path; its definition is the same test-only
// fixture as the B3-1 cases.

use crate::character_recovery_fence::CharacterRecoveryStore;
use crate::domain::CharacterId;
use crate::domain::progression::{
    FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
};
use crate::durability::DurabilityRoot;
use crate::durability::admission_authority_guards::GuardPublicationDisposition;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, ExperienceAwardRequest, ExperienceCommitOutcome,
    ExperienceRewardOccurrence,
};
use crate::durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};
use crate::durability::item_transfer::{
    CommittedItemTransfer, ContainerEntryPosition, CurrentCharacterItemFence, ItemDefinitionFacts,
    ItemStackClass, ItemTransferDestination, ItemTransferError, ItemTransferOutcome,
    ItemTransferRefusal, ItemTransferRequest,
};
use crate::durability::reward_claim_mint::{
    CommittedRewardClaimMint, RewardClaimMintError, RewardClaimMintOutcome, RewardClaimMintRequest,
    RewardClaimRefusal,
};
use crate::durability::reward_claim_mint_audit as audit;
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
    ChannelId, CommandId, CommandRef, ConnectionGeneration, GameSessionId, RuntimeScopeRefV1,
    ScopeOwnershipGeneration, WorldId,
};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};
use sqlx::{Connection, Executor, Row};
use std::future::Future;
use std::task::Poll;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const WORLD: u8 = 42;
const CHANNEL: u8 = 43;
const CHARACTER: u8 = 41;
const SESSION: u8 = 50;
const BACKPACK: &str = "fixture:b3.backpack";
const COIN: &str = "fixture:b3.coin";
const STONE: &str = "fixture:b3.stone";
const CLAIM: &str = "fixture:chest.claim";

fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

/// Canonical hyphenated UUID text of `value`.
fn uuid_text(value: [u8; 16]) -> String {
    let hex: String = value.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

fn debug<E: std::fmt::Debug>(error: E) -> String {
    format!("{error:?}")
}

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
        let name = format!("rc_{name}_{suffix}");
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
        // The protected lane pins PostgreSQL 17.6; any 17.x server runs the
        // same schema, and the exact server is recorded as evidence.
        assert!(
            version.starts_with("17"),
            "PostgreSQL 17 required: {version}"
        );
        eprintln!("REWARD-CLAIM-MINT-PG: database {name} on server_version_num={version}");
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
        std::env::temp_dir().join(format!("oteryn-reward-claim-parent-{}", std::process::id()));
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
    let launch = LaunchBinding::new(&format!("reward-claim-launch-{tag}")).map_err(debug)?;
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

/// Character 41 (account 40, World 42) with progression state, an active
/// GameSession 50 in Channel 43, its admission guards, the runtime-scope
/// assignment to `node` and runtime readiness.
async fn seed_character(
    pool: &sqlx::PgPool,
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
) -> TestResult {
    for statement in ["INSERT INTO game_character_interpretations VALUES \
         (1,'profile-1','ruleset-1','content-1','starter-1',1)"]
    {
        sqlx::query(statement).execute(pool).await?;
    }
    sqlx::query("INSERT INTO game_character_account_guards VALUES (encode($1,'hex')::uuid)")
        .bind(id(40).as_slice())
        .execute(pool)
        .await?;
    sqlx::query(
        "INSERT INTO game_character_roots VALUES \
         (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
          1,1,'profile-1','ruleset-1','content-1','starter-1')",
    )
    .bind(id(CHARACTER).as_slice())
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
    .bind(id(CHARACTER).as_slice())
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
        "INSERT INTO game_character_progression_state VALUES \
         (encode($1,'hex')::uuid,1,50,1000,'profile-1','ruleset-1','content-1',\
          'simulation-1','evidence-1','declaration-1','policy-1','reward-1')",
    )
    .bind(id(CHARACTER).as_slice())
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
    .bind(id(SESSION).as_slice())
    .bind(id(40).as_slice())
    .bind(id(CHARACTER).as_slice())
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
    .bind(id(CHARACTER).as_slice())
    .bind(id(SESSION).as_slice())
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO game_durability_admission_character_guards VALUES \
         (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
          true,1,encode($4,'hex')::uuid,1,'test',1,'character-current',1,0,'{}')",
    )
    .bind(id(CHARACTER).as_slice())
    .bind(id(40).as_slice())
    .bind(id(WORLD).as_slice())
    .bind(id(SESSION).as_slice())
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO game_control_scope_grants \
         (control_role, world_id, channel_id, operation) \
         VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1)",
    )
    .bind(id(WORLD).as_slice())
    .bind(id(CHANNEL).as_slice())
    .execute(pool)
    .await?;
    let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "reward-claim-writer")
        .await
        .map_err(debug)?;
    let assignment = writer
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
    let AssignmentOutcome::Committed(receipt) = assignment else {
        return Err(format!("unexpected assignment outcome: {assignment:?}").into());
    };
    assert_eq!(receipt.assignment.ownership_generation, 1);
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
            decision_identity: "reward-claim-runtime-ready-1".into(),
            source_observed_at: now,
            clock_uncertainty_seconds: 0,
        },
        precondition: AdmissionPublicationPreconditionV1::Bootstrap {
            restored_publication_high_water: Some(0),
        },
        publication_revision: 1,
        state: AdmissionAuthorityGuardStateV1::Runtime {
            ownership_generation: 1,
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

struct Harness {
    database: Database,
    root: DurabilityRoot,
    pool: sqlx::PgPool,
    recovery: CharacterRecoveryStore,
    retained: std::path::PathBuf,
    node: NodeIncarnationProof,
    next_actor: std::cell::Cell<u32>,
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
        seed_character(&pool, &root, &node).await?;
        Ok(Self {
            database,
            root,
            pool,
            recovery,
            retained,
            node,
            next_actor: std::cell::Cell::new(1),
        })
    }

    /// MINT one Ground item in Channel 43 (generation 1) through the real
    /// stage C path; returns its ItemInstanceId.
    async fn mint(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        key: &str,
        quantity: u32,
    ) -> TestResult<[u8; 16]> {
        let actor = self.next_actor.get();
        self.next_actor.set(actor + 1);
        let request = ItemMintRequest {
            cause: ItemMintCause::for_test(
                WorldId::decode(&id(WORLD)).map_err(debug)?,
                ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
                ScopeOwnershipGeneration::new(1).map_err(debug)?,
                actor,
                1,
                definition("LootTable", "fixture:loot.b3"),
                "fixture:purpose.drop".into(),
                0,
            ),
            item: definition("Item", key),
            quantity,
            ground: GroundPlacement {
                spatial_position: vec![1, 2, 3],
                corpse_ref: id(3).to_vec(),
                map_revision: "map-1".into(),
                content_revision: "content-1".into(),
                native_room_placement_context: id(6).to_vec(),
            },
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
        };
        let mut candidate = self
            .root
            .freeze_item_mint(authority, &self.node, request)
            .await
            .map_err(debug)?;
        match self
            .root
            .commit_item_mint(authority, &self.node, &mut candidate)
            .await
            .map_err(debug)?
        {
            ItemMintOutcome::Committed(result) => Ok(result.item_instance_id),
            other => Err(format!("unexpected MINT outcome {other:?}").into()),
        }
    }

    async fn transfer(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterItemFence,
        request: ItemTransferRequest,
    ) -> Result<ItemTransferOutcome, ItemTransferError> {
        let mut candidate = self
            .root
            .freeze_item_transfer(authority, &self.node, fence, request)
            .await?;
        self.root
            .commit_item_transfer(authority, &self.node, fence, &mut candidate)
            .await
    }

    async fn committed(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        request: ItemTransferRequest,
    ) -> TestResult<CommittedItemTransfer> {
        match self
            .transfer(authority, fence()?, request)
            .await
            .map_err(debug)?
        {
            ItemTransferOutcome::Committed(result) => Ok(result),
            other => Err(format!("expected a fresh commit, got {other:?}").into()),
        }
    }

    async fn claim(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterItemFence,
        request: RewardClaimMintRequest,
    ) -> Result<RewardClaimMintOutcome, RewardClaimMintError> {
        let mut candidate = self
            .root
            .freeze_reward_claim_mint(authority, &self.node, fence, request)
            .await?;
        self.root
            .commit_reward_claim_mint(authority, &self.node, fence, &mut candidate)
            .await
    }

    async fn claimed(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        request: RewardClaimMintRequest,
    ) -> TestResult<CommittedRewardClaimMint> {
        match self
            .claim(authority, fence()?, request)
            .await
            .map_err(debug)?
        {
            RewardClaimMintOutcome::Committed(result) => Ok(result),
            other => Err(format!("expected a fresh claim, got {other:?}").into()),
        }
    }

    /// Mint and equip the fixture backpack through the real B3-1 path.
    async fn equip_backpack(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        command_id: u64,
    ) -> TestResult<[u8; 16]> {
        let backpack = self.mint(authority, BACKPACK, 1).await?;
        self.committed(
            authority,
            to_slot(command(command_id)?, backpack, backpack_facts()),
        )
        .await?;
        Ok(backpack)
    }

    /// Pick up `count` Ground stones into the backpack (B3-1 new entries).
    async fn fill_with_stones(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        first_command: u64,
        count: u64,
    ) -> TestResult {
        for offset in 0..count {
            let stone = self.mint(authority, STONE, 1).await?;
            self.committed(
                authority,
                to_backpack(
                    command(first_command + offset)?,
                    stone,
                    facts(STONE, ItemStackClass::NonStackable),
                ),
            )
            .await?;
        }
        Ok(())
    }

    async fn count(&self, relation: &str) -> TestResult<i64> {
        Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {relation}"
        )))
        .fetch_one(&self.pool)
        .await?)
    }

    /// Row counts of every relation a reward-claim MINT could write.
    async fn footprint(&self) -> TestResult<Vec<i64>> {
        let mut counts = Vec::new();
        for relation in [
            "game_reward_claim_mint_reservations",
            "game_reward_claim_mint_receipts",
            "game_reward_claims",
            "game_item_instances",
            "game_item_ground_locations",
            "game_item_container_entries",
            "game_item_audit_outbox",
        ] {
            counts.push(self.count(relation).await?);
        }
        Ok(counts)
    }

    async fn item_state(&self, item: [u8; 16]) -> TestResult<(i64, i16)> {
        let row = sqlx::query(
            "SELECT quantity, lifecycle FROM game_item_instances \
              WHERE item_instance_id = encode($1,'hex')::uuid",
        )
        .bind(item.as_slice())
        .fetch_one(&self.pool)
        .await?;
        Ok((row.try_get("quantity")?, row.try_get("lifecycle")?))
    }

    async fn on_ground(&self, item: [u8; 16]) -> TestResult<bool> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM game_item_ground_locations \
                             WHERE item_instance_id = encode($1,'hex')::uuid)",
        )
        .bind(item.as_slice())
        .fetch_one(&self.pool)
        .await?)
    }

    async fn character_revision(&self) -> TestResult<String> {
        Ok(sqlx::query_scalar(
            "SELECT character_revision::text FROM game_character_roots \
              WHERE character_id = encode($1,'hex')::uuid",
        )
        .bind(id(CHARACTER).as_slice())
        .fetch_one(&self.pool)
        .await?)
    }

    /// Change one durable authority fact outside the runtime guards.
    async fn tamper(&self, statement: &str) -> TestResult {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query(sqlx::AssertSqlSafe(statement.to_owned()))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn cleanup(self) -> TestResult {
        self.pool.close().await;
        self.database.cleanup().await?;
        std::fs::remove_dir_all(self.retained)?;
        Ok(())
    }
}

fn definition(family: &str, key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: family.into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    }
}

fn facts(key: &str, stack: ItemStackClass) -> ItemDefinitionFacts {
    ItemDefinitionFacts {
        definition: definition("Item", key),
        stack,
        container_capacity: None,
        container_slot_equip_pattern: false,
    }
}

/// Test-only backpack: capacity 20 and a complete `container`-slot pattern.
fn backpack_facts() -> ItemDefinitionFacts {
    ItemDefinitionFacts {
        definition: definition("Item", BACKPACK),
        stack: ItemStackClass::NonStackable,
        container_capacity: Some(20),
        container_slot_equip_pattern: true,
    }
}

const STACKABLE: ItemStackClass = ItemStackClass::Stackable {
    proven_maximum: None,
};

fn command(value: u64) -> TestResult<CommandRef> {
    Ok(CommandRef::new(
        GameSessionId::decode(&id(SESSION)).map_err(debug)?,
        CommandId::new(value).map_err(debug)?,
    ))
}

fn to_slot(command: CommandRef, item: [u8; 16], facts: ItemDefinitionFacts) -> ItemTransferRequest {
    ItemTransferRequest {
        command,
        source_item_instance_id: item,
        destination: ItemTransferDestination::ContainerSlot,
        item: facts,
        backpack: None,
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
    }
}

fn to_backpack(
    command: CommandRef,
    item: [u8; 16],
    facts: ItemDefinitionFacts,
) -> ItemTransferRequest {
    ItemTransferRequest {
        backpack: Some(backpack_facts()),
        destination: ItemTransferDestination::MainBackpack,
        ..to_slot(command, item, facts)
    }
}

fn fence() -> TestResult<CurrentCharacterItemFence> {
    Ok(CurrentCharacterItemFence {
        character_id: CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?,
        game_session_id: GameSessionId::decode(&id(SESSION)).map_err(debug)?,
        connection_generation: ConnectionGeneration::new(1).map_err(debug)?,
        character_lease_generation: 1,
        runtime_scope: scope()?,
        scope_ownership_generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
    })
}

fn chest(
    command: CommandRef,
    claim_key: &str,
    item: ItemDefinitionFacts,
    quantity: u32,
) -> RewardClaimMintRequest {
    RewardClaimMintRequest {
        command,
        claim: definition("RewardClaim", claim_key),
        source_placement: "fixture:placement.chest".into(),
        item,
        quantity,
        backpack: backpack_facts(),
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
    }
}

fn coins(command: CommandRef, claim_key: &str, quantity: u32) -> RewardClaimMintRequest {
    chest(command, claim_key, facts(COIN, STACKABLE), quantity)
}

/// A claim of its own per fence-operator command: a stale commit leaves its reservation
/// pending, which would refuse the next command on a shared claim as ClaimPending (§17.2).
fn operator_claim(command: u64) -> String {
    format!("fixture:chest.operator.{command}")
}

fn refused(
    result: Result<impl std::fmt::Debug, RewardClaimMintError>,
    expected: RewardClaimRefusal,
) -> TestResult {
    match result {
        Err(RewardClaimMintError::Refused(reason)) if reason == expected => Ok(()),
        other => Err(format!("expected refusal {expected:?}, got {other:?}").into()),
    }
}

fn rejected(result: Result<impl std::fmt::Debug, RewardClaimMintError>, label: &str) -> TestResult {
    match result {
        Err(RewardClaimMintError::AuthorityRejected) => Ok(()),
        other => Err(format!("{label}: expected AuthorityRejected, got {other:?}").into()),
    }
}

fn at(parent: [u8; 16], ordinal: u64) -> Option<ContainerEntryPosition> {
    Some(ContainerEntryPosition {
        parent_item_instance_id: parent,
        placement_ordinal: ordinal,
    })
}

#[test]
fn claim_mints_one_item_into_a_new_entry_and_a_once_claim_never_repeats() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "claim").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        // No backpack equipped: refused with nothing written.
        let before = harness.footprint().await?;
        refused(
            harness
                .claim(&authority, fence()?, coins(command(1)?, CLAIM, 30))
                .await,
            RewardClaimRefusal::NoMainBackpack,
        )?;
        assert_eq!(harness.footprint().await?, before);

        let backpack = harness.equip_backpack(&authority, 2).await?;
        // A compatible coin stack with room is already in the backpack.
        let stack = harness.mint(&authority, COIN, 10).await?;
        harness
            .committed(
                &authority,
                to_backpack(command(3)?, stack, facts(COIN, STACKABLE)),
            )
            .await?;

        let first = harness
            .claimed(&authority, coins(command(10)?, CLAIM, 30))
            .await?;
        // A new entry, never a merge into the existing stack (§39.1).
        assert_eq!(first.destination, at(backpack, 2).ok_or("entry")?);
        assert_eq!(first.quantity, 30);
        assert_eq!(harness.item_state(first.item_instance_id).await?, (30, 1));
        assert_eq!(harness.item_state(stack).await?, (10, 1));
        assert!(!harness.on_ground(first.item_instance_id).await?);
        let read = harness
            .root
            .read_character_backpack(&authority, fence()?.character_id)
            .await
            .map_err(debug)?
            .ok_or("backpack")?;
        let order: Vec<_> = read
            .entries
            .iter()
            .map(|entry| (entry.item.item_instance_id, entry.placement_ordinal))
            .collect();
        assert_eq!(order, vec![(first.item_instance_id, 2), (stack, 1)]);

        // The RewardClaim is recorded once, `once` (no next allowed time),
        // and the claim does not advance CharacterRevision.
        let claim = sqlx::query(
            "SELECT claim_revision_ref, next_allowed_at IS NULL AS once, \
                    claimed_transaction_id::text \
               FROM game_reward_claims WHERE character_id = encode($1,'hex')::uuid",
        )
        .bind(id(CHARACTER).as_slice())
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(
            claim.try_get::<String, _>("claim_revision_ref")?,
            "definition-r1"
        );
        assert!(claim.try_get::<bool, _>("once")?);
        assert_eq!(
            claim.try_get::<String, _>("claimed_transaction_id")?,
            uuid_text(first.transaction_id)
        );
        assert_eq!(harness.character_revision().await?, "1");
        assert_eq!(harness.count("game_character_xp_receipts").await?, 0);

        // The audit event is the registered reward-claim MINT of the receipt.
        let row = sqlx::query(
            "SELECT a.envelope, a.envelope_sha256 = r.envelope_sha256 AS digest_ok \
               FROM game_item_audit_outbox a \
               JOIN game_reward_claim_mint_receipts r USING (event_id, transaction_id) \
              WHERE a.transaction_id = encode($1,'hex')::uuid",
        )
        .bind(first.transaction_id.as_slice())
        .fetch_one(&harness.pool)
        .await?;
        assert!(row.try_get::<bool, _>("digest_ok")?);
        let (envelope, payload) =
            audit::decode_reward_claim_mint_envelope(&row.try_get::<Vec<u8>, _>("envelope")?)
                .map_err(debug)?;
        assert_eq!(envelope.command_id, Some(10));
        assert_eq!(envelope.channel_id, Some(id(CHANNEL).to_vec()));
        let source = payload.source.ok_or("cause")?;
        assert_eq!(
            source.reward_claim.map(|claim| claim.production_key),
            Some(CLAIM.to_owned())
        );
        assert_eq!(
            payload
                .destination
                .and_then(|destination| destination.container_entry)
                .map(|entry| entry.placement_ordinal),
            Some(2)
        );

        // Replay of the same command returns the first result; reconcile too.
        let mut replay = harness
            .root
            .freeze_reward_claim_mint(
                &authority,
                &harness.node,
                fence()?,
                coins(command(10)?, CLAIM, 30),
            )
            .await
            .map_err(debug)?;
        assert_eq!(replay.transaction_id(), &first.transaction_id);
        assert_eq!(replay.item_instance_id(), &first.item_instance_id);
        match harness
            .root
            .commit_reward_claim_mint(&authority, &harness.node, fence()?, &mut replay)
            .await
            .map_err(debug)?
        {
            RewardClaimMintOutcome::AlreadyCommitted(result) => assert_eq!(result, first),
            other => return Err(format!("expected the retained result, got {other:?}").into()),
        }
        assert_eq!(
            harness
                .root
                .reconcile_reward_claim_mint(&authority, &mut replay)
                .await
                .map_err(debug)?,
            Some(first.clone())
        );
        // DUR03-RL-08: the third unit is the last; the fourth is rejected.
        assert_eq!(replay.work_units_used(), 3);
        assert!(matches!(
            harness
                .root
                .reconcile_reward_claim_mint(&authority, &mut replay)
                .await,
            Err(RewardClaimMintError::CapacityExceeded)
        ));
        // Same command, different intent: integrity conflict.
        assert!(matches!(
            harness
                .root
                .freeze_reward_claim_mint(
                    &authority,
                    &harness.node,
                    fence()?,
                    coins(command(10)?, CLAIM, 31),
                )
                .await,
            Err(RewardClaimMintError::ConflictingCause)
        ));

        // D40: another USE of the claimed `once` chest is refused with
        // nothing written, also under a later claim revision.
        let before = harness.footprint().await?;
        refused(
            harness
                .claim(&authority, fence()?, coins(command(11)?, CLAIM, 30))
                .await,
            RewardClaimRefusal::AlreadyClaimed,
        )?;
        let mut revised = coins(command(12)?, CLAIM, 30);
        revised.claim.revision_ref = "definition-r2".into();
        refused(
            harness.claim(&authority, fence()?, revised).await,
            RewardClaimRefusal::AlreadyClaimed,
        )?;
        assert_eq!(harness.footprint().await?, before);

        // Another claim of the same Character takes the next entry.
        let stone = harness
            .claimed(
                &authority,
                chest(
                    command(13)?,
                    "fixture:chest.other",
                    facts(STONE, ItemStackClass::NonStackable),
                    1,
                ),
            )
            .await?;
        assert_eq!(stone.destination, at(backpack, 3).ok_or("entry")?);
        assert_eq!(harness.count("game_reward_claims").await?, 2);
        assert_eq!(harness.count("game_reward_claim_mint_receipts").await?, 2);
        assert_eq!(harness.count("game_item_ground_locations").await?, 0);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn refusals_and_invalid_input_write_nothing() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "refuse").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        harness.equip_backpack(&authority, 1).await?;
        let before = harness.footprint().await?;
        let mut bag = facts("fixture:chest.bag", ItemStackClass::NonStackable);
        bag.container_capacity = Some(8);
        let mut wrong_backpack = coins(command(8)?, CLAIM, 1);
        wrong_backpack.backpack.definition = definition("Item", "fixture:other.backpack");
        let mut oversized_backpack = coins(command(9)?, CLAIM, 1);
        oversized_backpack.backpack.container_capacity = Some(21);
        let cases: Vec<(RewardClaimMintRequest, RewardClaimRefusal)> = vec![
            (
                chest(command(2)?, CLAIM, bag, 1),
                RewardClaimRefusal::RewardIsContainer,
            ),
            (
                chest(command(3)?, CLAIM, facts(COIN, ItemStackClass::Unknown), 1),
                RewardClaimRefusal::UnknownStackClass,
            ),
            (
                chest(
                    command(4)?,
                    CLAIM,
                    facts(
                        COIN,
                        ItemStackClass::Stackable {
                            proven_maximum: Some(101),
                        },
                    ),
                    1,
                ),
                RewardClaimRefusal::UnsupportedStackMaximum,
            ),
            (
                coins(command(5)?, CLAIM, 101),
                RewardClaimRefusal::QuantityAboveStackMaximum,
            ),
            (
                chest(
                    command(6)?,
                    CLAIM,
                    facts(STONE, ItemStackClass::NonStackable),
                    2,
                ),
                RewardClaimRefusal::QuantityAboveStackMaximum,
            ),
            (wrong_backpack, RewardClaimRefusal::DefinitionMismatch),
            (
                oversized_backpack,
                RewardClaimRefusal::UnsupportedContainerCapacity,
            ),
        ];
        for (request, expected) in cases {
            refused(harness.claim(&authority, fence()?, request).await, expected)?;
            assert_eq!(harness.footprint().await?, before, "{expected:?}");
        }
        // Oversize input is rejected before any database work.
        let mut oversize = coins(command(20)?, CLAIM, 1);
        oversize.claim.production_key = "k".repeat(513);
        assert!(matches!(
            harness.claim(&authority, fence()?, oversize).await,
            Err(RewardClaimMintError::InvalidInput)
        ));
        assert!(matches!(
            harness
                .claim(&authority, fence()?, coins(command(22)?, CLAIM, 0))
                .await,
            Err(RewardClaimMintError::InvalidInput)
        ));
        let mut empty = coins(command(21)?, CLAIM, 1);
        empty.claim.family = String::new();
        assert!(matches!(
            harness.claim(&authority, fence()?, empty).await,
            Err(RewardClaimMintError::InvalidInput)
        ));
        assert_eq!(harness.footprint().await?, before);

        // D92 room: the last free entry (20th, max) is admitted...
        harness.fill_with_stones(&authority, 30, 19).await?;
        let pending_command = command(60)?;
        let mut pending = harness
            .root
            .freeze_reward_claim_mint(
                &authority,
                &harness.node,
                fence()?,
                coins(pending_command, "fixture:chest.late", 5),
            )
            .await
            .map_err(debug)?;
        let last = harness
            .claimed(&authority, coins(command(61)?, CLAIM, 100))
            .await?;
        assert_eq!(last.destination.placement_ordinal, 20);
        assert_eq!(harness.count("game_item_container_entries").await?, 20);
        // ...and max+1 is refused at freeze and, for a claim frozen while
        // room remained, at commit, with nothing written either way.
        let before = harness.footprint().await?;
        refused(
            harness
                .claim(
                    &authority,
                    fence()?,
                    coins(command(62)?, "fixture:chest.full", 1),
                )
                .await,
            RewardClaimRefusal::MainBackpackFull,
        )?;
        refused(
            harness
                .root
                .commit_reward_claim_mint(&authority, &harness.node, fence()?, &mut pending)
                .await,
            RewardClaimRefusal::MainBackpackFull,
        )?;
        assert_eq!(harness.footprint().await?, before);
        assert_eq!(
            harness
                .root
                .reconcile_reward_claim_mint(&authority, &mut pending)
                .await
                .map_err(debug)?,
            None
        );

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn every_fence_operator_rejects_at_freeze_and_at_commit() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "fence").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        harness.equip_backpack(&authority, 1).await?;
        let other_node = register(&harness.root, 2).await?;
        let mut next = 10_u64;

        // Fence operators: each changes exactly one fact of a valid fence.
        type Operator = (&'static str, fn(&mut CurrentCharacterItemFence));
        let operators: [Operator; 7] = [
            ("stale connection_generation", |f| {
                f.connection_generation = ConnectionGeneration::new(2).expect("generation");
            }),
            ("replaced GameSession", |f| {
                f.game_session_id = GameSessionId::decode(&id(51)).expect("session");
            }),
            ("moved lease generation", |f| {
                f.character_lease_generation = 2
            }),
            ("stale scope ownership generation", |f| {
                f.scope_ownership_generation = ScopeOwnershipGeneration::new(2).expect("gen");
            }),
            ("another Channel scope", |f| {
                f.runtime_scope = RuntimeScopeRefV1::channel(
                    WorldId::decode(&id(WORLD)).expect("world"),
                    ChannelId::decode(&id(44)).expect("channel"),
                );
            }),
            ("cause keyed to another Character", |f| {
                f.character_id = CharacterId::from_bytes(id(45)).expect("character");
            }),
            ("instance runtime scope", |f| {
                f.runtime_scope = RuntimeScopeRefV1::instance(
                    WorldId::decode(&id(WORLD)).expect("world"),
                    id(46),
                )
                .expect("instance");
            }),
        ];
        for (label, mutate) in operators {
            let mut stale = fence()?;
            mutate(&mut stale);
            // Freeze boundary: rejected, no reservation.
            next += 1;
            let before = harness.footprint().await?;
            rejected(
                harness
                    .root
                    .freeze_reward_claim_mint(
                        &authority,
                        &harness.node,
                        stale,
                        coins(command(next)?, &operator_claim(next), 1),
                    )
                    .await,
                label,
            )?;
            assert_eq!(harness.footprint().await?, before, "{label}");
            // Commit boundary: a valid freeze, then a stale commit mints nothing.
            next += 1;
            let mut candidate = harness
                .root
                .freeze_reward_claim_mint(
                    &authority,
                    &harness.node,
                    fence()?,
                    coins(command(next)?, &operator_claim(next), 1),
                )
                .await
                .map_err(debug)?;
            let before = harness.footprint().await?;
            rejected(
                harness
                    .root
                    .commit_reward_claim_mint(&authority, &harness.node, stale, &mut candidate)
                    .await,
                label,
            )?;
            assert_eq!(harness.footprint().await?, before, "{label}");
        }

        // The CommandRef of another GameSession under a valid fence.
        let mut foreign = coins(command(90)?, CLAIM, 1);
        foreign.command = CommandRef::new(
            GameSessionId::decode(&id(51)).map_err(debug)?,
            CommandId::new(90).map_err(debug)?,
        );
        rejected(
            harness.claim(&authority, fence()?, foreign).await,
            "foreign CommandRef",
        )?;

        // Runtime assignment held by another node incarnation.
        next += 1;
        rejected(
            harness
                .root
                .freeze_reward_claim_mint(
                    &authority,
                    &other_node,
                    fence()?,
                    coins(command(next)?, &operator_claim(next), 1),
                )
                .await,
            "assignment holder",
        )?;
        next += 1;
        let mut candidate = harness
            .root
            .freeze_reward_claim_mint(
                &authority,
                &harness.node,
                fence()?,
                coins(command(next)?, &operator_claim(next), 1),
            )
            .await
            .map_err(debug)?;
        rejected(
            harness
                .root
                .commit_reward_claim_mint(&authority, &other_node, fence()?, &mut candidate)
                .await,
            "assignment holder",
        )?;

        // Durable-state operators: session outside states 1-2, ineligible
        // character guard, runtime guard not ready.
        let session = |set: &str| {
            format!(
                "UPDATE game_durability_reconnect_sessions SET {set} \
                  WHERE game_session_id = '{}'",
                uuid_text(id(SESSION))
            )
        };
        let guard = |set: &str| {
            format!(
                "UPDATE game_durability_admission_character_guards SET {set} \
                  WHERE character_id = '{}'",
                uuid_text(id(CHARACTER))
            )
        };
        let durable: [(&str, String, String); 3] = [
            (
                "session_state 3",
                session("session_state = 3"),
                session("session_state = 1"),
            ),
            (
                "ineligible character guard",
                guard("eligible = false"),
                guard("eligible = true"),
            ),
            (
                "runtime guard not ready",
                "UPDATE game_durability_admission_runtime_guards SET ready = false".into(),
                "UPDATE game_durability_admission_runtime_guards SET ready = true".into(),
            ),
        ];
        for (label, break_fact, restore) in durable {
            next += 1;
            let mut candidate = harness
                .root
                .freeze_reward_claim_mint(
                    &authority,
                    &harness.node,
                    fence()?,
                    coins(command(next)?, &operator_claim(next), 1),
                )
                .await
                .map_err(debug)?;
            harness.tamper(&break_fact).await?;
            next += 1;
            rejected(
                harness
                    .root
                    .freeze_reward_claim_mint(
                        &authority,
                        &harness.node,
                        fence()?,
                        coins(command(next)?, &operator_claim(next), 1),
                    )
                    .await,
                label,
            )?;
            rejected(
                harness
                    .root
                    .commit_reward_claim_mint(&authority, &harness.node, fence()?, &mut candidate)
                    .await,
                label,
            )?;
            harness.tamper(&restore).await?;
        }
        assert_eq!(harness.count("game_reward_claims").await?, 0);

        // DUR-03 §31: a still-pending CommandRef commits after an eligible
        // same-GameSession reconnect with the current connection generation;
        // the predecessor generation cannot.
        next += 1;
        let pending = command(next)?;
        let mut candidate = harness
            .root
            .freeze_reward_claim_mint(
                &authority,
                &harness.node,
                fence()?,
                coins(pending, CLAIM, 1),
            )
            .await
            .map_err(debug)?;
        harness.tamper(&session("current_generation = 2")).await?;
        let mut reconnected = fence()?;
        reconnected.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        rejected(
            harness
                .root
                .commit_reward_claim_mint(&authority, &harness.node, fence()?, &mut candidate)
                .await,
            "predecessor connection generation",
        )?;
        let continued = match harness
            .root
            .commit_reward_claim_mint(&authority, &harness.node, reconnected, &mut candidate)
            .await
            .map_err(debug)?
        {
            RewardClaimMintOutcome::Committed(result) => result,
            other => return Err(format!("expected commit, got {other:?}").into()),
        };

        // Ended node incarnation: freeze and commit both rejected.
        next += 1;
        let mut candidate = harness
            .root
            .freeze_reward_claim_mint(
                &authority,
                &harness.node,
                reconnected,
                coins(command(next)?, "fixture:chest.late", 1),
            )
            .await
            .map_err(debug)?;
        harness
            .root
            .revoke_node_registration(harness.node.fact())
            .await
            .map_err(debug)?;
        rejected(
            harness
                .root
                .commit_reward_claim_mint(&authority, &harness.node, reconnected, &mut candidate)
                .await,
            "ended node incarnation",
        )?;
        next += 1;
        rejected(
            harness
                .root
                .freeze_reward_claim_mint(
                    &authority,
                    &harness.node,
                    reconnected,
                    coins(command(next)?, "fixture:chest.late", 1),
                )
                .await,
            "ended node incarnation",
        )?;

        // A retry after commit replays under the current recovery fence; a
        // stale recovery fence returns no outcome at any boundary.
        let mut replay = harness
            .root
            .freeze_reward_claim_mint(
                &authority,
                &harness.node,
                reconnected,
                coins(pending, CLAIM, 1),
            )
            .await
            .map_err(debug)?;
        assert_eq!(
            harness
                .root
                .reconcile_reward_claim_mint(&authority, &mut replay)
                .await
                .map_err(debug)?,
            Some(continued)
        );
        harness
            .tamper(&format!(
                "INSERT INTO game_character_recovery_admissions(authority_scope_id, \
                   recovery_generation, recovery_event_id, predecessor_generation, \
                   predecessor_digest, issued_at, issuer_identity, reconciled_at) \
                 VALUES ('character-primary', 2, '{}', 1, decode(repeat('ab', 32), 'hex'), \
                   200, 'game-ops', 200)",
                uuid_text(id(12))
            ))
            .await?;
        assert!(matches!(
            harness
                .root
                .freeze_reward_claim_mint(
                    &authority,
                    &harness.node,
                    reconnected,
                    coins(pending, CLAIM, 1),
                )
                .await,
            Err(RewardClaimMintError::Unavailable(_))
        ));
        assert!(matches!(
            harness
                .root
                .reconcile_reward_claim_mint(&authority, &mut candidate)
                .await,
            Err(RewardClaimMintError::Unavailable(_))
        ));
        assert_eq!(harness.count("game_reward_claim_mint_receipts").await?, 1);
        assert_eq!(harness.character_revision().await?, "1");

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// Raw SQL as the database owner cannot bypass the guards: a RewardClaim,
/// an item, a backpack entry or a receipt that is not one complete
/// reward-claim MINT of the same physical transaction is rejected at COMMIT.
#[test]
fn database_rejects_unproven_reward_claim_rows() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "guard").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let backpack = harness.equip_backpack(&authority, 1).await?;
        let committed = harness
            .claimed(&authority, coins(command(2)?, CLAIM, 3))
            .await?;
        let uuid = uuid_text;
        let character = uuid(id(CHARACTER));
        let world = uuid(id(WORLD));
        let fresh_item = |tx: u8| {
            format!(
                "INSERT INTO game_item_instances(item_instance_id, world_id, definition_family, \
                   definition_production_key, definition_revision_ref, quantity, lifecycle, \
                   minted_transaction_id) VALUES ('{}', '{world}', 'Item', '{COIN}', \
                   'definition-r1', 1, 1, '{}')",
                uuid(id(tx + 1)),
                uuid(id(tx))
            )
        };
        let attempts: Vec<Vec<String>> = vec![
            // A RewardClaim without a MINT.
            vec![format!(
                "INSERT INTO game_reward_claims VALUES ('{character}', 'RewardClaim', \
                   'fixture:chest.forged', 'definition-r1', NULL, '{}', 1)",
                uuid(id(150))
            )],
            // D42: a second RewardClaim of the claimed claim, under another
            // revision and the committed claim's transaction.
            vec![format!(
                "INSERT INTO game_reward_claims VALUES ('{character}', 'RewardClaim', \
                   '{CLAIM}', 'definition-r9', NULL, '{}', 1)",
                uuid(committed.transaction_id)
            )],
            // A cooldown next allowed time (CHEST-1 is `once` only).
            vec![format!(
                "INSERT INTO game_reward_claims VALUES ('{character}', 'RewardClaim', \
                   'fixture:chest.cooldown', 'definition-r1', 5, '{}', 1)",
                uuid(id(151))
            )],
            // An item with no Ground MINT and no reward-claim receipt.
            vec![fresh_item(160)],
            // An item placed in a backpack entry without any receipt.
            vec![
                fresh_item(170),
                format!(
                    "INSERT INTO game_item_container_entries VALUES ('{}', '{world}', \
                       '{character}', '{}', 9, '{}')",
                    uuid(id(171)),
                    uuid(backpack),
                    uuid(id(170))
                ),
            ],
            // A second, Ground location for the claimed item through a forged
            // MINT receipt of the current transaction.
            vec![
                format!(
                    "INSERT INTO game_item_mint_receipts(death_world_id, death_channel_id, \
                       death_scope_ownership_generation, death_actor_local_id, \
                       death_actor_local_generation, loot_table_family, \
                       loot_table_production_key, loot_table_revision_ref, loot_purpose_key, \
                       draw_ordinal, intent_binding, transaction_id, event_id, \
                       item_instance_id, occurred_at, envelope_sha256, committed_at) \
                     VALUES ('{world}', '{}', 1, 900, 1, 'LootTable', 'fixture:loot.forged', \
                       'definition-r1', 'fixture:purpose', 0, decode(repeat('ab',33),'hex'), \
                       '{}', '{}', '{}', 1000, sha256(decode('ab','hex')), 1000)",
                    uuid(id(CHANNEL)),
                    uuid(id(175)),
                    uuid(id(176)),
                    uuid(committed.item_instance_id)
                ),
                format!(
                    "INSERT INTO game_item_ground_locations VALUES ('{}', '{world}', '{}', 1, \
                       decode('01','hex'), decode('01','hex'), 'map-1', 'content-1', \
                       decode('01','hex'))",
                    uuid(committed.item_instance_id),
                    uuid(id(CHANNEL))
                ),
            ],
            // A complete forged MINT without its reservation.
            forge_claim_statements(&character, &world, backpack, 180, 2, 3, false),
            // A complete forged MINT whose receipt quantity is not the item's.
            forge_claim_statements(&character, &world, backpack, 190, 2, 4, true),
            // Committed claims, receipts and reservations are immutable.
            vec!["UPDATE game_reward_claims SET claimed_at = 0".into()],
            vec!["DELETE FROM game_reward_claims".into()],
            vec!["UPDATE game_reward_claim_mint_receipts SET quantity = 4".into()],
            vec!["UPDATE game_reward_claim_mint_reservations SET occurred_at = 5".into()],
            vec!["TRUNCATE game_reward_claims CASCADE".into()],
            vec!["TRUNCATE game_reward_claim_mint_receipts CASCADE".into()],
        ];
        for statements in attempts {
            let before = harness.footprint().await?;
            let mut tx = harness.pool.begin().await?;
            let applied = async {
                for statement in &statements {
                    sqlx::query(sqlx::AssertSqlSafe(statement.clone()))
                        .execute(&mut *tx)
                        .await?;
                }
                tx.commit().await
            }
            .await;
            assert!(applied.is_err(), "accepted: {statements:?}");
            assert_eq!(harness.footprint().await?, before, "{statements:?}");
        }
        // The positive control: the same forged statements with their
        // reservation and a matching quantity pass every guard, so the
        // rejections above are caused by the one broken fact each.
        let mut tx = harness.pool.begin().await?;
        for statement in forge_claim_statements(&character, &world, backpack, 200, 2, 2, true) {
            sqlx::query(sqlx::AssertSqlSafe(statement))
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        assert_eq!(harness.count("game_reward_claims").await?, 2);

        // Least privilege: the runtime role may only insert and charge.
        let grants: Vec<bool> = sqlx::query_scalar(
            "SELECT unnest(ARRAY[\
               has_table_privilege('oteryn_game_runtime','game_reward_claims','INSERT'),\
               NOT has_table_privilege('oteryn_game_runtime','game_reward_claims','UPDATE'),\
               NOT has_table_privilege('oteryn_game_runtime','game_reward_claims','DELETE'),\
               has_table_privilege('oteryn_game_runtime','game_reward_claim_mint_receipts',\
                 'INSERT'),\
               NOT has_table_privilege('oteryn_game_runtime','game_reward_claim_mint_receipts',\
                 'UPDATE'),\
               has_column_privilege('oteryn_game_runtime','game_reward_claim_mint_reservations',\
                 'work_units_used','UPDATE'),\
               NOT has_column_privilege('oteryn_game_runtime',\
                 'game_reward_claim_mint_reservations','transaction_id','UPDATE'),\
               NOT has_table_privilege('oteryn_game_runtime',\
                 'game_reward_claim_mint_reservations','DELETE'),\
               has_table_privilege('oteryn_game_control','game_reward_claims','SELECT'),\
               NOT has_table_privilege('oteryn_game_control','game_reward_claims','INSERT'),\
               has_table_privilege('oteryn_game_control','game_reward_claim_mint_receipts',\
                 'SELECT'),\
               NOT has_table_privilege('oteryn_game_control',\
                 'game_reward_claim_mint_reservations','INSERT')])",
        )
        .fetch_all(&harness.pool)
        .await?;
        assert_eq!(grants.len(), 12);
        assert!(grants.iter().all(|granted| *granted), "{grants:?}");

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// Every row of one reward-claim MINT, forged in raw SQL under transaction
/// `id(tag)`, event `id(tag + 2)` and item `id(tag + 1)`, with or without its
/// reservation and with the receipt claiming `receipt_quantity` for an item of
/// `item_quantity`.
fn forge_claim_statements(
    character: &str,
    world: &str,
    backpack: [u8; 16],
    tag: u8,
    item_quantity: u32,
    receipt_quantity: u32,
    with_reservation: bool,
) -> Vec<String> {
    let uuid = uuid_text;
    let tx = uuid(id(tag));
    let item = uuid(id(tag + 1));
    let ev = uuid(id(tag + 2));
    let parent = uuid(backpack);
    let channel = uuid(id(CHANNEL));
    let claim = format!("fixture:chest.forged{tag}");
    let command_id = u64::from(tag) + 1000;
    let mut statements = Vec::new();
    if with_reservation {
        statements.push(format!(
            "INSERT INTO game_reward_claim_mint_reservations VALUES \
             ('{character}', {command_id}, '{character}', '{world}', '{channel}', 'RewardClaim', \
              '{claim}', 'definition-r1', decode(repeat('ab',33),'hex'), '{tx}', '{ev}', \
              '{item}', 1000, 1, 900)"
        ));
    }
    statements.extend([
        format!(
            "INSERT INTO game_item_instances(item_instance_id, world_id, definition_family, \
               definition_production_key, definition_revision_ref, quantity, lifecycle, \
               minted_transaction_id) VALUES ('{item}', '{world}', 'Item', '{COIN}', \
               'definition-r1', {item_quantity}, 1, '{tx}')"
        ),
        format!(
            "INSERT INTO game_item_container_entries VALUES \
             ('{item}', '{world}', '{character}', '{parent}', {tag}, '{tx}')"
        ),
        format!(
            "INSERT INTO game_item_audit_outbox(event_id, transaction_id, transaction_ordinal, \
               transaction_count, event_type_id, schema_revision, retention_profile_id, \
               item_instance_id, occurred_at, expires_at, envelope, envelope_sha256, \
               publication_state) VALUES \
             ('{ev}', '{tx}', 1, 1, 2, 1, 'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1', \
               '{item}', 1000, 7776001000, decode(repeat('ab',16),'hex'), \
               sha256(decode(repeat('ab',16),'hex')), 1)"
        ),
        format!(
            "INSERT INTO game_reward_claims VALUES ('{character}', 'RewardClaim', '{claim}', \
               'definition-r1', NULL, '{tx}', 1000)"
        ),
        format!(
            "INSERT INTO game_reward_claim_mint_receipts(game_session_id, command_id, \
               character_id, claim_family, claim_production_key, claim_revision_ref, \
               intent_binding, transaction_id, event_id, item_instance_id, quantity, \
               destination_parent_item_instance_id, destination_ordinal, occurred_at, \
               envelope_sha256, committed_at) VALUES \
             ('{character}', {command_id}, '{character}', 'RewardClaim', '{claim}', \
               'definition-r1', decode(repeat('ab',33),'hex'), '{tx}', '{ev}', '{item}', \
               {receipt_quantity}, '{parent}', {tag}, 1000, \
               sha256(decode(repeat('ab',16),'hex')), 1000)"
        ),
    ]);
    statements
}

#[test]
fn concurrent_claims_mint_once_and_serialize_with_pickup_and_xp() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "concurrent").await?;
        let second_root = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
        assert!(second_root.maintain_ready_once().await?);
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let first = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let second = second_root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        harness.equip_backpack(&first, 1).await?;

        // The same CommandRef on two roots claims exactly once.
        let request = coins(command(2)?, CLAIM, 5);
        let mut one = harness
            .root
            .freeze_reward_claim_mint(&first, &harness.node, fence()?, request.clone())
            .await
            .map_err(debug)?;
        let mut two = second_root
            .freeze_reward_claim_mint(&second, &harness.node, fence()?, request)
            .await
            .map_err(debug)?;
        assert_eq!(one.transaction_id(), two.transaction_id());
        let (left, right) = join_two(
            harness
                .root
                .commit_reward_claim_mint(&first, &harness.node, fence()?, &mut one),
            second_root.commit_reward_claim_mint(&second, &harness.node, fence()?, &mut two),
        )
        .await;
        match (left.map_err(debug)?, right.map_err(debug)?) {
            (RewardClaimMintOutcome::Committed(a), RewardClaimMintOutcome::AlreadyCommitted(b))
            | (RewardClaimMintOutcome::AlreadyCommitted(b), RewardClaimMintOutcome::Committed(a)) =>
            {
                assert_eq!(a, b);
            }
            other => return Err(format!("expected one commit, got {other:?}").into()),
        }

        // Two USE commands of one `once` claim race to freeze on two roots: exactly one
        // reserves, the other is refused as ClaimPending (GAME-INTERACTION §17.2) with nothing
        // written, and after the winner commits it is refused as AlreadyClaimed.
        let (left, right) = join_two(
            harness.root.freeze_reward_claim_mint(
                &first,
                &harness.node,
                fence()?,
                coins(command(3)?, "fixture:chest.race", 1),
            ),
            second_root.freeze_reward_claim_mint(
                &second,
                &harness.node,
                fence()?,
                coins(command(4)?, "fixture:chest.race", 1),
            ),
        )
        .await;
        let (mut winner, loser_command, winner_on_first) = match (left, right) {
            (
                Ok(candidate),
                Err(RewardClaimMintError::Refused(RewardClaimRefusal::ClaimPending)),
            ) => (candidate, command(4)?, true),
            (
                Err(RewardClaimMintError::Refused(RewardClaimRefusal::ClaimPending)),
                Ok(candidate),
            ) => (candidate, command(3)?, false),
            other => return Err(format!("expected one reservation, got {other:?}").into()),
        };
        let reservations: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_reward_claim_mint_reservations \
              WHERE claim_production_key = 'fixture:chest.race'",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(reservations, 1);
        let committed = if winner_on_first {
            harness
                .root
                .commit_reward_claim_mint(&first, &harness.node, fence()?, &mut winner)
                .await
        } else {
            second_root
                .commit_reward_claim_mint(&second, &harness.node, fence()?, &mut winner)
                .await
        };
        assert!(
            matches!(committed, Ok(RewardClaimMintOutcome::Committed(_))),
            "{committed:?}"
        );
        refused(
            harness
                .root
                .freeze_reward_claim_mint(
                    &first,
                    &harness.node,
                    fence()?,
                    coins(loser_command, "fixture:chest.race", 1),
                )
                .await,
            RewardClaimRefusal::AlreadyClaimed,
        )?;
        assert_eq!(harness.count("game_reward_claims").await?, 2);
        assert_eq!(harness.count("game_item_container_entries").await?, 2);

        // An XP award and a claim for one Character serialize on
        // character_root without deadlock; only the XP award advances the
        // revision.
        let mut late = harness
            .root
            .freeze_reward_claim_mint(
                &first,
                &harness.node,
                fence()?,
                coins(command(50)?, "fixture:chest.xp", 1),
            )
            .await
            .map_err(debug)?;
        let xp_fence = CurrentCharacterGameplayFence {
            character_id: fence()?.character_id,
            game_session_id: fence()?.game_session_id,
            connection_generation: fence()?.connection_generation,
            character_lease_generation: 1,
            runtime_scope: scope()?,
            scope_ownership_generation: fence()?.scope_ownership_generation,
            expected_character_revision: crate::domain::CharacterRevision::new(1).map_err(debug)?,
        };
        let (xp, claimed) = join_two(
            second_root.commit_character_experience(
                &second,
                &harness.node,
                xp_fence,
                xp_request(70)?,
            ),
            harness
                .root
                .commit_reward_claim_mint(&first, &harness.node, fence()?, &mut late),
        )
        .await;
        assert!(matches!(
            xp.map_err(debug)?,
            ExperienceCommitOutcome::Committed(_)
        ));
        assert!(matches!(
            claimed.map_err(debug)?,
            RewardClaimMintOutcome::Committed(_)
        ));
        assert_eq!(harness.character_revision().await?, "2");
        assert_eq!(harness.count("game_item_container_entries").await?, 3);

        // One free entry left: a pickup and a claim race for it; exactly one
        // wins and the other is refused as a full backpack.
        harness.fill_with_stones(&first, 10, 16).await?;
        assert_eq!(harness.count("game_item_container_entries").await?, 19);
        let stone = harness.mint(&first, STONE, 1).await?;
        let mut pickup = harness
            .root
            .freeze_item_transfer(
                &first,
                &harness.node,
                fence()?,
                to_backpack(
                    command(40)?,
                    stone,
                    facts(STONE, ItemStackClass::NonStackable),
                ),
            )
            .await
            .map_err(debug)?;
        let mut claim = second_root
            .freeze_reward_claim_mint(
                &second,
                &harness.node,
                fence()?,
                coins(command(41)?, "fixture:chest.room", 1),
            )
            .await
            .map_err(debug)?;
        let (moved, minted) = join_two(
            harness
                .root
                .commit_item_transfer(&first, &harness.node, fence()?, &mut pickup),
            second_root.commit_reward_claim_mint(&second, &harness.node, fence()?, &mut claim),
        )
        .await;
        match (moved, minted) {
            (
                Ok(ItemTransferOutcome::Committed(_)),
                Err(RewardClaimMintError::Refused(RewardClaimRefusal::MainBackpackFull)),
            )
            | (
                Err(ItemTransferError::Refused(ItemTransferRefusal::MainBackpackFull)),
                Ok(RewardClaimMintOutcome::Committed(_)),
            ) => {}
            other => return Err(format!("expected one winner, got {other:?}").into()),
        }
        assert_eq!(harness.count("game_item_container_entries").await?, 20);

        drop(first);
        drop(second);
        drop(seal);
        drop(second_root);
        harness.cleanup().await
    })
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
