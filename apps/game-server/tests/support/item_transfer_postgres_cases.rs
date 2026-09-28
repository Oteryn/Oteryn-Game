// Shared B3-1 DUR-03 TRANSFER cases (Ground -> CharacterEquipment container
// slot / main backpack entries, D83 merge shapes). Both wrappers provide the
// same path-loaded crate root.
//
// Content has no definition with a known `container`-slot equip pattern yet
// (the backpack oteryn:item.registry.i00002752 declares capacity 20 but its
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

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const WORLD: u8 = 42;
const CHANNEL: u8 = 43;
const CHARACTER: u8 = 41;
const SESSION: u8 = 50;
const BACKPACK: &str = "fixture:b3.backpack";
const COIN: &str = "fixture:b3.coin";
const STONE: &str = "fixture:b3.stone";

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

async fn register(root: &DurabilityRoot, tag: u8) -> TestResult<NodeIncarnationProof> {
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

    async fn count(&self, relation: &str) -> TestResult<i64> {
        Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {relation}"
        )))
        .fetch_one(&self.pool)
        .await?)
    }

    /// Row counts of every relation a TRANSFER could write.
    async fn footprint(&self) -> TestResult<Vec<i64>> {
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

fn refused(
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

        // Least privilege: the runtime role may only perform TRANSFER writes.
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
               NOT has_table_privilege('oteryn_game_runtime','game_item_container_entries','DELETE'),\
               NOT has_table_privilege('oteryn_game_runtime','game_item_container_entries','UPDATE'),\
               has_table_privilege('oteryn_game_runtime','game_item_transfer_receipts','INSERT'),\
               NOT has_table_privilege('oteryn_game_runtime','game_item_transfer_receipts','UPDATE'),\
               has_column_privilege('oteryn_game_runtime','game_item_transfer_reservations',\
                 'work_units_used','UPDATE'),\
               NOT has_column_privilege('oteryn_game_runtime','game_item_transfer_reservations',\
                 'transaction_id','UPDATE'),\
               has_table_privilege('oteryn_game_control','game_item_transfer_receipts','SELECT'),\
               NOT has_table_privilege('oteryn_game_control','game_item_transfer_receipts','INSERT')])",
        )
        .fetch_all(&harness.pool)
        .await?;
        assert_eq!(grants.len(), 16);
        assert!(grants.iter().all(|granted| *granted), "{grants:?}");

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
