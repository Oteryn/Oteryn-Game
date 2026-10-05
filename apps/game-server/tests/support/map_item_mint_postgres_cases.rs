// Shared MAP-OVERLAY-1b DUR-03 map-item MINT cases (ADR-0021 §4.4: an
// eligible base-map entry is minted into Ground at its own tile under the
// player's pickup CommandRef, then picked up through the real B3-1 TRANSFER).
// Both wrappers provide the same path-loaded crate root. The backpack is
// equipped through the real B3-1 TRANSFER path; its definition is the same
// test-only fixture as the B3-1 cases.

use crate::character_recovery_fence::CharacterRecoveryStore;
use crate::domain::CharacterId;
use crate::durability::DurabilityRoot;
use crate::durability::admission_authority_guards::GuardPublicationDisposition;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};
use crate::durability::item_transfer::{
    CurrentCharacterItemFence, ItemDefinitionFacts, ItemStackClass, ItemTransferDestination,
    ItemTransferOutcome, ItemTransferRequest,
};
use crate::durability::map_item_mint::{
    CommittedMapItemMint, MapItemMintError, MapItemMintOutcome, MapItemMintRefusal,
    MapItemMintRequest, MapItemPlacement,
};
use crate::durability::map_item_mint_audit as audit;
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
use oteryn_world_bundle::bundle::placement_key;
use sqlx::{Connection, Executor, Row};
use std::future::Future;
use std::task::Poll;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const WORLD: u8 = 42;
const CHANNEL: u8 = 43;
const SECOND_CHANNEL: u8 = 44;
const CHARACTER: u8 = 41;
const SESSION: u8 = 50;
const SECOND_SESSION: u8 = 52;
const BACKPACK: &str = "fixture:b3.backpack";
const COIN: &str = "fixture:b3.coin";
const STONE: &str = "fixture:b3.stone";
const DIGEST: [u8; 32] = [0xd1; 32];
const FLOOR: i8 = -7;

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
        let name = format!("mim_{name}_{suffix}");
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
        eprintln!("MAP-ITEM-MINT-PG: database {name} on server_version_num={version}");
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
        "oteryn-map-item-mint-parent-{}",
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
    let launch = LaunchBinding::new(&format!("map-item-launch-{tag}")).map_err(debug)?;
    let node = crate::foundation::NodeId::decode(&id(tag)).map_err(debug)?;
    root.issue_node_bootstrap_authorization(&secret, &launch, None)
        .await
        .map_err(debug)?;
    root.register_node_incarnation(&secret, &launch, node)
        .await
        .map_err(|error| debug(error).into())
}

fn scope_in(channel: u8) -> TestResult<RuntimeScopeRefV1> {
    Ok(RuntimeScopeRefV1::channel(
        WorldId::decode(&id(WORLD)).map_err(debug)?,
        ChannelId::decode(&id(channel)).map_err(debug)?,
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
    open_channel(pool, root, node, CHANNEL).await
}

/// Assign the Channel scope `channel` to `node` (generation 1) and publish
/// its runtime readiness.
async fn open_channel(
    pool: &sqlx::PgPool,
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    channel: u8,
) -> TestResult {
    sqlx::query(
        "INSERT INTO game_control_scope_grants \
         (control_role, world_id, channel_id, operation) \
         VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1)",
    )
    .bind(id(WORLD).as_slice())
    .bind(id(channel).as_slice())
    .execute(pool)
    .await?;
    let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "map-item-writer")
        .await
        .map_err(debug)?;
    let assignment = writer
        .submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([channel; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            command: AssignmentCommand::Assign {
                scope: scope_in(channel)?,
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
        key: AdmissionAuthorityGuardKeyV1::Runtime(scope_in(channel)?),
        source: AdmissionPublicationSourceV1 {
            authority: "game-runtime-publisher".into(),
            purpose: AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness,
            source_revision: 1,
            decision_identity: format!("map-item-runtime-ready-{channel}"),
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

    /// MINT the fixture backpack on Ground through the real stage C path
    /// and equip it through the real B3-1 TRANSFER path.
    async fn equip_backpack(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        command_id: u64,
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
            item: definition("Item", BACKPACK),
            quantity: 1,
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
        let backpack = match self
            .root
            .commit_item_mint(authority, &self.node, &mut candidate)
            .await
            .map_err(debug)?
        {
            ItemMintOutcome::Committed(result) => result.item_instance_id,
            other => return Err(format!("unexpected MINT outcome {other:?}").into()),
        };
        let request = ItemTransferRequest {
            command: command(command_id)?,
            source_item_instance_id: backpack,
            destination: ItemTransferDestination::ContainerSlot,
            item: backpack_facts(),
            backpack: None,
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
        };
        self.pick_up(authority, fence()?, request).await?;
        Ok(backpack)
    }

    /// One B3-1 TRANSFER that must commit fresh.
    async fn pick_up(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterItemFence,
        request: ItemTransferRequest,
    ) -> TestResult {
        let mut candidate = self
            .root
            .freeze_item_transfer(authority, &self.node, fence, request)
            .await
            .map_err(debug)?;
        match self
            .root
            .commit_item_transfer(authority, &self.node, fence, &mut candidate)
            .await
            .map_err(debug)?
        {
            ItemTransferOutcome::Committed(_) => Ok(()),
            other => Err(format!("expected a fresh TRANSFER, got {other:?}").into()),
        }
    }

    async fn take(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterItemFence,
        request: MapItemMintRequest,
    ) -> Result<MapItemMintOutcome, MapItemMintError> {
        let mut candidate = self
            .root
            .freeze_map_item_mint(authority, &self.node, fence, request)
            .await?;
        self.root
            .commit_map_item_mint(authority, &self.node, fence, &mut candidate)
            .await
    }

    async fn taken(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterItemFence,
        request: MapItemMintRequest,
    ) -> TestResult<CommittedMapItemMint> {
        match self.take(authority, fence, request).await.map_err(debug)? {
            MapItemMintOutcome::Committed(result) => Ok(result),
            other => Err(format!("expected a fresh map-item MINT, got {other:?}").into()),
        }
    }

    async fn count(&self, relation: &str) -> TestResult<i64> {
        Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {relation}"
        )))
        .fetch_one(&self.pool)
        .await?)
    }

    /// Row counts of every relation a map-item MINT could write.
    async fn footprint(&self) -> TestResult<Vec<i64>> {
        let mut counts = Vec::new();
        for relation in [
            "game_map_item_mint_reservations",
            "game_map_item_mint_receipts",
            "game_item_instances",
            "game_item_ground_locations",
            "game_item_container_entries",
            "game_item_audit_outbox",
        ] {
            counts.push(self.count(relation).await?);
        }
        Ok(counts)
    }

    /// The Ground row of `item`: (Channel, spatial position, map revision),
    /// or `None` when the item is not on Ground.
    async fn ground_of(&self, item: [u8; 16]) -> TestResult<Option<(String, Vec<u8>, String)>> {
        let row = sqlx::query(
            "SELECT channel_id::text, spatial_position, map_revision \
               FROM game_item_ground_locations \
              WHERE item_instance_id = encode($1,'hex')::uuid",
        )
        .bind(item.as_slice())
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            Ok((
                row.try_get("channel_id")?,
                row.try_get("spatial_position")?,
                row.try_get("map_revision")?,
            ))
        })
        .transpose()
    }

    async fn in_backpack(&self, item: [u8; 16]) -> TestResult<bool> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM game_item_container_entries \
                             WHERE item_instance_id = encode($1,'hex')::uuid)",
        )
        .bind(item.as_slice())
        .fetch_one(&self.pool)
        .await?)
    }

    /// The decoded audit event of a committed map-item MINT, after checking
    /// that its digest is the receipt's.
    async fn audit_of(
        &self,
        minted: &CommittedMapItemMint,
    ) -> TestResult<(
        crate::durability::item_mint_audit::EventEnvelopeV1,
        audit::OneItemMapItemMintV1,
    )> {
        let row = sqlx::query(
            "SELECT a.envelope, a.envelope_sha256 = r.envelope_sha256 AS digest_ok \
               FROM game_item_audit_outbox a \
               JOIN game_map_item_mint_receipts r USING (event_id, transaction_id) \
              WHERE a.transaction_id = encode($1,'hex')::uuid",
        )
        .bind(minted.transaction_id.as_slice())
        .fetch_one(&self.pool)
        .await?;
        assert!(row.try_get::<bool, _>("digest_ok")?);
        Ok(
            audit::decode_map_item_mint_envelope(&row.try_get::<Vec<u8>, _>("envelope")?)
                .map_err(debug)?,
        )
    }

    /// Move the Character to a new GameSession in Channel `channel`.
    async fn move_to_channel(&self, channel: u8) -> TestResult {
        open_channel(&self.pool, &self.root, &self.node, channel).await?;
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state = 3")
            .execute(&self.pool)
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
        .bind(id(SECOND_SESSION).as_slice())
        .bind(id(40).as_slice())
        .bind(id(CHARACTER).as_slice())
        .bind(id(WORLD).as_slice())
        .bind(id(channel).as_slice())
        .bind([8_u8; 16].as_slice())
        .execute(&self.pool)
        .await?;
        for guard in [
            "game_durability_admission_character_guards",
            "game_durability_admission_account_guards",
        ] {
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "UPDATE {guard} SET holder_game_session_id = encode($1,'hex')::uuid"
            )))
            .bind(id(SECOND_SESSION).as_slice())
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    /// Write past the runtime guards (triggers off); CHECK and UNIQUE still hold.
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

fn command_in(session: u8, value: u64) -> TestResult<CommandRef> {
    Ok(CommandRef::new(
        GameSessionId::decode(&id(session)).map_err(debug)?,
        CommandId::new(value).map_err(debug)?,
    ))
}

fn command(value: u64) -> TestResult<CommandRef> {
    command_in(SESSION, value)
}

fn fence_in(session: u8, channel: u8) -> TestResult<CurrentCharacterItemFence> {
    Ok(CurrentCharacterItemFence {
        character_id: CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?,
        game_session_id: GameSessionId::decode(&id(session)).map_err(debug)?,
        connection_generation: ConnectionGeneration::new(1).map_err(debug)?,
        character_lease_generation: 1,
        runtime_scope: scope_in(channel)?,
        scope_ownership_generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
    })
}

fn fence() -> TestResult<CurrentCharacterItemFence> {
    fence_in(SESSION, CHANNEL)
}

/// The `placement_key` of top-level entry `ordinal` of tile (x, y) on the
/// fixture floor.
fn entry(x: u16, y: u16, ordinal: u8) -> TestResult<u64> {
    Ok(placement_key(FLOOR, x, y, ordinal).ok_or("placement key")?)
}

/// The pickup of `quantity` coins of the entry `key` in reset epoch `epoch`.
fn pickup(command: CommandRef, key: u64, epoch: u64, quantity: u32) -> MapItemMintRequest {
    MapItemMintRequest {
        command,
        base_bundle_digest: DIGEST,
        placement_key: key,
        reset_epoch: epoch,
        item: facts(COIN, STACKABLE),
        quantity,
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
    }
}

/// The B3-1 pickup of a minted item from Ground into the main backpack.
fn into_backpack(command: CommandRef, item: [u8; 16]) -> ItemTransferRequest {
    ItemTransferRequest {
        command,
        source_item_instance_id: item,
        destination: ItemTransferDestination::MainBackpack,
        item: facts(COIN, STACKABLE),
        backpack: Some(backpack_facts()),
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
    }
}

fn refused(
    result: Result<impl std::fmt::Debug, MapItemMintError>,
    expected: MapItemMintRefusal,
) -> TestResult {
    match result {
        Err(MapItemMintError::Refused(reason)) if reason == expected => Ok(()),
        other => Err(format!("expected refusal {expected:?}, got {other:?}").into()),
    }
}

fn rejected(result: Result<impl std::fmt::Debug, MapItemMintError>, label: &str) -> TestResult {
    match result {
        Err(MapItemMintError::AuthorityRejected) => Ok(()),
        other => Err(format!("{label}: expected AuthorityRejected, got {other:?}").into()),
    }
}

/// The overlay position of the tile a committed map-item MINT placed its
/// item on: the entry's own tile.
fn minted_at(minted: &CommittedMapItemMint) -> production_server::map::overlay::TilePos {
    production_server::map::overlay::TilePos {
        x: minted.placement.x,
        y: minted.placement.y,
        floor: minted.placement.floor,
    }
}

fn reaches(player: (u16, u16, i8), minted: &CommittedMapItemMint) -> bool {
    let (x, y, floor) = player;
    production_server::map::overlay::pickup::within_reach(
        production_server::map::overlay::TilePos { x, y, floor },
        minted_at(minted),
    )
}

#[test]
fn map_item_mint_takes_an_entry_into_ground_then_transfers_it_once_per_channel_and_epoch()
-> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "take").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        harness.equip_backpack(&authority, 1).await?;
        let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
        let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
        let key = entry(300, 200, 2)?;

        // MINT into Ground at the entry's own tile, in this Channel, of this
        // base bundle.
        let first = harness
            .taken(&authority, fence()?, pickup(command(10)?, key, 0, 5))
            .await?;
        assert_eq!(
            first.placement,
            MapItemPlacement {
                x: 300,
                y: 200,
                floor: FLOOR,
                ordinal: 2,
            }
        );
        assert_eq!(first.quantity, 5);
        assert_eq!(
            harness.ground_of(first.item_instance_id).await?,
            Some((
                uuid_text(id(CHANNEL)),
                audit::map_item_spatial_position(300, 200, FLOOR),
                audit::map_revision_of(&DIGEST),
            ))
        );

        // The audit event is the registered map-item MINT of the receipt,
        // with its full cause.
        let (envelope, payload) = harness.audit_of(&first).await?;
        assert_eq!(envelope.command_id, Some(10));
        assert_eq!(envelope.channel_id, Some(id(CHANNEL).to_vec()));
        assert_eq!(payload.connection_generation, 1);
        assert!(payload.before_semantically_absent);
        let source = payload.source.ok_or("cause")?;
        assert_eq!(source.typed_cause, audit::MAP_ITEM_MINT_TYPED_CAUSE);
        assert_eq!(
            source.command_ref.map(|command| command.command_id),
            Some(10)
        );
        let materialization = source.materialization.ok_or("entry")?;
        assert_eq!(
            (
                materialization.world_id,
                materialization.channel_id,
                materialization.base_bundle_digest,
                materialization.placement_key,
                materialization.reset_epoch,
            ),
            (
                id(WORLD).to_vec(),
                id(CHANNEL).to_vec(),
                DIGEST.to_vec(),
                key,
                0
            )
        );

        // Reach is checked before the TRANSFER: out of reach, nothing moves;
        // within reach, the B3-1 TRANSFER picks the item up.
        assert!(!reaches((302, 200, FLOOR), &first));
        assert!(!reaches((300, 200, FLOOR - 1), &first));
        assert!(reaches((301, 201, FLOOR), &first));
        harness
            .pick_up(
                &authority,
                fence()?,
                into_backpack(command(11)?, first.item_instance_id),
            )
            .await?;
        assert_eq!(harness.ground_of(first.item_instance_id).await?, None);
        assert!(harness.in_backpack(first.item_instance_id).await?);

        // A retried MINT returns the existing item; reach is checked on it
        // exactly as on a fresh one.
        let mut replay = harness
            .root
            .freeze_map_item_mint(
                &authority,
                &harness.node,
                fence()?,
                pickup(command(10)?, key, 0, 5),
            )
            .await
            .map_err(debug)?;
        assert_eq!(replay.transaction_id(), &first.transaction_id);
        assert_eq!(replay.item_instance_id(), &first.item_instance_id);
        let retried = match harness
            .root
            .commit_map_item_mint(&authority, &harness.node, fence()?, &mut replay)
            .await
            .map_err(debug)?
        {
            MapItemMintOutcome::AlreadyCommitted(result) => result,
            other => return Err(format!("expected the retained result, got {other:?}").into()),
        };
        assert_eq!(retried, first);
        assert!(!reaches((302, 200, FLOOR), &retried));
        assert!(reaches((299, 199, FLOOR), &retried));
        assert_eq!(
            harness
                .root
                .reconcile_map_item_mint(&authority, &mut replay)
                .await
                .map_err(debug)?,
            Some(first.clone())
        );
        // DUR03-RL-08: the third unit is the last; the fourth is rejected.
        assert_eq!(replay.work_units_used(), 3);
        assert!(matches!(
            harness
                .root
                .reconcile_map_item_mint(&authority, &mut replay)
                .await,
            Err(MapItemMintError::CapacityExceeded)
        ));
        // Same command, different intent: integrity conflict.
        assert!(matches!(
            harness
                .root
                .freeze_map_item_mint(
                    &authority,
                    &harness.node,
                    fence()?,
                    pickup(command(10)?, key, 0, 6),
                )
                .await,
            Err(MapItemMintError::ConflictingCause)
        ));

        // Once per Channel and reset epoch: another command for the same
        // entry is refused with nothing written.
        let before = harness.footprint().await?;
        refused(
            harness
                .take(&authority, fence()?, pickup(command(12)?, key, 0, 5))
                .await,
            MapItemMintRefusal::AlreadyTaken,
        )?;
        assert_eq!(harness.footprint().await?, before);
        // Another entry of the same tile, and the same entry in the next
        // reset epoch, are each taken once.
        let sibling = entry(300, 200, 3)?;
        harness
            .taken(&authority, fence()?, pickup(command(13)?, sibling, 0, 1))
            .await?;
        harness
            .taken(&authority, fence()?, pickup(command(14)?, key, 1, 5))
            .await?;
        refused(
            harness
                .take(&authority, fence()?, pickup(command(15)?, key, 1, 5))
                .await,
            MapItemMintRefusal::AlreadyTaken,
        )?;

        // The rebuild listing: every taken entry of this Channel, digest and
        // epoch, ascending; more than the bundle holds is never truncated.
        let listed = |epoch, digest, max| {
            harness
                .root
                .read_map_item_mint_placements(world, channel, digest, epoch, max)
        };
        assert_eq!(
            listed(0, DIGEST, 8).await.map_err(debug)?,
            vec![key, sibling]
        );
        assert_eq!(listed(1, DIGEST, 8).await.map_err(debug)?, vec![key]);
        assert_eq!(
            listed(2, DIGEST, 8).await.map_err(debug)?,
            Vec::<u64>::new()
        );
        assert_eq!(
            listed(0, [0xd2; 32], 8).await.map_err(debug)?,
            Vec::<u64>::new()
        );
        assert_eq!(
            listed(0, DIGEST, 2).await.map_err(debug)?,
            vec![key, sibling]
        );
        assert!(matches!(
            listed(0, DIGEST, 1).await,
            Err(MapItemMintError::CapacityExceeded)
        ));

        // A second Channel takes the same entry in the same epoch, once.
        harness.move_to_channel(SECOND_CHANNEL).await?;
        let second = harness
            .taken(
                &authority,
                fence_in(SECOND_SESSION, SECOND_CHANNEL)?,
                pickup(command_in(SECOND_SESSION, 1)?, key, 0, 5),
            )
            .await?;
        assert_ne!(second.item_instance_id, first.item_instance_id);
        assert_eq!(
            harness
                .ground_of(second.item_instance_id)
                .await?
                .map(|(channel, _, _)| channel),
            Some(uuid_text(id(SECOND_CHANNEL)))
        );
        refused(
            harness
                .take(
                    &authority,
                    fence_in(SECOND_SESSION, SECOND_CHANNEL)?,
                    pickup(command_in(SECOND_SESSION, 2)?, key, 0, 5),
                )
                .await,
            MapItemMintRefusal::AlreadyTaken,
        )?;
        let second_channel = ChannelId::decode(&id(SECOND_CHANNEL)).map_err(debug)?;
        assert_eq!(
            harness
                .root
                .read_map_item_mint_placements(world, second_channel, DIGEST, 0, 8)
                .await
                .map_err(debug)?,
            vec![key]
        );
        assert_eq!(harness.count("game_map_item_mint_receipts").await?, 4);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn map_item_mint_refusals_and_invalid_input_write_nothing() -> TestResult {
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
        let key = entry(40, 50, 0)?;
        let before = harness.footprint().await?;

        let refusals: [(MapItemMintRequest, MapItemMintRefusal); 3] = [
            (
                MapItemMintRequest {
                    item: backpack_facts(),
                    quantity: 1,
                    ..pickup(command(1)?, key, 0, 1)
                },
                MapItemMintRefusal::ItemIsContainer,
            ),
            (
                MapItemMintRequest {
                    item: facts(STONE, ItemStackClass::Unknown),
                    ..pickup(command(2)?, key, 0, 1)
                },
                MapItemMintRefusal::UnknownStackClass,
            ),
            (
                MapItemMintRequest {
                    item: facts(STONE, ItemStackClass::NonStackable),
                    ..pickup(command(3)?, key, 0, 2)
                },
                MapItemMintRefusal::QuantityAboveStackMaximum,
            ),
        ];
        for (request, expected) in refusals {
            refused(harness.take(&authority, fence()?, request).await, expected)?;
            assert_eq!(harness.footprint().await?, before, "{expected:?}");
        }

        let invalid: [(&str, MapItemMintRequest); 4] = [
            ("zero quantity", pickup(command(4)?, key, 0, 0)),
            (
                "placement key naming no top-level entry",
                pickup(command(5)?, key | 0xff, 0, 1),
            ),
            (
                "not an Item definition",
                MapItemMintRequest {
                    item: ItemDefinitionFacts {
                        definition: definition("Outfit", COIN),
                        ..facts(COIN, STACKABLE)
                    },
                    ..pickup(command(6)?, key, 0, 1)
                },
            ),
            (
                "empty content revision",
                MapItemMintRequest {
                    content_revision: String::new(),
                    ..pickup(command(7)?, key, 0, 1)
                },
            ),
        ];
        for (label, request) in invalid {
            assert!(
                matches!(
                    harness.take(&authority, fence()?, request).await,
                    Err(MapItemMintError::InvalidInput)
                ),
                "{label}"
            );
            assert_eq!(harness.footprint().await?, before, "{label}");
        }

        // The entry is still free after every refusal.
        harness
            .taken(&authority, fence()?, pickup(command(8)?, key, 0, 1))
            .await?;

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn map_item_mint_every_fence_operator_rejects_at_freeze_and_at_commit() -> TestResult {
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
                    ChannelId::decode(&id(SECOND_CHANNEL)).expect("channel"),
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
        for (index, (label, mutate)) in operators.into_iter().enumerate() {
            let x = 100 + u16::try_from(index)?;
            let mut stale = fence()?;
            mutate(&mut stale);
            // Freeze boundary: rejected, no reservation.
            next += 1;
            let before = harness.footprint().await?;
            rejected(
                harness
                    .root
                    .freeze_map_item_mint(
                        &authority,
                        &harness.node,
                        stale,
                        pickup(command(next)?, entry(x, 1, 0)?, 0, 1),
                    )
                    .await,
                label,
            )?;
            assert_eq!(harness.footprint().await?, before, "{label}");
            // Commit boundary: a valid freeze, then a stale commit mints
            // nothing and leaves the entry free.
            next += 1;
            let mut candidate = harness
                .root
                .freeze_map_item_mint(
                    &authority,
                    &harness.node,
                    fence()?,
                    pickup(command(next)?, entry(x, 2, 0)?, 0, 1),
                )
                .await
                .map_err(debug)?;
            let before = harness.footprint().await?;
            rejected(
                harness
                    .root
                    .commit_map_item_mint(&authority, &harness.node, stale, &mut candidate)
                    .await,
                label,
            )?;
            assert_eq!(harness.footprint().await?, before, "{label}");
            assert_eq!(
                harness
                    .root
                    .reconcile_map_item_mint(&authority, &mut candidate)
                    .await
                    .map_err(debug)?,
                None,
                "{label}"
            );
        }

        // The CommandRef of another GameSession under a valid fence.
        let before = harness.footprint().await?;
        rejected(
            harness
                .take(
                    &authority,
                    fence()?,
                    pickup(command_in(51, 90)?, entry(1, 1, 0)?, 0, 1),
                )
                .await,
            "foreign CommandRef",
        )?;
        assert_eq!(harness.footprint().await?, before);

        // Every rejected entry is still free under the valid fence.
        harness
            .taken(
                &authority,
                fence()?,
                pickup(command(91)?, entry(100, 2, 0)?, 0, 1),
            )
            .await?;

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn map_item_mint_concurrent_pickups_of_one_entry_mint_once() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "race").await?;
        let second_root = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
        assert!(second_root.maintain_ready_once().await?);
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let second_authority = second_root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let key = entry(7, 9, 1)?;
        // Two commands of one entry freeze on two roots while it is free;
        // their commits race.
        let mut first = harness
            .root
            .freeze_map_item_mint(
                &authority,
                &harness.node,
                fence()?,
                pickup(command(1)?, key, 0, 3),
            )
            .await
            .map_err(debug)?;
        let mut second = second_root
            .freeze_map_item_mint(
                &second_authority,
                &harness.node,
                fence()?,
                pickup(command(2)?, key, 0, 3),
            )
            .await
            .map_err(debug)?;
        let (a, b) = join_two(
            harness
                .root
                .commit_map_item_mint(&authority, &harness.node, fence()?, &mut first),
            second_root.commit_map_item_mint(
                &second_authority,
                &harness.node,
                fence()?,
                &mut second,
            ),
        )
        .await;
        let outcomes = [a, b];
        let committed = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, Ok(MapItemMintOutcome::Committed(_))))
            .count();
        let taken = outcomes
            .iter()
            .filter(|outcome| {
                matches!(
                    outcome,
                    Err(MapItemMintError::Refused(MapItemMintRefusal::AlreadyTaken))
                )
            })
            .count();
        assert_eq!((committed, taken), (1, 1), "{outcomes:?}");
        assert_eq!(harness.count("game_map_item_mint_receipts").await?, 1);
        assert_eq!(harness.count("game_item_ground_locations").await?, 1);

        // The database refuses a receipt whose Ground tile is not the tile
        // its placement key names, even outside the runtime guards.
        let forged = harness
            .tamper("UPDATE game_map_item_mint_receipts SET placement_key = placement_key + 65536")
            .await;
        assert!(forged.is_err(), "{forged:?}");

        drop(second_authority);
        drop(authority);
        drop(seal);
        drop(second_root);
        harness.cleanup().await
    })
}

/// ADR-0021 §4.4 overlay side of a map-item pickup, against a fixture bundle
/// assembled as in `map_overlay_channel`.
mod map_overlay_pickup {
    use std::error::Error as StdError;
    use std::sync::Arc;

    use oteryn_world_bundle::bundle::placement_key;
    use oteryn_world_bundle_compiler::Error;
    use oteryn_world_bundle_compiler::bundle::{
        self, BuildClass, Extent, Family, Identity, Manifest, Terrain, TerrainKind,
    };
    use oteryn_world_bundle_compiler::compile::{Input, KeyResolver, Resolution, compile};
    use oteryn_world_bundle_compiler::project::Families;
    use oteryn_world_bundle_compiler::sector::{self, Attrs, Item, Tile};
    use oteryn_world_bundle_compiler::spawn;
    use production_server::durability::map_item_mint::{CommittedMapItemMint, MapItemPlacement};
    use production_server::foundation::{ChannelId, WorldId};
    use production_server::map::overlay::pickup::{
        MintResolution, RehideError, hide_origin_at_freeze, rehide_taken_origins, settle_origin,
    };
    use production_server::map::overlay::{ChannelOverlay, OverlayError, TilePos};
    use production_server::map::{self, BundlePins, LoadError, WorldBase};
    use sha2::{Digest, Sha256};

    type TestResult = Result<(), Box<dyn StdError>>;

    const KEYS: [&str; 2] = ["terrain:grass", "item:coin"];

    struct Resolver;

    impl KeyResolver for Resolver {
        fn resolve(&self, key: &str) -> Resolution {
            match key {
                "terrain:grass" => Resolution::Resolved(Family::Terrain, 101),
                "item:coin" => Resolution::Resolved(Family::Item, 9),
                _ => Resolution::Unknown,
            }
        }

        fn terrain(&self, key: &str) -> Result<Option<Terrain>, Error> {
            Ok((key == "terrain:grass").then_some(Terrain {
                kind: TerrainKind::Ground,
                walkable: Some(true),
                ground_speed: Some(150),
            }))
        }

        fn floor_change(&self, _: &str) -> bool {
            false
        }
    }

    fn item(palette: u32) -> Item {
        Item {
            palette,
            depth: 0,
            attrs: Attrs::default(),
        }
    }

    fn tile(x: u16, y: u16, items: Vec<Item>) -> Tile {
        Tile {
            x,
            y,
            flags: 0,
            house: 0,
            zones: Vec::new(),
            items,
        }
    }

    /// The manifest of a one-tile compiled bundle; the fixtures reuse it with their own sectors.
    fn manifest() -> Result<Manifest, Box<dyn StdError>> {
        let frame =
            zstd::bulk::compress(&sector::encode(&[tile(1, 1, vec![item(0), item(1)])])?, 3)?;
        let mut region = b"OTRB".to_vec();
        region.extend_from_slice(&[1, 7]);
        for value in [0u16, 0, 1] {
            region.extend_from_slice(&value.to_le_bytes());
        }
        region.push(0);
        region.extend_from_slice(&21u32.to_le_bytes());
        region.extend_from_slice(&(frame.len() as u32).to_le_bytes());
        region.extend_from_slice(&frame);
        let palette: Vec<String> = KEYS.map(String::from).to_vec();
        let families = Families::default();
        let input = Input {
            regions: &[region],
            palette: &palette,
            identity: Identity {
                project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
                world_schema_version: "world-schema-1".into(),
                content_revision: "rev-1".into(),
                ..Identity::default()
            },
            world: Extent {
                min_x: 0,
                min_y: 0,
                max_x: 512,
                max_y: 256,
                floors: vec![-7],
            },
            build_class: BuildClass::NonProduction,
            draft_areas: Vec::new(),
            families: &families,
        };
        Ok(bundle::read(&compile(&input, &Resolver)?.bytes)?.manifest)
    }

    fn compress(raw: &[u8]) -> Result<Vec<u8>, Box<dyn StdError>> {
        let mut compressor = zstd::bulk::Compressor::new(3)?;
        compressor.include_checksum(true)?;
        compressor.include_contentsize(true)?;
        Ok(compressor.compress(raw)?)
    }

    /// A bundle of one sector at native floor -7, assembled byte by byte (as the MAP-LOAD-1 tests
    /// do) so a tile can hold more top-level entries than the compiler writes, and its digest.
    fn assemble(tiles: &[Tile]) -> Result<(Vec<u8>, [u8; 32]), Box<dyn StdError>> {
        let json = serde_json::to_vec(&manifest()?)?;
        let raw = sector::encode(tiles)?;
        let frame = compress(&raw)?;
        let spawn_raw = spawn::encode(&spawn::Table::default());
        let spawn_frame = compress(&spawn_raw)?;
        let u32_of = |value: usize| (value as u32).to_le_bytes();
        let mut out = b"OTWB".to_vec();
        out.extend_from_slice(&3u16.to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&u32_of(json.len()));
        out.extend_from_slice(&u32_of(1));
        out.extend_from_slice(&json);
        let offset = out.len() + 50 + 44;
        out.extend_from_slice(&[-7i8 as u8, 0]);
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&u32_of(offset));
        out.extend_from_slice(&u32_of(frame.len()));
        out.extend_from_slice(&u32_of(raw.len()));
        out.extend_from_slice(&Sha256::digest(&frame));
        out.extend_from_slice(&u32_of(offset + frame.len()));
        out.extend_from_slice(&u32_of(spawn_frame.len()));
        out.extend_from_slice(&u32_of(spawn_raw.len()));
        out.extend_from_slice(&Sha256::digest(&spawn_frame));
        out.extend_from_slice(&frame);
        out.extend_from_slice(&spawn_frame);
        let body = out.len();
        let digest: [u8; 32] = Sha256::new()
            .chain_update(b"OTERYN_WORLD_BUNDLE/v3\0")
            .chain_update(&out)
            .finalize()
            .into();
        out.extend_from_slice(&digest);
        debug_assert_eq!(out.len(), body + 32);
        Ok((out, digest))
    }

    fn load(tiles: &[Tile]) -> Result<Result<WorldBase, LoadError>, Box<dyn StdError>> {
        let (bytes, digest) = assemble(tiles)?;
        Ok(map::load(
            &bytes,
            &BundlePins {
                digest,
                project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
                world_schema_version: "world-schema-1".into(),
                content_revision: "rev-1".into(),
                production: false,
            },
        ))
    }

    /// The fixture base: (1, 1) holds 64 top-level entries, (2, 1) a grass ground under one coin,
    /// and every other tile of the sector a grass ground.
    fn base() -> Result<Arc<WorldBase>, Box<dyn StdError>> {
        let mut tiles = Vec::new();
        for y in 0..32 {
            for x in 0..32 {
                let items = match (x, y) {
                    (1, 1) => vec![item(0); 64],
                    (2, 1) => vec![item(0), item(1)],
                    _ => vec![item(0)],
                };
                tiles.push(tile(x, y, items));
            }
        }
        Ok(Arc::new(load(&tiles)??))
    }

    /// A UUIDv7 of `seed`.
    fn uuid(seed: u8) -> [u8; 16] {
        let mut bytes = [seed; 16];
        bytes[6] = 0x70 | (seed & 0x0f);
        bytes[8] = 0x80 | (seed & 0x3f);
        bytes
    }

    fn channel_overlay(
        base: &Arc<WorldBase>,
        budget: Option<usize>,
    ) -> Result<ChannelOverlay, Box<dyn StdError>> {
        let world = WorldId::decode(&uuid(1)).map_err(|e| format!("{e:?}"))?;
        let channel = ChannelId::decode(&uuid(2)).map_err(|e| format!("{e:?}"))?;
        Ok(match budget {
            Some(budget) => ChannelOverlay::with_budget(Arc::clone(base), world, channel, budget),
            None => ChannelOverlay::new(Arc::clone(base), world, channel),
        })
    }

    /// The (2, 1) coin: top-level entry 1 of its tile on floor -7.
    fn coin_key() -> Result<u64, Box<dyn StdError>> {
        Ok(placement_key(-7, 2, 1, 1).ok_or("key")?)
    }

    const COIN_TILE: TilePos = TilePos {
        x: 2,
        y: 1,
        floor: -7,
    };

    fn committed(key: u64) -> Result<CommittedMapItemMint, Box<dyn StdError>> {
        Ok(CommittedMapItemMint {
            transaction_id: [1; 16],
            event_id: [2; 16],
            item_instance_id: [3; 16],
            occurred_at_unix_ms: 1,
            envelope_sha256: [4; 32],
            quantity: 1,
            placement: MapItemPlacement::of_key(key).ok_or("placement")?,
        })
    }

    #[test]
    fn map_overlay_pickup_hides_the_origin_at_freeze_and_refuses_a_second_freeze() -> TestResult {
        let base = base()?;
        let mut overlay = channel_overlay(&base, None)?;
        let key = coin_key()?;
        hide_origin_at_freeze(&mut overlay, key)?;
        assert!(overlay.tile(COIN_TILE).ok_or("tile")?.is_hidden(1));
        // A pickup in flight, or a taken entry, refuses another freeze.
        assert_eq!(
            hide_origin_at_freeze(&mut overlay, key),
            Err(OverlayError::AlreadyHidden)
        );
        // A key naming no top-level entry of the base is refused.
        let past = placement_key(-7, 2, 1, 2).ok_or("key")?;
        assert_eq!(
            hide_origin_at_freeze(&mut overlay, past),
            Err(OverlayError::Ordinal)
        );
        assert_eq!(
            hide_origin_at_freeze(&mut overlay, u64::MAX),
            Err(OverlayError::Ordinal)
        );
        // Over the budget the freeze-time hide is refused atomically.
        let tight = channel_overlay(&base, None)?.used_bytes();
        let mut full = channel_overlay(&base, Some(tight))?;
        assert!(matches!(
            hide_origin_at_freeze(&mut full, key),
            Err(OverlayError::OverBudget { .. })
        ));
        assert!(full.tile(COIN_TILE).is_none());
        assert_eq!(full.alarm_count(), 0);
        Ok(())
    }

    #[test]
    fn map_overlay_pickup_unhides_the_origin_only_on_a_proven_non_commit() -> TestResult {
        let base = base()?;
        let mut overlay = channel_overlay(&base, None)?;
        let key = coin_key()?;
        hide_origin_at_freeze(&mut overlay, key)?;
        let minted = committed(key)?;
        assert!(settle_origin(&mut overlay, key, MintResolution::Unknown)?);
        assert!(settle_origin(
            &mut overlay,
            key,
            MintResolution::Committed(&minted)
        )?);
        assert!(overlay.tile(COIN_TILE).ok_or("tile")?.is_hidden(1));
        assert!(!settle_origin(
            &mut overlay,
            key,
            MintResolution::ProvenNotCommitted
        )?);
        assert!(
            overlay
                .tile(COIN_TILE)
                .is_none_or(|tile| !tile.is_hidden(1))
        );
        // The entry can be frozen again once shown.
        hide_origin_at_freeze(&mut overlay, key)?;
        Ok(())
    }

    #[test]
    fn map_overlay_pickup_rebuild_rehides_every_taken_origin_over_the_budget() -> TestResult {
        let base = base()?;
        let key = coin_key()?;
        let crowded: Vec<u64> = (0..3)
            .map(|ordinal| placement_key(-7, 1, 1, ordinal).ok_or("key"))
            .collect::<Result<_, _>>()?;
        let tight = channel_overlay(&base, None)?.used_bytes();
        let mut overlay = channel_overlay(&base, Some(tight))?;
        let mut taken = vec![key];
        taken.extend(&crowded);
        rehide_taken_origins(&mut overlay, &taken).map_err(|e| format!("{e:?}"))?;
        assert!(overlay.tile(COIN_TILE).ok_or("tile")?.is_hidden(1));
        let tile = overlay
            .tile(TilePos {
                x: 1,
                y: 1,
                floor: -7,
            })
            .ok_or("tile")?;
        assert!((0..3).all(|ordinal| tile.is_hidden(ordinal)));
        assert!(overlay.over_budget());
        assert_eq!(overlay.alarm_count(), 4);
        // Re-running the rebuild leaves every origin hidden.
        rehide_taken_origins(&mut overlay, &taken).map_err(|e| format!("{e:?}"))?;
        assert!(overlay.tile(COIN_TILE).ok_or("tile")?.is_hidden(1));
        // A receipt naming no top-level base entry fails closed.
        let past = placement_key(-7, 2, 1, 2).ok_or("key")?;
        assert_eq!(
            rehide_taken_origins(&mut overlay, &[past]),
            Err(RehideError {
                placement_key: past,
                reason: OverlayError::Ordinal,
            })
        );
        Ok(())
    }
}
