// Shared B3-1 DUR-03 TRANSFER cases (Ground -> CharacterEquipment container
// slot / main backpack entries, D83 merge shapes). Both wrappers provide the
// same path-loaded crate root.
//
// Content has no definition with a known `container`-slot equip pattern yet
// (the backpack oteryn:item.tibia.i2854 declares capacity 20 but its
// equipment semantics are UNKNOWN), so the backpack below is a test-only
// fixture definition, like the D1 CombatDeathFixture.

use crate::durability::item_weight_lookup::TEST_WEIGHTS;

use crate::character_recovery_fence::CharacterRecoveryStore;
use crate::domain::CharacterId;
use crate::durability::DurabilityRoot;
use crate::durability::admission_authority_guards::GuardPublicationDisposition;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};
use crate::durability::item_transfer::{
    CommittedItemTransfer, ContainerEntryPosition, CurrentCharacterItemFence, ItemDefinitionFacts,
    ItemStackClass, ItemTransferDestination, ItemTransferError, ItemTransferOutcome,
    ItemTransferRefusal, ItemTransferRequest,
};
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, BootstrapSecret, ControlActor,
    LaunchBinding, NodeIncarnationProof, OperationKey, RuntimeScopeAssignmentWriter,
};
use crate::durability::spell_owner_commit::SpellLanePermit;
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
use sqlx::{Connection, Executor, Row};
use std::future::Future;
use std::task::Poll;

// `pub(crate)` on the generic harness pieces below (TestResult, WORLD/CHANNEL/CHARACTER/
// SESSION, id, debug, configured_admin, runtime, Database, fence_parent, register, scope,
// seed_character, Harness) lets `combat_pickup_postgres_cases.rs` reuse the exact same
// PostgreSQL 18 DB/admission/session bootstrap (B3-2, `combat_pickup_postgres.rs`) instead of
// duplicating it; behavior is unchanged.
pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(crate) const WORLD: u8 = 42;
pub(crate) const CHANNEL: u8 = 43;
pub(crate) const CHARACTER: u8 = 41;
pub(crate) const SESSION: u8 = 50;
pub(crate) const BACKPACK: &str = "fixture:b3.backpack";
pub(crate) const COIN: &str = "fixture:b3.coin";
pub(crate) const STONE: &str = "fixture:b3.stone";

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
        // The protected lane pins PostgreSQL 18.6; any 18.x server runs the
        // same schema, and the exact server is recorded as evidence.
        assert!(
            version.starts_with("18"),
            "PostgreSQL 18 required: {version}"
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
        self.mint_definition(authority, definition("Item", key), quantity)
            .await
    }

    /// [`Self::mint`] of an item with an explicit definition revision.
    pub(crate) async fn mint_definition(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        item: TypedDefinitionRef,
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
            item,
            quantity,
            ground: GroundPlacement {
                // Canonical 10-byte Ground64 address (x=1, y=2, floor=3).
                spatial_position: vec![0, 0, 0, 1, 0, 0, 0, 2, 0, 3],
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
            .commit_item_mint(
                &candidate.fresh_death_lane_permit().await,
                authority,
                &self.node,
                &mut candidate,
            )
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
            .freeze_item_transfer(authority, &self.node, fence, request, &TEST_WEIGHTS)
            .await?;
        self.root
            .commit_item_transfer(
                &SpellLanePermit::of_fresh_scope(fence.runtime_scope).await?,
                authority,
                &self.node,
                fence,
                &mut candidate,
                &TEST_WEIGHTS,
            )
            .await
    }

    pub(crate) async fn committed(
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

    pub(crate) async fn character_revision(&self) -> TestResult<String> {
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

pub(crate) fn definition(family: &str, key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: family.into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    }
}

pub(crate) fn facts(key: &str, stack: ItemStackClass) -> ItemDefinitionFacts {
    ItemDefinitionFacts {
        weight: Some(1),
        definition: definition("Item", key),
        stack,
        container_capacity: None,
        container_slot_equip_pattern: false,
    }
}

/// Test-only backpack: capacity 20 and a complete `container`-slot pattern.
pub(crate) fn backpack_facts() -> ItemDefinitionFacts {
    ItemDefinitionFacts {
        weight: Some(1),
        definition: definition("Item", BACKPACK),
        stack: ItemStackClass::NonStackable,
        container_capacity: Some(20),
        container_slot_equip_pattern: true,
    }
}

pub(crate) const STACKABLE: ItemStackClass = ItemStackClass::Stackable {
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
        weight_generation_digest: [0xD1; 32],
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

pub(crate) fn rejected(
    result: Result<impl std::fmt::Debug, ItemTransferError>,
    label: &str,
) -> TestResult {
    match result {
        Err(ItemTransferError::AuthorityRejected) => Ok(()),
        other => Err(format!("{label}: expected AuthorityRejected, got {other:?}").into()),
    }
}

pub(crate) fn at(parent: [u8; 16], ordinal: u64) -> Option<ContainerEntryPosition> {
    Some(ContainerEntryPosition {
        parent_item_instance_id: parent,
        placement_ordinal: ordinal,
    })
}
