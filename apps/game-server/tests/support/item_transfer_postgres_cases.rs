// Shared B3-1 DUR-03 TRANSFER cases (Ground -> CharacterEquipment container
// slot / main backpack entries, D83 merge shapes). Both wrappers provide the
// same path-loaded crate root.
//
// Content has no definition with a known `container`-slot equip pattern yet
// (the backpack oteryn:item.tibia.i2854 declares capacity 20 but its
// equipment semantics are UNKNOWN), so the backpack below is a test-only
// fixture definition, like the D1 CombatDeathFixture.

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
    ItemTransferRefusal, ItemTransferRequest, TransferShape,
};
use crate::durability::item_transfer_audit as audit;
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

// `pub(crate)` on the generic harness pieces below (TestResult, WORLD/CHANNEL/CHARACTER/
// SESSION, id, debug, configured_admin, runtime, Database, fence_parent, register, scope,
// seed_character, Harness) lets `combat_pickup_postgres_cases.rs` reuse the exact same
// PostgreSQL 17 DB/admission/session bootstrap (B3-2, `combat_pickup_postgres.rs`) instead of
// duplicating it; behavior is unchanged.
pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(crate) const WORLD: u8 = 42;
pub(crate) const CHANNEL: u8 = 43;
pub(crate) const CHARACTER: u8 = 41;
pub(crate) const SESSION: u8 = 50;
pub(crate) const BACKPACK: &str = "fixture:b3.backpack";
const COIN: &str = "fixture:b3.coin";
const STONE: &str = "fixture:b3.stone";

pub(crate) fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

/// Canonical hyphenated UUID text of `value`.
pub(crate) fn uuid_text(value: [u8; 16]) -> String {
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

pub(crate) fn debug<E: std::fmt::Debug>(error: E) -> String {
    format!("{error:?}")
}

pub(crate) async fn join_two<A, B>(first: A, second: B) -> (A::Output, B::Output)
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

pub(crate) fn configured_admin() -> Option<String> {
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

pub(crate) fn runtime() -> TestResult<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?)
}

pub(crate) struct Database {
    admin_url: String,
    name: String,
    pub(crate) url: String,
}

impl Database {
    pub(crate) async fn create(admin_url: String, name: &str) -> TestResult<Self> {
        if !admin_url.starts_with("postgresql://oteryn_test_admin:")
            || !admin_url.ends_with("@127.0.0.1:5432/postgres")
        {
            return Err("unsafe PostgreSQL test admin URL".into());
        }
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let name = format!("it_{name}_{suffix}");
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
        eprintln!("ITEM-TRANSFER-PG: database {name} on server_version_num={version}");
        sqlx::migrate!("./migrations").run(&mut connection).await?;
        connection.close().await?;
        Ok(Self {
            admin_url,
            name,
            url,
        })
    }

    pub(crate) async fn cleanup(self) -> TestResult {
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

pub(crate) fn fence_parent(tag: &str) -> TestResult<std::path::PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let parent = std::env::temp_dir().join(format!(
        "oteryn-item-transfer-parent-{}",
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

pub(crate) async fn register(root: &DurabilityRoot, tag: u8) -> TestResult<NodeIncarnationProof> {
    let secret = BootstrapSecret::from_bytes([tag; 32]);
    let launch = LaunchBinding::new(&format!("item-transfer-launch-{tag}")).map_err(debug)?;
    let node = crate::foundation::NodeId::decode(&id(tag)).map_err(debug)?;
    root.issue_node_bootstrap_authorization(&secret, &launch, None)
        .await
        .map_err(debug)?;
    root.register_node_incarnation(&secret, &launch, node)
        .await
        .map_err(|error| debug(error).into())
}

pub(crate) fn scope() -> TestResult<RuntimeScopeRefV1> {
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

/// Character 41 (account 40, World 42) with progression state, an active
/// GameSession 50 in Channel 43, its admission guards, the runtime-scope
/// assignment to `node` and runtime readiness.
pub(crate) async fn seed_character(
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
          1,1,'profile-1','ruleset-1','content-1','starter-1','Fixture Hero')",
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
    let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "item-transfer-writer")
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
            decision_identity: "item-transfer-runtime-ready-1".into(),
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

pub(crate) struct Harness {
    pub(crate) database: Database,
    pub(crate) root: DurabilityRoot,
    pub(crate) pool: sqlx::PgPool,
    pub(crate) recovery: CharacterRecoveryStore,
    retained: std::path::PathBuf,
    pub(crate) node: NodeIncarnationProof,
    next_actor: std::cell::Cell<u32>,
}

impl Harness {
    pub(crate) async fn create(admin: String, tag: &str) -> TestResult<Self> {
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
    pub(crate) async fn mint(
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

    pub(crate) async fn transfer(
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

    pub(crate) async fn count(&self, relation: &str) -> TestResult<i64> {
        Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {relation}"
        )))
        .fetch_one(&self.pool)
        .await?)
    }

    /// Row counts of every relation a TRANSFER could write.
    pub(crate) async fn footprint(&self) -> TestResult<Vec<i64>> {
        let mut counts = Vec::new();
        for relation in [
            "game_item_transfer_reservations",
            "game_item_transfer_receipts",
            "game_item_ground_locations",
            "game_item_container_slots",
            "game_item_container_entries",
            "game_item_audit_outbox",
            "game_item_instances WHERE last_transaction_id IS NOT NULL",
        ] {
            counts.push(self.count(relation).await?);
        }
        Ok(counts)
    }

    pub(crate) async fn item_state(&self, item: [u8; 16]) -> TestResult<(i64, i16)> {
        let row = sqlx::query(
            "SELECT quantity, lifecycle FROM game_item_instances \
              WHERE item_instance_id = encode($1,'hex')::uuid",
        )
        .bind(item.as_slice())
        .fetch_one(&self.pool)
        .await?;
        Ok((row.try_get("quantity")?, row.try_get("lifecycle")?))
    }

    pub(crate) async fn on_ground(&self, item: [u8; 16]) -> TestResult<bool> {
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
    pub(crate) async fn tamper(&self, statement: &str) -> TestResult {
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

    pub(crate) async fn cleanup(self) -> TestResult {
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
pub(crate) fn backpack_facts() -> ItemDefinitionFacts {
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

pub(crate) fn command(value: u64) -> TestResult<CommandRef> {
    Ok(CommandRef::new(
        GameSessionId::decode(&id(SESSION)).map_err(debug)?,
        CommandId::new(value).map_err(debug)?,
    ))
}

pub(crate) fn to_slot(
    command: CommandRef,
    item: [u8; 16],
    facts: ItemDefinitionFacts,
) -> ItemTransferRequest {
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

pub(crate) fn to_backpack(
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

pub(crate) fn fence() -> TestResult<CurrentCharacterItemFence> {
    Ok(CurrentCharacterItemFence {
        character_id: CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?,
        game_session_id: GameSessionId::decode(&id(SESSION)).map_err(debug)?,
        connection_generation: ConnectionGeneration::new(1).map_err(debug)?,
        character_lease_generation: 1,
        runtime_scope: scope()?,
        scope_ownership_generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
    })
}

pub(crate) fn refused(
    result: Result<impl std::fmt::Debug, ItemTransferError>,
    expected: ItemTransferRefusal,
) -> TestResult {
    match result {
        Err(ItemTransferError::Refused(reason)) if reason == expected => Ok(()),
        other => Err(format!("expected refusal {expected:?}, got {other:?}").into()),
    }
}

fn rejected(result: Result<impl std::fmt::Debug, ItemTransferError>, label: &str) -> TestResult {
    match result {
        Err(ItemTransferError::AuthorityRejected) => Ok(()),
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
fn slot_then_entries_newest_first_replay_and_no_revision_advance() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "slot").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let backpack = harness.mint(&authority, BACKPACK, 1).await?;
        let first = harness.mint(&authority, STONE, 1).await?;
        let second = harness.mint(&authority, STONE, 1).await?;
        let stone = || facts(STONE, ItemStackClass::NonStackable);

        // No backpack equipped: refused, the item stays on Ground.
        let before = harness.footprint().await?;
        refused(
            harness
                .transfer(
                    &authority,
                    fence()?,
                    to_backpack(command(1)?, first, stone()),
                )
                .await,
            ItemTransferRefusal::NoMainBackpack,
        )?;
        assert_eq!(harness.footprint().await?, before);

        // The empty backpack with a container-slot pattern enters the slot.
        let equipped = harness
            .committed(&authority, to_slot(command(2)?, backpack, backpack_facts()))
            .await?;
        assert_eq!(equipped.shape, TransferShape::ContainerSlot);
        assert_eq!(equipped.destination, None);
        let one = harness
            .committed(&authority, to_backpack(command(3)?, first, stone()))
            .await?;
        assert_eq!(
            (one.shape, one.destination),
            (TransferShape::NewEntry, at(backpack, 1))
        );
        let two = harness
            .committed(&authority, to_backpack(command(4)?, second, stone()))
            .await?;
        assert_eq!(two.destination, at(backpack, 2));

        // Newest first, no renumbering; nothing is left on Ground.
        let read = harness
            .root
            .read_character_backpack(&authority, fence()?.character_id)
            .await
            .map_err(debug)?
            .ok_or("backpack")?;
        assert_eq!(read.backpack.item_instance_id, backpack);
        let order: Vec<_> = read
            .entries
            .iter()
            .map(|entry| (entry.item.item_instance_id, entry.placement_ordinal))
            .collect();
        assert_eq!(order, vec![(second, 2), (first, 1)]);
        for item in [backpack, first, second] {
            assert!(!harness.on_ground(item).await?);
        }

        // Inventory-only TRANSFERs do not advance CharacterRevision.
        assert_eq!(harness.character_revision().await?, "1");
        assert_eq!(harness.count("game_character_xp_receipts").await?, 0);

        // The audit event is the registered TRANSFER event of the receipt.
        let row = sqlx::query(
            "SELECT a.envelope, a.envelope_sha256 = r.envelope_sha256 AS digest_ok \
               FROM game_item_audit_outbox a \
               JOIN game_item_transfer_receipts r USING (event_id, transaction_id) \
              WHERE a.transaction_id = encode($1,'hex')::uuid",
        )
        .bind(two.transaction_id.as_slice())
        .fetch_one(&harness.pool)
        .await?;
        assert!(row.try_get::<bool, _>("digest_ok")?);
        let (envelope, payload) =
            audit::decode_transfer_envelope(&row.try_get::<Vec<u8>, _>("envelope")?)
                .map_err(debug)?;
        assert_eq!(envelope.command_id, Some(4));
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
            .freeze_item_transfer(
                &authority,
                &harness.node,
                fence()?,
                to_backpack(command(4)?, second, stone()),
            )
            .await
            .map_err(debug)?;
        assert_eq!(replay.transaction_id(), &two.transaction_id);
        match harness
            .root
            .commit_item_transfer(&authority, &harness.node, fence()?, &mut replay)
            .await
            .map_err(debug)?
        {
            ItemTransferOutcome::AlreadyCommitted(result) => assert_eq!(result, two),
            other => return Err(format!("expected the retained result, got {other:?}").into()),
        }
        assert_eq!(
            harness
                .root
                .reconcile_item_transfer(&authority, &mut replay)
                .await
                .map_err(debug)?,
            Some(two.clone())
        );
        // DUR03-RL-08: the third unit is the last; the fourth is rejected.
        assert_eq!(replay.work_units_used(), 3);
        assert!(matches!(
            harness
                .root
                .reconcile_item_transfer(&authority, &mut replay)
                .await,
            Err(ItemTransferError::CapacityExceeded)
        ));
        // Same command, different intent: integrity conflict.
        assert!(matches!(
            harness
                .root
                .freeze_item_transfer(
                    &authority,
                    &harness.node,
                    fence()?,
                    to_backpack(command(4)?, first, stone()),
                )
                .await,
            Err(ItemTransferError::ConflictingCause)
        ));
        assert_eq!(harness.count("game_item_transfer_receipts").await?, 3);
        assert_eq!(harness.count("game_item_audit_outbox").await?, 6);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn merge_shapes_conserve_units_and_a_full_backpack_refuses() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "merge").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let coin = || facts(COIN, STACKABLE);
        let backpack = harness.mint(&authority, BACKPACK, 1).await?;
        harness
            .committed(&authority, to_slot(command(1)?, backpack, backpack_facts()))
            .await?;
        let mut next = 2_u64;
        let mut pick = |item: [u8; 16], facts: ItemDefinitionFacts| {
            next += 1;
            to_backpack(command(next).expect("command"), item, facts)
        };

        let a1 = harness.mint(&authority, COIN, 60).await?;
        let placed = harness.committed(&authority, pick(a1, coin())).await?;
        assert_eq!(placed.destination, at(backpack, 1));

        // Full merge: A2 (30) into A1 (60); A2 retires with no location.
        let a2 = harness.mint(&authority, COIN, 30).await?;
        let merged = harness.committed(&authority, pick(a2, coin())).await?;
        assert_eq!(merged.shape, TransferShape::FullMerge);
        assert_eq!(merged.source_quantity_after, 0);
        let receiver = merged.receiver.ok_or("receiver")?;
        assert_eq!(
            (
                receiver.item_instance_id,
                receiver.quantity_before,
                receiver.quantity_after
            ),
            (a1, 60, 90)
        );
        assert_eq!(harness.item_state(a2).await?, (0, 2));
        assert!(!harness.on_ground(a2).await?);
        assert_eq!(harness.count("game_item_container_entries").await?, 1);

        // Top-up: A3 (25) fills A1 to 100 and keeps 15 in a new entry.
        let a3 = harness.mint(&authority, COIN, 25).await?;
        let topped = harness.committed(&authority, pick(a3, coin())).await?;
        assert_eq!(topped.shape, TransferShape::TopUp);
        assert_eq!(topped.destination, at(backpack, 2));
        assert_eq!(topped.source_quantity_after, 15);
        assert_eq!(harness.item_state(a1).await?, (100, 1));
        assert_eq!(harness.item_state(a3).await?, (15, 1));

        // The receiver is the newest compatible stack with room: A3.
        let a4 = harness.mint(&authority, COIN, 50).await?;
        let into_newest = harness.committed(&authority, pick(a4, coin())).await?;
        assert_eq!(
            into_newest
                .receiver
                .map(|receiver| receiver.item_instance_id),
            Some(a3)
        );
        assert_eq!(harness.item_state(a3).await?, (65, 1));

        // Exact units are conserved: 60 + 30 + 25 + 50 = 100 + 65.
        let live: i64 = sqlx::query_scalar(
            "SELECT sum(quantity)::bigint FROM game_item_instances \
              WHERE definition_production_key = $1",
        )
        .bind(COIN)
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(live, 165);

        // Fill the backpack: 2 coin entries + 18 stones = 20 entries (max).
        for _ in 0..18 {
            let stone = harness.mint(&authority, STONE, 1).await?;
            harness
                .committed(
                    &authority,
                    pick(stone, facts(STONE, ItemStackClass::NonStackable)),
                )
                .await?;
        }
        assert_eq!(harness.count("game_item_container_entries").await?, 20);

        // max+1: the 21st entry is refused and the stone stays on Ground.
        let extra = harness.mint(&authority, STONE, 1).await?;
        let before = harness.footprint().await?;
        refused(
            harness
                .transfer(
                    &authority,
                    fence()?,
                    pick(extra, facts(STONE, ItemStackClass::NonStackable)),
                )
                .await,
            ItemTransferRefusal::MainBackpackFull,
        )?;
        assert!(harness.on_ground(extra).await?);
        // A top-up needing a free entry is refused with nothing moved.
        let big = harness.mint(&authority, COIN, 40).await?;
        let before_big = harness.footprint().await?;
        assert_eq!(before_big[2], before[2] + 1);
        refused(
            harness
                .transfer(&authority, fence()?, pick(big, coin()))
                .await,
            ItemTransferRefusal::MainBackpackFull,
        )?;
        assert_eq!(harness.footprint().await?, before_big);
        assert_eq!(harness.item_state(a3).await?, (65, 1));
        // A full merge still fits a full backpack: 65 + 35 = 100.
        let small = harness.mint(&authority, COIN, 35).await?;
        let last = harness.committed(&authority, pick(small, coin())).await?;
        assert_eq!(last.shape, TransferShape::FullMerge);
        assert_eq!(harness.item_state(a3).await?, (100, 1));

        // GAMEITEM01-REACHABLE-ITEMS: the backpack and its 20 entries.
        let read = harness
            .root
            .read_character_backpack(&authority, fence()?.character_id)
            .await
            .map_err(debug)?
            .ok_or("backpack")?;
        assert_eq!(read.entries.len() + 1, 21);
        assert_eq!(harness.character_revision().await?, "1");

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn refusals_write_nothing() -> TestResult {
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
        let bag = harness.mint(&authority, "fixture:b3.bag", 1).await?;
        let backpack = harness.mint(&authority, BACKPACK, 1).await?;
        let spare = harness.mint(&authority, BACKPACK, 1).await?;
        let coin = harness.mint(&authority, COIN, 10).await?;
        let heap = harness.mint(&authority, COIN, 101).await?;

        // A container without a container-slot equip pattern.
        let before = harness.footprint().await?;
        let mut no_pattern = facts("fixture:b3.bag", ItemStackClass::NonStackable);
        no_pattern.container_capacity = Some(8);
        refused(
            harness
                .transfer(&authority, fence()?, to_slot(command(1)?, bag, no_pattern))
                .await,
            ItemTransferRefusal::NotContainerSlotEquippable,
        )?;
        assert_eq!(harness.footprint().await?, before);

        harness
            .committed(&authority, to_slot(command(2)?, backpack, backpack_facts()))
            .await?;
        let before = harness.footprint().await?;
        let cases: Vec<(ItemTransferRequest, ItemTransferRefusal)> = vec![
            (
                to_slot(command(3)?, spare, backpack_facts()),
                ItemTransferRefusal::ContainerSlotOccupied,
            ),
            (
                to_backpack(command(4)?, coin, facts(COIN, ItemStackClass::Unknown)),
                ItemTransferRefusal::UnknownStackClass,
            ),
            (
                to_backpack(
                    command(5)?,
                    coin,
                    facts(
                        COIN,
                        ItemStackClass::Stackable {
                            proven_maximum: Some(101),
                        },
                    ),
                ),
                ItemTransferRefusal::UnsupportedStackMaximum,
            ),
            (
                to_backpack(command(6)?, heap, facts(COIN, STACKABLE)),
                ItemTransferRefusal::QuantityAboveStackMaximum,
            ),
            (
                to_backpack(command(7)?, coin, facts(STONE, STACKABLE)),
                ItemTransferRefusal::DefinitionMismatch,
            ),
            (
                to_backpack(command(8)?, backpack, backpack_facts()),
                ItemTransferRefusal::SourceNotOnGround,
            ),
        ];
        for (request, expected) in cases {
            refused(
                harness.transfer(&authority, fence()?, request).await,
                expected,
            )?;
            assert_eq!(harness.footprint().await?, before, "{expected:?}");
        }
        let mut wrong_backpack = to_backpack(command(9)?, coin, facts(COIN, STACKABLE));
        if let Some(backpack) = wrong_backpack.backpack.as_mut() {
            backpack.container_capacity = Some(21);
        }
        refused(
            harness.transfer(&authority, fence()?, wrong_backpack).await,
            ItemTransferRefusal::UnsupportedContainerCapacity,
        )?;
        assert_eq!(harness.footprint().await?, before);

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
        let backpack = harness.mint(&authority, BACKPACK, 1).await?;
        harness
            .committed(&authority, to_slot(command(1)?, backpack, backpack_facts()))
            .await?;
        let stone = || facts(STONE, ItemStackClass::NonStackable);
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
            ("scope that does not own the Ground", |f| {
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
            let item = harness.mint(&authority, STONE, 1).await?;
            let mut stale = fence()?;
            mutate(&mut stale);
            // Freeze boundary: rejected, no reservation.
            next += 1;
            let reservations = harness.count("game_item_transfer_reservations").await?;
            rejected(
                harness
                    .root
                    .freeze_item_transfer(
                        &authority,
                        &harness.node,
                        stale,
                        to_backpack(command(next)?, item, stone()),
                    )
                    .await,
                label,
            )?;
            assert_eq!(
                harness.count("game_item_transfer_reservations").await?,
                reservations,
                "{label}"
            );
            // Commit boundary: a valid freeze, then a stale commit moves nothing.
            next += 1;
            let mut candidate = harness
                .root
                .freeze_item_transfer(
                    &authority,
                    &harness.node,
                    fence()?,
                    to_backpack(command(next)?, item, stone()),
                )
                .await
                .map_err(debug)?;
            rejected(
                harness
                    .root
                    .commit_item_transfer(&authority, &harness.node, stale, &mut candidate)
                    .await,
                label,
            )?;
            assert!(harness.on_ground(item).await?, "{label}");
        }

        // The CommandRef of another GameSession under a valid fence.
        let item = harness.mint(&authority, STONE, 1).await?;
        let mut foreign = to_backpack(command(90)?, item, stone());
        foreign.command = CommandRef::new(
            GameSessionId::decode(&id(51)).map_err(debug)?,
            CommandId::new(90).map_err(debug)?,
        );
        rejected(
            harness.transfer(&authority, fence()?, foreign).await,
            "foreign CommandRef",
        )?;

        // Runtime assignment held by another node incarnation.
        next += 1;
        rejected(
            harness
                .root
                .freeze_item_transfer(
                    &authority,
                    &other_node,
                    fence()?,
                    to_backpack(command(next)?, item, stone()),
                )
                .await,
            "assignment holder",
        )?;
        next += 1;
        let mut candidate = harness
            .root
            .freeze_item_transfer(
                &authority,
                &harness.node,
                fence()?,
                to_backpack(command(next)?, item, stone()),
            )
            .await
            .map_err(debug)?;
        rejected(
            harness
                .root
                .commit_item_transfer(&authority, &other_node, fence()?, &mut candidate)
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
                .freeze_item_transfer(
                    &authority,
                    &harness.node,
                    fence()?,
                    to_backpack(command(next)?, item, stone()),
                )
                .await
                .map_err(debug)?;
            harness.tamper(&break_fact).await?;
            next += 1;
            rejected(
                harness
                    .root
                    .freeze_item_transfer(
                        &authority,
                        &harness.node,
                        fence()?,
                        to_backpack(command(next)?, item, stone()),
                    )
                    .await,
                label,
            )?;
            rejected(
                harness
                    .root
                    .commit_item_transfer(&authority, &harness.node, fence()?, &mut candidate)
                    .await,
                label,
            )?;
            harness.tamper(&restore).await?;
            assert!(harness.on_ground(item).await?, "{label}");
        }

        // DUR-03 §31: a still-pending CommandRef commits after an eligible
        // same-GameSession reconnect with the current connection generation;
        // the predecessor generation cannot.
        next += 1;
        let pending = command(next)?;
        let mut candidate = harness
            .root
            .freeze_item_transfer(
                &authority,
                &harness.node,
                fence()?,
                to_backpack(pending, item, stone()),
            )
            .await
            .map_err(debug)?;
        harness.tamper(&session("current_generation = 2")).await?;
        let mut reconnected = fence()?;
        reconnected.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        rejected(
            harness
                .root
                .commit_item_transfer(&authority, &harness.node, fence()?, &mut candidate)
                .await,
            "predecessor connection generation",
        )?;
        let continued = match harness
            .root
            .commit_item_transfer(&authority, &harness.node, reconnected, &mut candidate)
            .await
            .map_err(debug)?
        {
            ItemTransferOutcome::Committed(result) => result,
            other => return Err(format!("expected commit, got {other:?}").into()),
        };
        assert!(!harness.on_ground(item).await?);

        // Ended node incarnation: freeze and commit both rejected.
        let late = harness.mint(&authority, STONE, 1).await?;
        next += 1;
        let mut candidate = harness
            .root
            .freeze_item_transfer(
                &authority,
                &harness.node,
                reconnected,
                to_backpack(command(next)?, late, stone()),
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
                .commit_item_transfer(&authority, &harness.node, reconnected, &mut candidate)
                .await,
            "ended node incarnation",
        )?;
        next += 1;
        rejected(
            harness
                .root
                .freeze_item_transfer(
                    &authority,
                    &harness.node,
                    reconnected,
                    to_backpack(command(next)?, late, stone()),
                )
                .await,
            "ended node incarnation",
        )?;
        assert!(harness.on_ground(late).await?);

        // A retry after commit replays under the current recovery fence; a
        // stale recovery fence returns no outcome at any boundary.
        let mut replay = harness
            .root
            .freeze_item_transfer(
                &authority,
                &harness.node,
                reconnected,
                to_backpack(pending, item, stone()),
            )
            .await
            .map_err(debug)?;
        assert_eq!(
            harness
                .root
                .reconcile_item_transfer(&authority, &mut replay)
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
                .freeze_item_transfer(
                    &authority,
                    &harness.node,
                    reconnected,
                    to_backpack(pending, item, stone()),
                )
                .await,
            Err(ItemTransferError::Unavailable(_))
        ));
        next += 1;
        assert!(matches!(
            harness
                .root
                .freeze_item_transfer(
                    &authority,
                    &harness.node,
                    reconnected,
                    to_backpack(command(next)?, late, stone()),
                )
                .await,
            Err(ItemTransferError::Unavailable(_))
        ));
        assert_eq!(harness.count("game_item_transfer_receipts").await?, 2);
        assert_eq!(harness.character_revision().await?, "1");

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn database_rejects_unproven_item_and_location_changes() -> TestResult {
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
        let backpack = harness.mint(&authority, BACKPACK, 1).await?;
        harness
            .committed(&authority, to_slot(command(1)?, backpack, backpack_facts()))
            .await?;
        let entry = harness.mint(&authority, STONE, 1).await?;
        harness
            .committed(
                &authority,
                to_backpack(command(2)?, entry, facts(STONE, ItemStackClass::NonStackable)),
            )
            .await?;
        let loose = harness.mint(&authority, COIN, 5).await?;
        let uuid = uuid_text;
        let attempts = [
            // A quantity change without a new transaction.
            format!(
                "UPDATE game_item_instances SET quantity = 6 WHERE item_instance_id = '{}'",
                uuid(loose)
            ),
            // A change claiming a transaction that has no receipt.
            format!(
                "UPDATE game_item_instances SET quantity = 6, \
                   last_transaction_id = '{}' WHERE item_instance_id = '{}'",
                uuid(id(99)),
                uuid(loose)
            ),
            // Ground custody ending without a TRANSFER.
            format!(
                "DELETE FROM game_item_ground_locations WHERE item_instance_id = '{}'",
                uuid(loose)
            ),
            // GAMEITEM01-PLACEMENT-DEPTH max+1: an entry inside an entry.
            format!(
                "INSERT INTO game_item_container_entries VALUES ('{}', '{}', '{}', '{}', 1, '{}')",
                uuid(loose),
                uuid(id(WORLD)),
                uuid(id(CHARACTER)),
                uuid(entry),
                uuid(id(98))
            ),
            // Repair generation 2, finding 1: Ground custody regained after a
            // TRANSFER already moved the item into the backpack.
            format!(
                "INSERT INTO game_item_ground_locations VALUES ('{}', '{}', '{}', 1, \
                   decode('01','hex'), decode('01','hex'), 'map-1', 'content-1', \
                   decode('01','hex'))",
                uuid(entry),
                uuid(id(WORLD)),
                uuid(id(CHANNEL))
            ),
            // Committed locations and receipts are immutable.
            "DELETE FROM game_item_container_entries".into(),
            "UPDATE game_item_transfer_receipts SET committed_at = 0".into(),
            "TRUNCATE game_item_container_slots CASCADE".into(),
            "DELETE FROM game_item_instances".into(),
        ];
        for statement in attempts {
            let mut tx = harness.pool.begin().await?;
            let applied = async {
                sqlx::query(sqlx::AssertSqlSafe(statement.clone()))
                    .execute(&mut *tx)
                    .await?;
                tx.commit().await
            }
            .await;
            assert!(applied.is_err(), "accepted: {statement}");
        }
        assert_eq!(harness.item_state(loose).await?, (5, 1));
        assert!(harness.on_ground(loose).await?);
        assert_eq!(harness.count("game_item_container_entries").await?, 1);

        // Least privilege: the runtime role may only perform TRANSFER writes, and (0023) delete
        // a backpack entry, which commits only as a proven fee whole burn.
        let grants: Vec<bool> = sqlx::query_scalar(
            "SELECT unnest(ARRAY[\
               has_table_privilege('oteryn_game_runtime','game_item_ground_locations','DELETE'),\
               has_column_privilege('oteryn_game_runtime','game_item_instances','quantity','UPDATE'),\
               has_column_privilege('oteryn_game_runtime','game_item_instances','lifecycle','UPDATE'),\
               has_column_privilege('oteryn_game_runtime','game_item_instances',\
                 'last_transaction_id','UPDATE'),\
               NOT has_column_privilege('oteryn_game_runtime','game_item_instances',\
                 'definition_production_key','UPDATE'),\
               NOT has_table_privilege('oteryn_game_runtime','game_item_instances','DELETE'),\
               has_table_privilege('oteryn_game_runtime','game_item_container_slots','INSERT'),\
               has_table_privilege('oteryn_game_runtime','game_item_container_entries','INSERT'),\
               has_table_privilege('oteryn_game_runtime','game_item_container_entries','DELETE'),\
               NOT has_table_privilege('oteryn_game_runtime','game_item_container_entries','UPDATE'),\
               has_table_privilege('oteryn_game_runtime','game_item_transfer_receipts','INSERT'),\
               NOT has_table_privilege('oteryn_game_runtime','game_item_transfer_receipts','UPDATE'),\
               has_column_privilege('oteryn_game_runtime','game_item_transfer_reservations',\
                 'work_units_used','UPDATE'),\
               NOT has_column_privilege('oteryn_game_runtime','game_item_transfer_reservations',\
                 'transaction_id','UPDATE'),\
               has_table_privilege('oteryn_game_control','game_item_transfer_receipts','SELECT'),\
               NOT has_table_privilege('oteryn_game_control','game_item_transfer_receipts','INSERT'),\
               has_table_privilege('oteryn_game_runtime','game_item_transfer_quantity_evidence',\
                 'SELECT'),\
               NOT has_table_privilege('oteryn_game_runtime',\
                 'game_item_transfer_quantity_evidence','INSERT'),\
               NOT has_table_privilege('oteryn_game_runtime',\
                 'game_item_transfer_quantity_evidence','UPDATE'),\
               NOT has_table_privilege('oteryn_game_runtime',\
                 'game_item_transfer_quantity_evidence','DELETE'),\
               has_table_privilege('oteryn_game_control','game_item_transfer_quantity_evidence',\
                 'SELECT'),\
               has_table_privilege('oteryn_game_runtime','game_item_ground_removal_evidence',\
                 'SELECT'),\
               NOT has_table_privilege('oteryn_game_runtime','game_item_ground_removal_evidence',\
                 'INSERT'),\
               has_table_privilege('oteryn_game_control','game_item_ground_removal_evidence',\
                 'SELECT')])",
        )
        .fetch_all(&harness.pool)
        .await?;
        assert_eq!(grants.len(), 24);
        assert!(grants.iter().all(|granted| *granted), "{grants:?}");

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// P1 (comment 4126035286): a runtime-role SQL transaction that fabricates
/// the receipt's own `source_quantity_before` -- while the real source item
/// truly holds only 2 units -- must be rejected at COMMIT, not merely by the
/// receipt's self-consistent arithmetic. The deferred conservation guard
/// binds `source_quantity_before`/`receiver_quantity_before` to the guarded
/// `game_item_transfer_quantity_evidence` captured from the real OLD.quantity
/// of every item row a TRANSFER touches, so the receipt can no longer claim a
/// fictitious before-quantity for either participant.
#[test]
fn full_merge_cannot_claim_a_fictitious_source_quantity_before() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "qtyevidence").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let coin = || facts(COIN, STACKABLE);
        let backpack = harness.mint(&authority, BACKPACK, 1).await?;
        harness
            .committed(&authority, to_slot(command(1)?, backpack, backpack_facts()))
            .await?;
        // The receiver genuinely holds 1 unit in the backpack.
        let receiver = harness.mint(&authority, COIN, 1).await?;
        harness
            .committed(&authority, to_backpack(command(2)?, receiver, coin()))
            .await?;
        // The source genuinely holds only 2 units, still on Ground.
        let source = harness.mint(&authority, COIN, 2).await?;

        let uuid = uuid_text;
        let session = uuid(id(SESSION));
        let character = uuid(id(CHARACTER));
        let world = uuid(id(WORLD));
        let channel = uuid(id(CHANNEL));
        let source_text = uuid(source);
        let receiver_text = uuid(receiver);
        let tx_id = uuid(id(220));
        let event_id = uuid(id(221));
        // A forged runtime-role commit: the item rows genuinely end up with
        // the claimed *_after values, but `source_quantity_before` (99) is a
        // fiction -- the source truly held 2. Every statement below is one a
        // real TRANSFER commit would issue; only the deferred conservation
        // guard, bound to the captured evidence, can catch the lie.
        let statements = [
            format!(
                "INSERT INTO game_item_transfer_reservations VALUES \
                 ('{session}', 99, '{character}', '{world}', '{channel}', '{source_text}', 2, \
                   decode(repeat('ab',33),'hex'), '{tx_id}', '{event_id}', 1000, 1, 900)"
            ),
            format!(
                "INSERT INTO game_item_audit_outbox VALUES \
                 ('{event_id}', '{tx_id}', 1, 1, 2, 1, \
                   'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1', '{source_text}', 1000, \
                   7776001000, decode(repeat('ab',16),'hex'), \
                   sha256(decode(repeat('ab',16),'hex')), 1, NULL)"
            ),
            format!(
                "UPDATE game_item_instances SET quantity = 0, lifecycle = 2, \
                   last_transaction_id = '{tx_id}' WHERE item_instance_id = '{source_text}'"
            ),
            format!(
                "DELETE FROM game_item_ground_locations WHERE item_instance_id = '{source_text}'"
            ),
            format!(
                "UPDATE game_item_instances SET quantity = 100, last_transaction_id = '{tx_id}' \
                   WHERE item_instance_id = '{receiver_text}'"
            ),
            format!(
                "INSERT INTO game_item_transfer_receipts \
                   (game_session_id, command_id, character_id, intent_binding, transaction_id, \
                    event_id, shape, source_item_instance_id, source_quantity_before, \
                    source_quantity_after, receiver_item_instance_id, receiver_quantity_before, \
                    receiver_quantity_after, destination_parent_item_instance_id, \
                    destination_ordinal, occurred_at, envelope_sha256, committed_at) \
                 VALUES \
                 ('{session}', 99, '{character}', decode(repeat('ab',33),'hex'), '{tx_id}', \
                   '{event_id}', 3, '{source_text}', 99, 0, '{receiver_text}', 1, 100, NULL, \
                   NULL, 1000, sha256(decode(repeat('ab',16),'hex')), 1000)"
            ),
        ];
        let mut tx = harness.pool.begin().await?;
        let applied: TestResult = async {
            for statement in &statements {
                sqlx::query(sqlx::AssertSqlSafe(statement.clone()))
                    .execute(&mut *tx)
                    .await?;
            }
            tx.commit().await?;
            Ok(())
        }
        .await;
        assert!(
            applied.is_err(),
            "a fictitious 99-unit source_quantity_before was accepted"
        );
        // Nothing moved: the real quantities and Ground custody are untouched.
        assert_eq!(harness.item_state(source).await?, (2, 1));
        assert!(harness.on_ground(source).await?);
        assert_eq!(harness.item_state(receiver).await?, (1, 1));

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// Repair generation 2 (Codex review 5343738115), finding 2: a placement
/// insert into `game_item_container_entries` reused a historical receipt's
/// TransactionId, checked only against `source_item_instance_id` and
/// `character_id`, never against the receipt's own shape. A full merge
/// (shape 3) retires its source with no entry of its own; reusing that
/// receipt's TransactionId to forge an entry for the now-retired source must
/// be rejected.
#[test]
fn container_entry_cannot_reuse_a_full_merge_receipts_transaction_id() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "mergereuse").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let coin = || facts(COIN, STACKABLE);
        let backpack = harness.mint(&authority, BACKPACK, 1).await?;
        harness
            .committed(&authority, to_slot(command(1)?, backpack, backpack_facts()))
            .await?;
        let receiver = harness.mint(&authority, COIN, 3).await?;
        harness
            .committed(&authority, to_backpack(command(2)?, receiver, coin()))
            .await?;
        let source = harness.mint(&authority, COIN, 2).await?;
        let merged = harness
            .committed(&authority, to_backpack(command(3)?, source, coin()))
            .await?;
        assert_eq!(merged.shape, TransferShape::FullMerge);
        assert_eq!(harness.item_state(source).await?, (0, 2));

        let uuid = uuid_text;
        let statement = format!(
            "INSERT INTO game_item_container_entries VALUES ('{}', '{}', '{}', '{}', 5, '{}')",
            uuid(source),
            uuid(id(WORLD)),
            uuid(id(CHARACTER)),
            uuid(backpack),
            uuid(merged.transaction_id)
        );
        let mut tx = harness.pool.begin().await?;
        let applied: TestResult = async {
            sqlx::query(sqlx::AssertSqlSafe(statement))
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            Ok(())
        }
        .await;
        assert!(
            applied.is_err(),
            "a full-merge receipt's TransactionId placed its retired source"
        );
        assert_eq!(harness.item_state(source).await?, (0, 2));
        assert_eq!(harness.count("game_item_container_entries").await?, 1);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// Repair generation 2, finding 3: the receipt guard checked the
/// reservation's World against the source item but never its Channel
/// against the Ground row the TRANSFER actually removed (already deleted by
/// the time the deferred guard runs). A forged commit whose reservation
/// claims a different Channel than the item's real Ground row -- every other
/// field genuine, captured by the real triggers -- must be rejected.
#[test]
fn transfer_channel_must_match_the_removed_ground_location() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "channelcheck").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let source = harness.mint(&authority, STONE, 1).await?;
        assert!(harness.on_ground(source).await?);

        let uuid = uuid_text;
        let character = uuid(id(CHARACTER));
        let world = uuid(id(WORLD));
        let source_text = uuid(source);
        let tx_id = uuid(id(230));
        let event_id = uuid(id(231));
        // The real Ground row is Channel 43; the reservation lies about it.
        let wrong_channel = uuid(id(199));
        let statements = [
            format!(
                "INSERT INTO game_item_transfer_reservations VALUES \
                 ('{character}', 1, '{character}', '{world}', '{wrong_channel}', \
                   '{source_text}', 1, decode(repeat('ab',33),'hex'), '{tx_id}', '{event_id}', \
                   1000, 1, 900)"
            ),
            format!(
                "INSERT INTO game_item_audit_outbox VALUES \
                 ('{event_id}', '{tx_id}', 1, 1, 2, 1, \
                   'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1', '{source_text}', 1000, \
                   7776001000, decode(repeat('ab',16),'hex'), \
                   sha256(decode(repeat('ab',16),'hex')), 1, NULL)"
            ),
            format!(
                "UPDATE game_item_instances SET last_transaction_id = '{tx_id}' \
                   WHERE item_instance_id = '{source_text}' AND lifecycle = 1 AND quantity = 1"
            ),
            format!(
                "DELETE FROM game_item_ground_locations WHERE item_instance_id = '{source_text}'"
            ),
            format!(
                "INSERT INTO game_item_container_slots VALUES \
                 ('{character}', '{source_text}', '{world}', '{tx_id}')"
            ),
            format!(
                "INSERT INTO game_item_transfer_receipts \
                   (game_session_id, command_id, character_id, intent_binding, transaction_id, \
                    event_id, shape, source_item_instance_id, source_quantity_before, \
                    source_quantity_after, occurred_at, envelope_sha256, committed_at) \
                 VALUES \
                 ('{character}', 1, '{character}', decode(repeat('ab',33),'hex'), '{tx_id}', \
                   '{event_id}', 1, '{source_text}', 1, 1, 1000, \
                   sha256(decode(repeat('ab',16),'hex')), 1000)"
            ),
        ];
        // The reservation's own `game_session_id` reuses `character` above
        // only as a distinct, valid-format UUID; FND-02 CommandRef identity
        // plays no part in this SQL-level guard.
        let mut tx = harness.pool.begin().await?;
        let applied: TestResult = async {
            for statement in &statements {
                sqlx::query(sqlx::AssertSqlSafe(statement.clone()))
                    .execute(&mut *tx)
                    .await?;
            }
            tx.commit().await?;
            Ok(())
        }
        .await;
        assert!(
            applied.is_err(),
            "a reservation Channel that did not match the real Ground row was accepted"
        );
        assert_eq!(harness.item_state(source).await?, (1, 1));
        assert!(harness.on_ground(source).await?);
        assert_eq!(harness.count("game_item_container_slots").await?, 0);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// Codex P1 on #1152, finding 1: the consistency guard proved a TRANSFER's
/// audit event only by TransactionId/EventId/timestamp/envelope hash, and
/// `game_item_audit_outbox` carries the same `event_type_id` (2) for a MINT's
/// own outbox row as for a TRANSFER's, so a forged commit could replay a
/// still-live MINT's row -- transaction_id, event_id, occurred_at and
/// envelope hash all genuinely matching -- as its own audit evidence,
/// without writing a fresh outbox row of its own. The guard now also
/// requires that matched row's `created_xact_id` to be the CURRENT physical
/// transaction's, which a historical, already-committed MINT row can never
/// be.
#[test]
fn transfer_cannot_reuse_the_mint_audit_event() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "mintreuse").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let source = harness.mint(&authority, STONE, 1).await?;
        assert!(harness.on_ground(source).await?);

        // The MINT's own outbox row, read back exactly as the real MINT
        // committed it: same transaction_id, event_id, occurred_at and
        // envelope hash the consistency guard checks for a TRANSFER.
        let row = sqlx::query(
            "SELECT transaction_id::text, event_id::text, occurred_at, \
               encode(envelope_sha256,'hex') \
               FROM game_item_audit_outbox WHERE item_instance_id = encode($1,'hex')::uuid",
        )
        .bind(source.as_slice())
        .fetch_one(&harness.pool)
        .await?;
        let mint_tx_id: String = row.try_get(0)?;
        let mint_event_id: String = row.try_get(1)?;
        let mint_occurred_at: i64 = row.try_get(2)?;
        let mint_envelope_sha256_hex: String = row.try_get(3)?;

        let uuid = uuid_text;
        let character = uuid(id(CHARACTER));
        let world = uuid(id(WORLD));
        let channel = uuid(id(CHANNEL));
        let source_text = uuid(source);
        // Every statement below is one a real container-slot TRANSFER commit
        // would issue, except that no fresh row is ever inserted into
        // `game_item_audit_outbox` -- the reservation and receipt instead
        // reuse the MINT's own transaction_id/event_id/occurred_at/hash.
        let statements = [
            format!(
                "INSERT INTO game_item_transfer_reservations VALUES \
                 ('{character}', 1, '{character}', '{world}', '{channel}', '{source_text}', 1, \
                   decode(repeat('ab',33),'hex'), '{mint_tx_id}', '{mint_event_id}', \
                   {mint_occurred_at}, 1, 0)"
            ),
            format!(
                "UPDATE game_item_instances SET last_transaction_id = '{mint_tx_id}' \
                   WHERE item_instance_id = '{source_text}' AND lifecycle = 1 AND quantity = 1"
            ),
            format!(
                "DELETE FROM game_item_ground_locations WHERE item_instance_id = '{source_text}'"
            ),
            format!(
                "INSERT INTO game_item_container_slots VALUES \
                 ('{character}', '{source_text}', '{world}', '{mint_tx_id}')"
            ),
            format!(
                "INSERT INTO game_item_transfer_receipts \
                   (game_session_id, command_id, character_id, intent_binding, transaction_id, \
                    event_id, shape, source_item_instance_id, source_quantity_before, \
                    source_quantity_after, occurred_at, envelope_sha256, committed_at) \
                 VALUES \
                 ('{character}', 1, '{character}', decode(repeat('ab',33),'hex'), '{mint_tx_id}', \
                   '{mint_event_id}', 1, '{source_text}', 1, 1, {mint_occurred_at}, \
                   decode('{mint_envelope_sha256_hex}','hex'), {mint_occurred_at})"
            ),
        ];
        let mut tx = harness.pool.begin().await?;
        let applied: TestResult = async {
            for statement in &statements {
                sqlx::query(sqlx::AssertSqlSafe(statement.clone()))
                    .execute(&mut *tx)
                    .await?;
            }
            tx.commit().await?;
            Ok(())
        }
        .await;
        assert!(
            applied.is_err(),
            "a historical MINT outbox row was reused as a TRANSFER's own audit evidence"
        );
        // Nothing moved: the item is still on Ground, untouched.
        assert_eq!(harness.item_state(source).await?, (1, 1));
        assert!(harness.on_ground(source).await?);
        assert_eq!(harness.count("game_item_container_slots").await?, 0);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// Codex P1 on #1152, finding 2: the consistency guard bound a TRANSFER to
/// the source item's World but never checked the destination Character's own
/// root World against it, so a forged commit could place a World-A Ground
/// item into a Character rooted in a different World B. The guard now also
/// requires `game_character_roots.world_id` of the destination Character to
/// equal the source item's World. A forged commit with every other fact
/// genuine -- a fresh reservation and its own fresh audit row, the real
/// removed Ground row's World and Channel, the real captured quantity
/// evidence -- but a destination Character rooted in another World must be
/// rejected.
#[test]
fn transfer_rejects_a_destination_character_in_another_world() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "worldcheck").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        // A second Character root, genuinely rooted in a different World,
        // seeded the same way `seed_character` seeds the first one, reusing
        // the account guard it already installed.
        let uuid = uuid_text;
        let other_character = uuid(id(52));
        let other_world = uuid(id(53));
        let account = uuid(id(40));
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "INSERT INTO game_character_roots VALUES \
             ('{other_character}', '{account}', '{other_world}', 1, 1, \
               'profile-1', 'ruleset-1', 'content-1', 'starter-1', 'Other Hero')"
        )))
        .execute(&harness.pool)
        .await?;

        let source = harness.mint(&authority, STONE, 1).await?;
        assert!(harness.on_ground(source).await?);

        let world = uuid(id(WORLD));
        let channel = uuid(id(CHANNEL));
        let source_text = uuid(source);
        let tx_id = uuid(id(60));
        let event_id = uuid(id(61));
        // Every statement below is one a real container-slot TRANSFER commit
        // would issue -- fresh reservation, fresh audit row of this same
        // physical transaction, the item's real World and Channel -- except
        // the destination Character (`other_character`) is rooted in
        // `other_world`, not the source item's real World.
        let statements = [
            format!(
                "INSERT INTO game_item_transfer_reservations VALUES \
                 ('{other_character}', 1, '{other_character}', '{world}', '{channel}', \
                   '{source_text}', 1, decode(repeat('ab',33),'hex'), '{tx_id}', '{event_id}', \
                   1000, 1, 900)"
            ),
            format!(
                "INSERT INTO game_item_audit_outbox VALUES \
                 ('{event_id}', '{tx_id}', 1, 1, 2, 1, \
                   'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1', '{source_text}', 1000, \
                   7776001000, decode(repeat('ab',16),'hex'), \
                   sha256(decode(repeat('ab',16),'hex')), 1, NULL)"
            ),
            format!(
                "UPDATE game_item_instances SET last_transaction_id = '{tx_id}' \
                   WHERE item_instance_id = '{source_text}' AND lifecycle = 1 AND quantity = 1"
            ),
            format!(
                "DELETE FROM game_item_ground_locations WHERE item_instance_id = '{source_text}'"
            ),
            format!(
                "INSERT INTO game_item_container_slots VALUES \
                 ('{other_character}', '{source_text}', '{world}', '{tx_id}')"
            ),
            format!(
                "INSERT INTO game_item_transfer_receipts \
                   (game_session_id, command_id, character_id, intent_binding, transaction_id, \
                    event_id, shape, source_item_instance_id, source_quantity_before, \
                    source_quantity_after, occurred_at, envelope_sha256, committed_at) \
                 VALUES \
                 ('{other_character}', 1, '{other_character}', decode(repeat('ab',33),'hex'), \
                   '{tx_id}', '{event_id}', 1, '{source_text}', 1, 1, 1000, \
                   sha256(decode(repeat('ab',16),'hex')), 1000)"
            ),
        ];
        let mut tx = harness.pool.begin().await?;
        let applied: TestResult = async {
            for statement in &statements {
                sqlx::query(sqlx::AssertSqlSafe(statement.clone()))
                    .execute(&mut *tx)
                    .await?;
            }
            tx.commit().await?;
            Ok(())
        }
        .await;
        assert!(
            applied.is_err(),
            "a destination Character rooted in another World was accepted"
        );
        // Nothing moved: the item is still on Ground, untouched.
        assert_eq!(harness.item_state(source).await?, (1, 1));
        assert!(harness.on_ground(source).await?);
        assert_eq!(harness.count("game_item_container_slots").await?, 0);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// Raw-SQL statements of one forged shape-2 (NewEntry) TRANSFER commit, the
/// exact sequence `apply_transfer` issues, for the two-connection capacity
/// race below (repair generation 2, finding 4).
#[allow(clippy::too_many_arguments)]
fn forge_new_entry_statements(
    character: &str,
    world: &str,
    source: [u8; 16],
    parent: [u8; 16],
    ordinal: u64,
    command_id: u64,
    tx_id: [u8; 16],
    event_id: [u8; 16],
) -> Vec<String> {
    let uuid = uuid_text;
    let source_text = uuid(source);
    let tx = uuid(tx_id);
    let ev = uuid(event_id);
    let channel = uuid(id(CHANNEL));
    let parent_text = uuid(parent);
    vec![
        format!(
            "INSERT INTO game_item_transfer_reservations VALUES \
             ('{character}', {command_id}, '{character}', '{world}', '{channel}', \
               '{source_text}', 2, decode(repeat('ab',33),'hex'), '{tx}', '{ev}', 1000, 1, 900)"
        ),
        format!(
            "INSERT INTO game_item_audit_outbox VALUES \
             ('{ev}', '{tx}', 1, 1, 2, 1, 'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1', \
               '{source_text}', 1000, 7776001000, decode(repeat('ab',16),'hex'), \
               sha256(decode(repeat('ab',16),'hex')), 1, NULL)"
        ),
        format!(
            "UPDATE game_item_instances SET last_transaction_id = '{tx}' \
               WHERE item_instance_id = '{source_text}' AND lifecycle = 1 AND quantity = 1"
        ),
        format!("DELETE FROM game_item_ground_locations WHERE item_instance_id = '{source_text}'"),
        format!(
            "INSERT INTO game_item_container_entries VALUES \
             ('{source_text}', '{world}', '{character}', '{parent_text}', {ordinal}, '{tx}')"
        ),
        format!(
            "INSERT INTO game_item_transfer_receipts \
               (game_session_id, command_id, character_id, intent_binding, transaction_id, \
                event_id, shape, source_item_instance_id, source_quantity_before, \
                source_quantity_after, destination_parent_item_instance_id, \
                destination_ordinal, occurred_at, envelope_sha256, committed_at) \
             VALUES \
             ('{character}', {command_id}, '{character}', decode(repeat('ab',33),'hex'), '{tx}', \
               '{ev}', 2, '{source_text}', 1, 1, '{parent_text}', {ordinal}, 1000, \
               sha256(decode(repeat('ab',16),'hex')), 1000)"
        ),
    ]
}

/// Repair generation 2, finding 4: the GAMEITEM01-CONTAINER-ENTRIES-MAX
/// `count(*) > 20` check raced under concurrent inserts into the same
/// parent, each seeing a snapshot that excluded the other's uncommitted row.
/// Two raw-SQL connections, bypassing the `character_root` lock the normal
/// commit path takes, each forge a complete, otherwise-valid 20th/21st entry
/// for the SAME backpack and commit concurrently: with 19 entries already
/// present, one of the two must be rejected at COMMIT with nothing changed,
/// leaving exactly 20.
#[test]
fn concurrent_container_entry_inserts_enforce_the_capacity_ceiling() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "capacityrace").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let uuid = uuid_text;
        let character = uuid(id(CHARACTER));
        let world = uuid(id(WORLD));
        let backpack = harness.mint(&authority, BACKPACK, 1).await?;
        harness
            .committed(&authority, to_slot(command(1)?, backpack, backpack_facts()))
            .await?;
        // 19 pre-existing entries, seeded directly (bypassing the placement
        // guard, which is not under test here) so the race sits exactly at
        // the ceiling: one more admitted entry is the legal 20th.
        for ordinal in 1..=19_u64 {
            let item = harness.mint(&authority, STONE, 1).await?;
            harness
                .tamper(&format!(
                    "INSERT INTO game_item_container_entries VALUES ('{}', '{world}', \
                       '{character}', '{}', {ordinal}, '{}')",
                    uuid(item),
                    uuid(backpack),
                    uuid(id(100 + ordinal as u8))
                ))
                .await?;
        }
        assert_eq!(harness.count("game_item_container_entries").await?, 19);

        let item_a = harness.mint(&authority, STONE, 1).await?;
        let item_b = harness.mint(&authority, STONE, 1).await?;
        let statements_a = forge_new_entry_statements(
            &character,
            &world,
            item_a,
            backpack,
            20,
            201,
            id(240),
            id(241),
        );
        let statements_b = forge_new_entry_statements(
            &character,
            &world,
            item_b,
            backpack,
            21,
            202,
            id(250),
            id(251),
        );
        let run = |statements: Vec<String>| {
            let pool = harness.pool.clone();
            async move {
                let mut tx = pool.begin().await?;
                for statement in &statements {
                    sqlx::query(sqlx::AssertSqlSafe(statement.clone()))
                        .execute(&mut *tx)
                        .await?;
                }
                tx.commit().await
            }
        };
        let (left, right): (TestResult<_>, TestResult<_>) = join_two(
            async { run(statements_a).await.map_err(|e| debug(e).into()) },
            async { run(statements_b).await.map_err(|e| debug(e).into()) },
        )
        .await;
        assert_ne!(
            left.is_ok(),
            right.is_ok(),
            "the concurrent 20th and 21st entries must not both succeed or both fail: \
             left={left:?} right={right:?}"
        );
        assert_eq!(harness.count("game_item_container_entries").await?, 20);
        let (winner, loser) = if left.is_ok() {
            (item_a, item_b)
        } else {
            (item_b, item_a)
        };
        assert_eq!(harness.item_state(winner).await?, (1, 1));
        assert!(!harness.on_ground(winner).await?);
        assert_eq!(harness.item_state(loser).await?, (1, 1));
        assert!(harness.on_ground(loser).await?);

        drop(authority);
        drop(seal);
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

#[test]
fn concurrent_commits_on_two_roots_serialize() -> TestResult {
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
        let backpack = harness.mint(&first, BACKPACK, 1).await?;
        harness
            .committed(&first, to_slot(command(1)?, backpack, backpack_facts()))
            .await?;

        // The same CommandRef on two roots transfers exactly once.
        let coin = harness.mint(&first, COIN, 10).await?;
        let request = to_backpack(command(2)?, coin, facts(COIN, STACKABLE));
        let mut one = harness
            .root
            .freeze_item_transfer(&first, &harness.node, fence()?, request.clone())
            .await
            .map_err(debug)?;
        let mut two = second_root
            .freeze_item_transfer(&second, &harness.node, fence()?, request)
            .await
            .map_err(debug)?;
        assert_eq!(one.transaction_id(), two.transaction_id());
        let (left, right) = join_two(
            harness
                .root
                .commit_item_transfer(&first, &harness.node, fence()?, &mut one),
            second_root.commit_item_transfer(&second, &harness.node, fence()?, &mut two),
        )
        .await;
        match (left.map_err(debug)?, right.map_err(debug)?) {
            (ItemTransferOutcome::Committed(a), ItemTransferOutcome::AlreadyCommitted(b))
            | (ItemTransferOutcome::AlreadyCommitted(b), ItemTransferOutcome::Committed(a)) => {
                assert_eq!(a, b);
            }
            other => return Err(format!("expected one commit, got {other:?}").into()),
        }
        assert_eq!(harness.count("game_item_transfer_receipts").await?, 2);

        // An XP award and a TRANSFER for one Character serialize on
        // character_root without deadlock; only the XP award advances the
        // revision.
        let stone = harness.mint(&first, STONE, 1).await?;
        let mut candidate = harness
            .root
            .freeze_item_transfer(
                &first,
                &harness.node,
                fence()?,
                to_backpack(
                    command(3)?,
                    stone,
                    facts(STONE, ItemStackClass::NonStackable),
                ),
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
        let (xp, moved) = join_two(
            second_root.commit_character_experience(
                &second,
                &harness.node,
                xp_fence,
                xp_request(70)?,
            ),
            harness
                .root
                .commit_item_transfer(&first, &harness.node, fence()?, &mut candidate),
        )
        .await;
        assert!(matches!(
            xp.map_err(debug)?,
            ExperienceCommitOutcome::Committed(_)
        ));
        assert!(matches!(
            moved.map_err(debug)?,
            ItemTransferOutcome::Committed(_)
        ));
        assert_eq!(harness.character_revision().await?, "2");
        assert!(!harness.on_ground(stone).await?);

        drop(first);
        drop(second);
        drop(seal);
        drop(second_root);
        harness.cleanup().await
    })
}

/// `created_xact_id` must be the actual inserting physical transaction's own
/// id, never a caller-supplied value, so a row committed by one physical
/// transaction can never be stamped with the id of a DIFFERENT, still-open
/// transaction to make it look like that other transaction's own evidence.
/// Connection 1 inserts and commits a `game_item_audit_outbox` row while
/// naming transaction B's own `pg_current_xact_id()` as its
/// `created_xact_id`, even though connection 1 -- not B -- performs and
/// commits that INSERT. The stamping trigger overwrites the supplied value
/// with connection 1's real id (asserted directly below), so when B later
/// tries to complete a full TRANSFER reusing that row as its own audit
/// evidence, the consistency guard's `a.created_xact_id = pg_current_xact_id()`
/// check sees connection 1's real (foreign) id, not B's, and rejects it.
#[test]
fn audit_created_xact_id_cannot_be_forged() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "xactforge").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let source = harness.mint(&authority, STONE, 1).await?;
        assert!(harness.on_ground(source).await?);

        // Transaction B: opened on its own connection and held open while
        // connection 1 does its forged insert below.
        let mut b = harness.pool.begin().await?;
        let b_xid: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
            .fetch_one(&mut *b)
            .await?;

        let uuid = uuid_text;
        let character = uuid(id(CHARACTER));
        let world = uuid(id(WORLD));
        let channel = uuid(id(CHANNEL));
        let source_text = uuid(source);
        let tx_id = uuid(id(90));
        let event_id = uuid(id(91));

        // Connection 1: a real INSERT + COMMIT (autocommit on the pool, a
        // separate connection from B's held transaction), explicitly naming
        // B's xid as `created_xact_id` even though connection 1 performs
        // this INSERT.
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "INSERT INTO game_item_audit_outbox \
               (event_id, transaction_id, transaction_ordinal, transaction_count, \
                event_type_id, schema_revision, retention_profile_id, item_instance_id, \
                occurred_at, expires_at, envelope, envelope_sha256, publication_state, \
                published_at, created_xact_id) \
             VALUES \
               ('{event_id}', '{tx_id}', 1, 1, 2, 1, \
                 'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1', '{source_text}', 1000, \
                 7776001000, decode(repeat('ab',16),'hex'), \
                 sha256(decode(repeat('ab',16),'hex')), 1, NULL, '{b_xid}'::xid8)"
        )))
        .execute(&harness.pool)
        .await?;

        // The stored value is connection 1's OWN inserting transaction, not
        // the forged value it supplied (the "simpler alternative" check).
        let stored_xid: String = sqlx::query_scalar(
            "SELECT created_xact_id::text FROM game_item_audit_outbox \
               WHERE event_id = $1::uuid",
        )
        .bind(&event_id)
        .fetch_one(&harness.pool)
        .await?;
        assert_ne!(
            stored_xid, b_xid,
            "an explicit created_xact_id value was stored instead of the \
             inserting transaction's own id"
        );

        // In B: a complete, otherwise-genuine container-slot TRANSFER commit
        // sequence for `source`, whose only evidence for the audit event is
        // connection 1's already-committed row above (same transaction_id,
        // event_id, occurred_at and envelope hash).
        let statements = [
            format!(
                "INSERT INTO game_item_transfer_reservations VALUES \
                 ('{character}', 777, '{character}', '{world}', '{channel}', '{source_text}', 1, \
                   decode(repeat('ab',33),'hex'), '{tx_id}', '{event_id}', 1000, 1, 900)"
            ),
            format!(
                "UPDATE game_item_instances SET last_transaction_id = '{tx_id}' \
                   WHERE item_instance_id = '{source_text}' AND lifecycle = 1 AND quantity = 1"
            ),
            format!(
                "DELETE FROM game_item_ground_locations WHERE item_instance_id = '{source_text}'"
            ),
            format!(
                "INSERT INTO game_item_container_slots VALUES \
                 ('{character}', '{source_text}', '{world}', '{tx_id}')"
            ),
            format!(
                "INSERT INTO game_item_transfer_receipts \
                   (game_session_id, command_id, character_id, intent_binding, transaction_id, \
                    event_id, shape, source_item_instance_id, source_quantity_before, \
                    source_quantity_after, occurred_at, envelope_sha256, committed_at) \
                 VALUES \
                 ('{character}', 777, '{character}', decode(repeat('ab',33),'hex'), '{tx_id}', \
                   '{event_id}', 1, '{source_text}', 1, 1, 1000, \
                   sha256(decode(repeat('ab',16),'hex')), 1000)"
            ),
        ];
        for statement in &statements {
            sqlx::query(sqlx::AssertSqlSafe(statement.clone()))
                .execute(&mut *b)
                .await?;
        }
        let applied = b.commit().await;
        assert!(
            applied.is_err(),
            "a foreign transaction's already-committed audit row, forged to name B's own xid, \
             was accepted as B's audit evidence"
        );
        // Nothing moved: the item is still on Ground, untouched.
        assert_eq!(harness.item_state(source).await?, (1, 1));
        assert!(harness.on_ground(source).await?);
        assert_eq!(harness.count("game_item_container_slots").await?, 0);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// D82's absolute stack ceiling (100): the receipt quantity CHECKs cap every
/// `source_quantity_before`/`after` and `receiver_quantity_before`/`after` at
/// 100, so no TRANSFER -- however forged -- can commit an oversized stack. A
/// genuine 100-unit receiver stack and a genuine 1-unit compatible Ground
/// source, merged by a forged full-merge (shape 3) receipt that claims
/// `receiver_quantity_after = 101`, must be rejected even though every other
/// fact (reservation, audit event, item evidence, Ground removal evidence)
/// is genuine.
#[test]
fn transfer_cannot_exceed_the_stack_ceiling() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "ceiling").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let backpack = harness.mint(&authority, BACKPACK, 1).await?;
        harness
            .committed(&authority, to_slot(command(1)?, backpack, backpack_facts()))
            .await?;

        // A genuine 100-unit receiver stack, placed as the backpack's only
        // entry through the real commit path.
        let receiver = harness.mint(&authority, COIN, 100).await?;
        harness
            .committed(
                &authority,
                to_backpack(command(2)?, receiver, facts(COIN, STACKABLE)),
            )
            .await?;
        assert_eq!(harness.item_state(receiver).await?, (100, 1));

        // A genuine 1-unit compatible Ground source, left on Ground.
        let source = harness.mint(&authority, COIN, 1).await?;
        assert!(harness.on_ground(source).await?);

        let uuid = uuid_text;
        let character = uuid(id(CHARACTER));
        let world = uuid(id(WORLD));
        let channel = uuid(id(CHANNEL));
        let source_text = uuid(source);
        let receiver_text = uuid(receiver);
        let tx_id = uuid(id(92));
        let event_id = uuid(id(93));
        // Every statement below is one a real full-merge TRANSFER commit
        // would issue -- fresh reservation, fresh audit row of this same
        // physical transaction, the item's real World and Channel, the
        // receiver's real prior quantity -- except the receipt claims a
        // forged 101-unit receiver total, one past the D82 ceiling.
        let statements = [
            format!(
                "INSERT INTO game_item_transfer_reservations VALUES \
                 ('{character}', 888, '{character}', '{world}', '{channel}', '{source_text}', 2, \
                   decode(repeat('ab',33),'hex'), '{tx_id}', '{event_id}', 1000, 1, 900)"
            ),
            format!(
                "INSERT INTO game_item_audit_outbox VALUES \
                 ('{event_id}', '{tx_id}', 1, 1, 2, 1, \
                   'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1', '{source_text}', 1000, \
                   7776001000, decode(repeat('ab',16),'hex'), \
                   sha256(decode(repeat('ab',16),'hex')), 1, NULL)"
            ),
            format!(
                "UPDATE game_item_instances SET last_transaction_id = '{tx_id}', \
                   quantity = 0, lifecycle = 2 \
                   WHERE item_instance_id = '{source_text}' AND lifecycle = 1 AND quantity = 1"
            ),
            format!(
                "UPDATE game_item_instances SET last_transaction_id = '{tx_id}', quantity = 101 \
                   WHERE item_instance_id = '{receiver_text}' AND lifecycle = 1 AND quantity = 100"
            ),
            format!(
                "DELETE FROM game_item_ground_locations WHERE item_instance_id = '{source_text}'"
            ),
            format!(
                "INSERT INTO game_item_transfer_receipts \
                   (game_session_id, command_id, character_id, intent_binding, transaction_id, \
                    event_id, shape, source_item_instance_id, source_quantity_before, \
                    source_quantity_after, receiver_item_instance_id, receiver_quantity_before, \
                    receiver_quantity_after, occurred_at, envelope_sha256, committed_at) \
                 VALUES \
                 ('{character}', 888, '{character}', decode(repeat('ab',33),'hex'), '{tx_id}', \
                   '{event_id}', 3, '{source_text}', 1, 0, '{receiver_text}', 100, 101, 1000, \
                   sha256(decode(repeat('ab',16),'hex')), 1000)"
            ),
        ];
        let mut tx = harness.pool.begin().await?;
        let applied: TestResult = async {
            for statement in &statements {
                sqlx::query(sqlx::AssertSqlSafe(statement.clone()))
                    .execute(&mut *tx)
                    .await?;
            }
            tx.commit().await?;
            Ok(())
        }
        .await;
        assert!(
            applied.is_err(),
            "a forged full-merge receipt exceeding the D82 100-unit stack ceiling was accepted"
        );
        // Nothing changed: the source is still on Ground at 1 unit, and the
        // receiver stack is still at its genuine 100.
        assert_eq!(harness.item_state(source).await?, (1, 1));
        assert!(harness.on_ground(source).await?);
        assert_eq!(harness.item_state(receiver).await?, (100, 1));

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// A new TRANSFER's same-transaction audit outbox row must start pending and
/// unpublished (`publication_state = 1`, `published_at IS NULL`); only the
/// publisher's later acknowledgement may advance it. A forged commit whose
/// outbox row is inserted already published -- `publication_state = 2` with
/// a non-NULL `published_at` at or after `occurred_at` -- every other field
/// genuine, captured by the real triggers, must be rejected.
#[test]
fn transfer_audit_event_must_start_pending() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "auditpending").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let source = harness.mint(&authority, STONE, 1).await?;
        assert!(harness.on_ground(source).await?);

        let uuid = uuid_text;
        let character = uuid(id(CHARACTER));
        let world = uuid(id(WORLD));
        let channel = uuid(id(CHANNEL));
        let source_text = uuid(source);
        let tx_id = uuid(id(232));
        let event_id = uuid(id(233));
        let statements = [
            format!(
                "INSERT INTO game_item_transfer_reservations VALUES \
                 ('{character}', 1, '{character}', '{world}', '{channel}', \
                   '{source_text}', 1, decode(repeat('ab',33),'hex'), '{tx_id}', '{event_id}', \
                   1000, 1, 900)"
            ),
            // Every other field genuine, but the outbox row is inserted
            // already published: publication_state = 2 and published_at
            // (1000, at occurred_at) rather than the required pending
            // publication_state = 1 with published_at IS NULL.
            format!(
                "INSERT INTO game_item_audit_outbox VALUES \
                 ('{event_id}', '{tx_id}', 1, 1, 2, 1, \
                   'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1', '{source_text}', 1000, \
                   7776001000, decode(repeat('ab',16),'hex'), \
                   sha256(decode(repeat('ab',16),'hex')), 2, 1000)"
            ),
            format!(
                "UPDATE game_item_instances SET last_transaction_id = '{tx_id}' \
                   WHERE item_instance_id = '{source_text}' AND lifecycle = 1 AND quantity = 1"
            ),
            format!(
                "DELETE FROM game_item_ground_locations WHERE item_instance_id = '{source_text}'"
            ),
            format!(
                "INSERT INTO game_item_container_slots VALUES \
                 ('{character}', '{source_text}', '{world}', '{tx_id}')"
            ),
            format!(
                "INSERT INTO game_item_transfer_receipts \
                   (game_session_id, command_id, character_id, intent_binding, transaction_id, \
                    event_id, shape, source_item_instance_id, source_quantity_before, \
                    source_quantity_after, occurred_at, envelope_sha256, committed_at) \
                 VALUES \
                 ('{character}', 1, '{character}', decode(repeat('ab',33),'hex'), '{tx_id}', \
                   '{event_id}', 1, '{source_text}', 1, 1, 1000, \
                   sha256(decode(repeat('ab',16),'hex')), 1000)"
            ),
        ];
        let mut tx = harness.pool.begin().await?;
        let applied: TestResult = async {
            for statement in &statements {
                sqlx::query(sqlx::AssertSqlSafe(statement.clone()))
                    .execute(&mut *tx)
                    .await?;
            }
            tx.commit().await?;
            Ok(())
        }
        .await;
        assert!(
            applied.is_err(),
            "a TRANSFER's audit outbox row inserted already published was accepted"
        );
        assert_eq!(harness.item_state(source).await?, (1, 1));
        assert!(harness.on_ground(source).await?);
        assert_eq!(harness.count("game_item_container_slots").await?, 0);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
