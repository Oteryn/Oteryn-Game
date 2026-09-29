// Shared DUR-03 stage C one-item Ground MINT cases. Both wrappers provide the
// same path-loaded crate root.

use crate::character_recovery_fence::CharacterRecoveryStore;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_mint::{
    CORPSE_MATERIALIZATION_PURPOSE_KEY, CommittedItemMint, GroundPlacement, ItemMintCandidate,
    ItemMintCause, ItemMintError, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};
use crate::durability::item_mint_audit as audit;
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentReceipt, AssignmentRequest, BootstrapSecret,
    ControlActor, LaunchBinding, NodeIncarnationProof, OperationKey, RuntimeScopeAssignmentWriter,
};
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::{
    CarrierError, ChannelId, CombatDeathFixture, CreatureDeathOccurrenceKey, MovementLocalPosition,
    RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId,
};
use sqlx::{Connection, Executor, Row};
use std::future::Future;
use std::task::Poll;
use std::time::{Duration, Instant};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const WORLD: u8 = 42;
const CHANNEL: u8 = 43;

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
        let name = format!("im_{name}_{suffix}");
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
        eprintln!("ITEM-MINT-PG: database {name} on server_version_num={version}");
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
        std::env::temp_dir().join(format!("oteryn-item-mint-parent-{}", std::process::id()));
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
    let launch = LaunchBinding::new(&format!("item-mint-launch-{tag}")).map_err(debug)?;
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

struct Harness {
    database: Database,
    root: DurabilityRoot,
    pool: sqlx::PgPool,
    recovery: CharacterRecoveryStore,
    retained: std::path::PathBuf,
    node: NodeIncarnationProof,
    writer: RuntimeScopeAssignmentWriter,
    assignment: AssignmentReceipt,
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
        let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "item-mint-writer")
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
        Ok(Self {
            database,
            root,
            pool,
            recovery,
            retained,
            node,
            writer,
            assignment,
        })
    }

    async fn count(&self, relation: &str) -> TestResult<i64> {
        Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {relation}"
        )))
        .fetch_one(&self.pool)
        .await?)
    }

    async fn assert_minted(&self, expected: i64) -> TestResult {
        for relation in [
            "game_item_instances",
            "game_item_ground_locations",
            "game_item_mint_receipts",
            "game_item_audit_outbox",
        ] {
            assert_eq!(self.count(relation).await?, expected, "{relation}");
        }
        Ok(())
    }

    async fn cleanup(self) -> TestResult {
        drop(self.writer);
        self.pool.close().await;
        self.database.cleanup().await?;
        std::fs::remove_dir_all(self.retained)?;
        Ok(())
    }
}

fn definition(family: &str, key: &str, revision: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: family.into(),
        production_key: key.into(),
        revision_ref: revision.into(),
    }
}

fn request(generation: u64, actor: u32, draw: u32) -> TestResult<ItemMintRequest> {
    Ok(ItemMintRequest {
        cause: ItemMintCause::for_test(
            WorldId::decode(&id(WORLD)).map_err(debug)?,
            ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
            ScopeOwnershipGeneration::new(generation).map_err(debug)?,
            actor,
            1,
            definition("LootTable", "fixture:loot.alpha", "loot-r1"),
            "fixture:purpose.drop".into(),
            draw,
        ),
        item: definition("ItemType", "fixture:alpha", "rev-a/1"),
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
    })
}

/// Every registered per-field bound at its maximum (DUR03-RL-07).
fn max_request(actor: u32) -> TestResult<ItemMintRequest> {
    let content = |fill: &str| fill.repeat(audit::RL07_CONTENT_KEY_BYTES_MAX);
    let technical = vec![0xa5; audit::RL07_TECHNICAL_FIELD_BYTES_MAX];
    Ok(ItemMintRequest {
        cause: ItemMintCause::for_test(
            WorldId::decode(&id(WORLD)).map_err(debug)?,
            ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
            ScopeOwnershipGeneration::new(1).map_err(debug)?,
            actor,
            u64::MAX,
            definition(&"f".repeat(128), &content("l"), &content("r")),
            content("p"),
            u32::MAX,
        ),
        item: definition(&"g".repeat(128), &content("i"), &content("v")),
        quantity: u32::MAX,
        ground: GroundPlacement {
            spatial_position: technical.clone(),
            corpse_ref: technical.clone(),
            map_revision: content("m"),
            content_revision: content("c"),
            native_room_placement_context: technical,
        },
        content_revision: content("c"),
        ruleset_revision: content("s"),
        sim_revision: content("x"),
    })
}

fn committed(outcome: ItemMintOutcome) -> TestResult<CommittedItemMint> {
    match outcome {
        ItemMintOutcome::Committed(result) => Ok(result),
        other => Err(format!("expected a fresh commit, got {other:?}").into()),
    }
}

fn already(outcome: ItemMintOutcome) -> TestResult<CommittedItemMint> {
    match outcome {
        ItemMintOutcome::AlreadyCommitted(result) => Ok(result),
        other => Err(format!("expected the retained result, got {other:?}").into()),
    }
}

fn assert_candidate_result(candidate: &ItemMintCandidate, result: &CommittedItemMint) {
    assert_eq!(&result.transaction_id, candidate.transaction_id());
    assert_eq!(&result.event_id, candidate.event_id());
    assert_eq!(&result.item_instance_id, candidate.item_instance_id());
    assert_eq!(result.occurred_at_unix_ms, candidate.occurred_at_unix_ms());
}

#[test]
fn mint_commits_once_and_duplicate_cause_returns_identical_result() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "commit").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        let mut candidate = harness
            .root
            .freeze_item_mint(&authority, &harness.node, request(1, 7, 1)?)
            .await
            .map_err(debug)?;
        let result = committed(
            harness
                .root
                .commit_item_mint(&authority, &harness.node, &mut candidate)
                .await
                .map_err(debug)?,
        )?;
        assert_candidate_result(&candidate, &result);
        harness.assert_minted(1).await?;

        // The outbox holds the exact frozen event bytes, P90D expiry, pending.
        let outbox = sqlx::query(
            "SELECT envelope, occurred_at, expires_at, publication_state, event_type_id \
               FROM game_item_audit_outbox",
        )
        .fetch_one(&harness.pool)
        .await?;
        let envelope: Vec<u8> = outbox.try_get("envelope")?;
        assert_eq!(envelope.as_slice(), candidate.envelope());
        assert!(envelope.len() <= audit::RL07_ENVELOPE_BYTES_MAX);
        let (decoded, mint) = audit::decode_envelope(&envelope).map_err(debug)?;
        assert_eq!(decoded.event_id, candidate.event_id().to_vec());
        assert!(decoded.command_id.is_none() && decoded.game_session_id.is_none());
        assert_eq!(
            mint.after
                .as_ref()
                .map(|after| after.item_instance_id.clone()),
            Some(candidate.item_instance_id().to_vec())
        );
        let occurred_at: i64 = outbox.try_get("occurred_at")?;
        assert_eq!(occurred_at, result.occurred_at_unix_ms);
        assert_eq!(
            outbox.try_get::<i64, _>("expires_at")?,
            occurred_at + audit::AUDIT_RETENTION_P90D_MS
        );
        assert_eq!(outbox.try_get::<i16, _>("publication_state")?, 1);
        assert_eq!(outbox.try_get::<i64, _>("event_type_id")?, 2);

        // Exact replay returns the original terminal result. Re-freezing the
        // same cause resumes its durable reservation: the same identities and
        // the same budget, so the third unit is the last one.
        let replay = already(
            harness
                .root
                .commit_item_mint(&authority, &harness.node, &mut candidate)
                .await
                .map_err(debug)?,
        )?;
        assert_eq!(replay, result);
        assert_eq!(candidate.work_units_used(), 2);
        let mut refrozen = harness
            .root
            .freeze_item_mint(&authority, &harness.node, request(1, 7, 1)?)
            .await
            .map_err(debug)?;
        assert_eq!(refrozen.transaction_id(), candidate.transaction_id());
        assert_eq!(refrozen.event_id(), candidate.event_id());
        assert_eq!(refrozen.item_instance_id(), candidate.item_instance_id());
        assert_eq!(
            refrozen.occurred_at_unix_ms(),
            candidate.occurred_at_unix_ms()
        );
        assert_eq!(refrozen.envelope(), candidate.envelope());
        assert_eq!(refrozen.work_units_used(), 2);
        let duplicate = already(
            harness
                .root
                .commit_item_mint(&authority, &harness.node, &mut refrozen)
                .await
                .map_err(debug)?,
        )?;
        assert_eq!(duplicate, result);
        assert_eq!(refrozen.work_units_used(), audit::RL08_RETRY_WORK_UNITS_MAX);
        assert!(matches!(
            harness
                .root
                .reconcile_item_mint(&authority, &mut refrozen)
                .await,
            Err(ItemMintError::CapacityExceeded)
        ));
        assert_eq!(harness.count("game_item_mint_reservations").await?, 1);

        // Same cause, different intent: integrity conflict at the reservation,
        // nothing reserved or minted.
        let mut changed = request(1, 7, 1)?;
        changed.item = definition("ItemType", "fixture:beta", "rev-b/7");
        assert!(matches!(
            harness
                .root
                .freeze_item_mint(&authority, &harness.node, changed)
                .await,
            Err(ItemMintError::ConflictingCause)
        ));
        assert_eq!(harness.count("game_item_mint_reservations").await?, 1);
        harness.assert_minted(1).await?;

        let item = harness
            .root
            .read_item_instance(&authority, result.item_instance_id)
            .await
            .map_err(debug)?
            .ok_or("missing item")?;
        assert_eq!(item.world_id.as_bytes(), &id(WORLD));
        assert_eq!(item.channel_id.as_bytes(), &id(CHANNEL));
        assert_eq!(item.runtime_scope_ownership_generation, 1);
        assert_eq!(
            item.definition,
            definition("ItemType", "fixture:alpha", "rev-a/1")
        );
        assert_eq!(item.quantity, 1);
        assert_eq!(item.minted_transaction_id, result.transaction_id);

        // Every registered field at its maximum commits and reads back intact.
        let mut max = harness
            .root
            .freeze_item_mint(&authority, &harness.node, max_request(9)?)
            .await
            .map_err(debug)?;
        eprintln!(
            "ITEM-MINT-PG: max-bound MINT envelope {} B (registered cap {} B)",
            max.envelope().len(),
            audit::RL07_ENVELOPE_BYTES_MAX
        );
        let max_result = committed(
            harness
                .root
                .commit_item_mint(&authority, &harness.node, &mut max)
                .await
                .map_err(debug)?,
        )?;
        let max_item = harness
            .root
            .read_item_instance(&authority, max_result.item_instance_id)
            .await
            .map_err(debug)?
            .ok_or("missing max item")?;
        assert_eq!(max_item.definition, max_request(9)?.item);
        assert_eq!(max_item.ground, max_request(9)?.ground);
        assert_eq!(max_item.quantity, u32::MAX);
        harness.assert_minted(2).await?;

        // Restart: a fresh root resumes the max cause's reservation and
        // reconciles the same terminal result within its remaining budget.
        let restarted = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let restart_seal = harness.recovery.seal_current().map_err(debug)?;
        let restart_authority = restarted
            .open_character_authority(&restart_seal)
            .await
            .map_err(debug)?;
        let mut after_restart = restarted
            .freeze_item_mint(&restart_authority, &harness.node, max_request(9)?)
            .await
            .map_err(debug)?;
        assert_candidate_result(&after_restart, &max_result);
        assert_eq!(after_restart.work_units_used(), 1);
        assert_eq!(
            restarted
                .reconcile_item_mint(&restart_authority, &mut after_restart)
                .await
                .map_err(debug)?,
            Some(max_result.clone())
        );
        assert_eq!(
            already(
                restarted
                    .commit_item_mint(&restart_authority, &harness.node, &mut after_restart)
                    .await
                    .map_err(debug)?
            )?,
            max_result
        );
        assert_eq!(
            restarted
                .read_item_instance(&restart_authority, result.item_instance_id)
                .await
                .map_err(debug)?,
            Some(item)
        );
        // An authority of another root is refused before any work is charged.
        let mut foreign = restarted
            .freeze_item_mint(&restart_authority, &harness.node, request(1, 7, 2)?)
            .await
            .map_err(debug)?;
        assert!(matches!(
            restarted
                .commit_item_mint(&authority, &harness.node, &mut foreign)
                .await,
            Err(ItemMintError::AuthorityRejected)
        ));
        assert_eq!(foreign.work_units_used(), 0);
        harness.assert_minted(2).await?;
        drop(restart_authority);
        drop(restart_seal);
        drop(restarted);

        // Database-level guards: committed rows are immutable, the audit event
        // and the reservation have no deletion path, a reservation changes
        // only by one work-unit charge up to the maximum, and an ItemInstance
        // cannot commit alone.
        for statement in [
            "UPDATE game_item_mint_reservations SET work_units_used = 0",
            "UPDATE game_item_mint_reservations SET work_units_used = work_units_used + 2",
            "UPDATE game_item_mint_reservations \
             SET work_units_used = work_units_used + 1, draw_ordinal = 9",
            "UPDATE game_item_mint_reservations SET work_units_used = 4",
            "DELETE FROM game_item_mint_reservations",
            "TRUNCATE game_item_mint_reservations CASCADE",
            "UPDATE game_item_instances SET quantity = 2",
            "DELETE FROM game_item_ground_locations",
            "UPDATE game_item_mint_receipts SET draw_ordinal = 9",
            "DELETE FROM game_item_mint_receipts",
            "DELETE FROM game_item_audit_outbox",
            "UPDATE game_item_audit_outbox SET envelope = '\\x00'",
            "TRUNCATE game_item_mint_receipts CASCADE",
        ] {
            assert!(
                sqlx::query(sqlx::AssertSqlSafe(statement))
                    .execute(&harness.pool)
                    .await
                    .is_err(),
                "{statement}"
            );
        }
        let orphan = sqlx::query(
            "INSERT INTO game_item_instances VALUES (encode($1,'hex')::uuid, \
             encode($2,'hex')::uuid, 'ItemType', 'fixture:alpha', 'rev-a/1', 1, 1, \
             encode($3,'hex')::uuid)",
        )
        .bind(id(90).as_slice())
        .bind(id(WORLD).as_slice())
        .bind(id(91).as_slice())
        .execute(&harness.pool)
        .await;
        assert!(
            orphan.is_err(),
            "an ItemInstance without MINT evidence commits"
        );
        harness.assert_minted(2).await?;
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn d52_ended_generation_refuses_and_committed_result_stays() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "generation").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        let mut minted = harness
            .root
            .freeze_item_mint(&authority, &harness.node, request(1, 7, 1)?)
            .await
            .map_err(debug)?;
        let result = committed(
            harness
                .root
                .commit_item_mint(&authority, &harness.node, &mut minted)
                .await
                .map_err(debug)?,
        )?;
        let mut pending = harness
            .root
            .freeze_item_mint(&authority, &harness.node, request(1, 8, 1)?)
            .await
            .map_err(debug)?;
        let pending_transaction = *pending.transaction_id();

        // A death key naming a generation that is not the current assignment,
        // or an unassigned Channel of the same World, is never reserved.
        let mut unassigned = request(1, 7, 1)?;
        unassigned.cause = ItemMintCause::for_test(
            WorldId::decode(&id(WORLD)).map_err(debug)?,
            ChannelId::decode(&id(44)).map_err(debug)?,
            ScopeOwnershipGeneration::new(1).map_err(debug)?,
            7,
            1,
            definition("LootTable", "fixture:loot.alpha", "loot-r1"),
            "fixture:purpose.drop".into(),
            1,
        );
        for refused in [request(2, 7, 1)?, unassigned] {
            assert!(matches!(
                harness
                    .root
                    .freeze_item_mint(&authority, &harness.node, refused)
                    .await,
                Err(ItemMintError::AuthorityRejected)
            ));
        }
        assert_eq!(harness.count("game_item_mint_reservations").await?, 2);
        harness.assert_minted(1).await?;

        // The scope moves to another node: generation 1 ends.
        let node2 = register(&harness.root, 2).await?;
        let moved = harness
            .writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([8_u8; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                command: AssignmentCommand::Replace {
                    scope: scope()?,
                    predecessor: harness.assignment.predecessor(),
                    target: node2.fact(),
                },
            })
            .await
            .map_err(debug)?;
        let AssignmentOutcome::Committed(moved) = moved else {
            return Err(format!("unexpected replacement outcome: {moved:?}").into());
        };
        assert_eq!(moved.assignment.ownership_generation, 2);

        // D52: the uncommitted reservation of generation 1 is refused by the
        // former holder and by the new holder alike, whether it is resumed by
        // a re-freeze or committed from the held candidate. It is never
        // re-reserved with new identities and nothing is minted.
        assert!(matches!(
            harness
                .root
                .commit_item_mint(&authority, &harness.node, &mut pending)
                .await,
            Err(ItemMintError::AuthorityRejected)
        ));
        assert!(matches!(
            harness
                .root
                .commit_item_mint(&authority, &node2, &mut pending)
                .await,
            Err(ItemMintError::AuthorityRejected)
        ));
        for holder in [&harness.node, &node2] {
            assert!(matches!(
                harness
                    .root
                    .freeze_item_mint(&authority, holder, request(1, 8, 1)?)
                    .await,
                Err(ItemMintError::AuthorityRejected)
            ));
        }
        assert_eq!(
            harness
                .root
                .reconcile_item_mint(&authority, &mut pending)
                .await
                .map_err(debug)?,
            None
        );
        assert_eq!(pending.work_units_used(), audit::RL08_RETRY_WORK_UNITS_MAX);
        assert_eq!(harness.count("game_item_mint_reservations").await?, 2);
        let reserved: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_mint_reservations \
              WHERE death_actor_local_id = 8 AND transaction_id = encode($1,'hex')::uuid",
        )
        .bind(pending_transaction.as_slice())
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(reserved, 1);
        harness.assert_minted(1).await?;

        // Whatever committed in generation 1 stays committed: its terminal
        // receipt resumes and returns the identical result on the new holder.
        let mut replay = harness
            .root
            .freeze_item_mint(&authority, &node2, request(1, 7, 1)?)
            .await
            .map_err(debug)?;
        assert_candidate_result(&replay, &result);
        assert_eq!(
            already(
                harness
                    .root
                    .commit_item_mint(&authority, &node2, &mut replay)
                    .await
                    .map_err(debug)?
            )?,
            result
        );

        // Generation 2 reserves and mints only on its current holder.
        assert!(matches!(
            harness
                .root
                .freeze_item_mint(&authority, &harness.node, request(2, 7, 1)?)
                .await,
            Err(ItemMintError::AuthorityRejected)
        ));
        let mut second = harness
            .root
            .freeze_item_mint(&authority, &node2, request(2, 7, 1)?)
            .await
            .map_err(debug)?;
        assert!(matches!(
            harness
                .root
                .commit_item_mint(&authority, &harness.node, &mut second)
                .await,
            Err(ItemMintError::AuthorityRejected)
        ));
        committed(
            harness
                .root
                .commit_item_mint(&authority, &node2, &mut second)
                .await
                .map_err(debug)?,
        )?;
        harness.assert_minted(2).await?;

        // An ended incarnation cannot reserve or mint even in its assigned
        // generation, including a reservation it made while current.
        let mut reserved_before_revoke = harness
            .root
            .freeze_item_mint(&authority, &node2, request(2, 10, 1)?)
            .await
            .map_err(debug)?;
        harness
            .root
            .revoke_node_registration(node2.fact())
            .await
            .map_err(debug)?;
        assert!(matches!(
            harness
                .root
                .freeze_item_mint(&authority, &node2, request(2, 9, 1)?)
                .await,
            Err(ItemMintError::AuthorityRejected)
        ));
        assert!(matches!(
            harness
                .root
                .commit_item_mint(&authority, &node2, &mut reserved_before_revoke)
                .await,
            Err(ItemMintError::AuthorityRejected)
        ));
        harness.assert_minted(2).await?;
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// A deferred trigger holds COMMIT past the client pass deadline, so the
/// client reports an unknown outcome. Inside COMMIT the server transaction
/// timeout is disabled (PostgreSQL 17 arms no separate statement timer when it
/// equals the transaction timeout), so the trigger alone decides the real
/// outcome: it either fails after the client gave up or lets COMMIT succeed.
async fn install_slow_commit(pool: &sqlx::PgPool, then_fail: bool) -> TestResult {
    let outcome = if then_fail {
        "RAISE EXCEPTION 'injected late COMMIT failure';"
    } else {
        "RETURN NULL;"
    };
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "CREATE FUNCTION test_slow_item_commit() RETURNS trigger LANGUAGE plpgsql AS $$ \
         BEGIN PERFORM set_config('transaction_timeout', '0', true); \
         PERFORM pg_sleep(2.3); {outcome} END; $$; \
         CREATE CONSTRAINT TRIGGER test_slow_item_commit AFTER INSERT \
         ON game_item_audit_outbox DEFERRABLE INITIALLY DEFERRED \
         FOR EACH ROW EXECUTE FUNCTION test_slow_item_commit();"
    )))
    .execute(pool)
    .await?;
    Ok(())
}

async fn remove_slow_commit(pool: &sqlx::PgPool) -> TestResult {
    sqlx::raw_sql(
        "DROP TRIGGER test_slow_item_commit ON game_item_audit_outbox; \
         DROP FUNCTION test_slow_item_commit();",
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[test]
fn known_abort_ambiguous_commit_and_lost_ack_reconcile_to_one_result() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "ambiguous").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        // Known abort: the receipt insert fails, nothing commits, and the same
        // frozen candidate (same TransactionId) commits on retry.
        sqlx::raw_sql(
            "CREATE FUNCTION test_reject_item_receipt() RETURNS trigger LANGUAGE plpgsql AS $$ \
             BEGIN RAISE EXCEPTION 'injected receipt failure'; END; $$; \
             CREATE TRIGGER test_reject_item_receipt BEFORE INSERT ON game_item_mint_receipts \
             FOR EACH ROW EXECUTE FUNCTION test_reject_item_receipt();",
        )
        .execute(&harness.pool)
        .await?;
        let mut aborted = harness
            .root
            .freeze_item_mint(&authority, &harness.node, request(1, 7, 1)?)
            .await
            .map_err(debug)?;
        assert!(matches!(
            harness
                .root
                .commit_item_mint(&authority, &harness.node, &mut aborted)
                .await,
            Err(ItemMintError::Unavailable(DurabilityError::Database(_)))
        ));
        sqlx::raw_sql(
            "DROP TRIGGER test_reject_item_receipt ON game_item_mint_receipts; \
             DROP FUNCTION test_reject_item_receipt();",
        )
        .execute(&harness.pool)
        .await?;
        harness.assert_minted(0).await?;
        assert_eq!(
            harness
                .root
                .reconcile_item_mint(&authority, &mut aborted)
                .await
                .map_err(debug)?,
            None
        );
        let retried = committed(
            harness
                .root
                .commit_item_mint(&authority, &harness.node, &mut aborted)
                .await
                .map_err(debug)?,
        )?;
        assert_candidate_result(&aborted, &retried);
        assert_eq!(aborted.work_units_used(), 3);

        // Ambiguous commit, not committed: COMMIT outlives the client pass
        // deadline and then fails server-side. Reconciliation waits on the
        // cause lock, proves non-commit, and the same candidate commits.
        install_slow_commit(&harness.pool, true).await?;
        let mut unknown = harness
            .root
            .freeze_item_mint(&authority, &harness.node, request(1, 8, 1)?)
            .await
            .map_err(debug)?;
        let outcome = harness
            .root
            .commit_item_mint(&authority, &harness.node, &mut unknown)
            .await;
        eprintln!("ITEM-MINT-PG: ambiguous (server-aborted) COMMIT outcome {outcome:?}");
        assert!(matches!(
            outcome,
            Err(ItemMintError::Unavailable(
                DurabilityError::CommitOutcomeUnknown
            ))
        ));
        remove_slow_commit(&harness.pool).await?;
        assert!(harness.root.maintain_ready_once().await?);
        assert_eq!(
            harness
                .root
                .reconcile_item_mint(&authority, &mut unknown)
                .await
                .map_err(debug)?,
            None
        );
        harness.assert_minted(1).await?;
        let resolved = committed(
            harness
                .root
                .commit_item_mint(&authority, &harness.node, &mut unknown)
                .await
                .map_err(debug)?,
        )?;
        assert_candidate_result(&unknown, &resolved);

        // Ambiguous commit, committed: the server finishes COMMIT after the
        // client gave up. Reconciliation returns the frozen candidate's own
        // result and a retry returns it unchanged.
        install_slow_commit(&harness.pool, false).await?;
        let mut late = harness
            .root
            .freeze_item_mint(&authority, &harness.node, request(1, 10, 1)?)
            .await
            .map_err(debug)?;
        let outcome = harness
            .root
            .commit_item_mint(&authority, &harness.node, &mut late)
            .await;
        eprintln!("ITEM-MINT-PG: ambiguous (server-committed) COMMIT outcome {outcome:?}");
        assert!(matches!(
            outcome,
            Err(ItemMintError::Unavailable(
                DurabilityError::CommitOutcomeUnknown
            ))
        ));
        assert!(harness.root.maintain_ready_once().await?);
        let late_result = harness
            .root
            .reconcile_item_mint(&authority, &mut late)
            .await
            .map_err(debug)?
            .ok_or("the late COMMIT was not reconciled")?;
        assert_candidate_result(&late, &late_result);
        remove_slow_commit(&harness.pool).await?;
        assert_eq!(
            already(
                harness
                    .root
                    .commit_item_mint(&authority, &harness.node, &mut late)
                    .await
                    .map_err(debug)?
            )?,
            late_result
        );
        harness.assert_minted(3).await?;

        // Lost acknowledgement: the commit happened but its answer was lost.
        let mut lost = harness
            .root
            .freeze_item_mint(&authority, &harness.node, request(1, 9, 1)?)
            .await
            .map_err(debug)?;
        let first = committed(
            harness
                .root
                .commit_item_mint(&authority, &harness.node, &mut lost)
                .await
                .map_err(debug)?,
        )?;
        assert_eq!(
            harness
                .root
                .reconcile_item_mint(&authority, &mut lost)
                .await
                .map_err(debug)?,
            Some(first.clone())
        );
        assert_eq!(
            already(
                harness
                    .root
                    .commit_item_mint(&authority, &harness.node, &mut lost)
                    .await
                    .map_err(debug)?
            )?,
            first
        );
        harness.assert_minted(4).await?;
        let distinct: i64 = sqlx::query_scalar(
            "SELECT count(DISTINCT transaction_id) FROM game_item_mint_receipts",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(distinct, 4);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn install_receipt_rejection(pool: &sqlx::PgPool) -> TestResult {
    sqlx::raw_sql(
        "CREATE FUNCTION test_reject_item_receipt() RETURNS trigger LANGUAGE plpgsql AS $$ \
         BEGIN RAISE EXCEPTION 'injected receipt failure'; END; $$; \
         CREATE TRIGGER test_reject_item_receipt BEFORE INSERT ON game_item_mint_receipts \
         FOR EACH ROW EXECUTE FUNCTION test_reject_item_receipt();",
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn remove_receipt_rejection(pool: &sqlx::PgPool) -> TestResult {
    sqlx::raw_sql(
        "DROP TRIGGER test_reject_item_receipt ON game_item_mint_receipts; \
         DROP FUNCTION test_reject_item_receipt();",
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn stored_work_units(pool: &sqlx::PgPool, actor: i64) -> TestResult<i16> {
    Ok(sqlx::query_scalar(
        "SELECT work_units_used FROM game_item_mint_reservations \
          WHERE death_actor_local_id = $1",
    )
    .bind(actor)
    .fetch_one(pool)
    .await?)
}

#[test]
fn rl08_budget_is_durable_per_cause_across_refreeze_and_process_restart() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "rl08").await?;

        // Unit 1: a known-abort commit attempt. Unit 2: reconciliation proves
        // that nothing committed. Both are durably charged to the reservation.
        let (transaction_id, event_id, item_instance_id, occurred_at, envelope) = {
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut candidate = harness
                .root
                .freeze_item_mint(&authority, &harness.node, request(1, 7, 1)?)
                .await
                .map_err(debug)?;
            assert_eq!(candidate.work_units_used(), 0);
            install_receipt_rejection(&harness.pool).await?;
            assert!(matches!(
                harness
                    .root
                    .commit_item_mint(&authority, &harness.node, &mut candidate)
                    .await,
                Err(ItemMintError::Unavailable(DurabilityError::Database(_)))
            ));
            remove_receipt_rejection(&harness.pool).await?;
            assert_eq!(
                harness
                    .root
                    .reconcile_item_mint(&authority, &mut candidate)
                    .await
                    .map_err(debug)?,
                None
            );
            assert_eq!(candidate.work_units_used(), 2);
            assert_eq!(stored_work_units(&harness.pool, 7).await?, 2);
            (
                *candidate.transaction_id(),
                *candidate.event_id(),
                *candidate.item_instance_id(),
                candidate.occurred_at_unix_ms(),
                candidate.envelope().to_vec(),
            )
            // The candidate, the authority and the seal are dropped here.
        };

        // Simulated process restart in the same generation: no in-memory
        // candidate survives; a fresh root re-freezes from the cause alone
        // and resumes the same identities, event bytes and budget.
        let restarted = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = restarted
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut resumed = restarted
            .freeze_item_mint(&authority, &harness.node, request(1, 7, 1)?)
            .await
            .map_err(debug)?;
        assert_eq!(resumed.transaction_id(), &transaction_id);
        assert_eq!(resumed.event_id(), &event_id);
        assert_eq!(resumed.item_instance_id(), &item_instance_id);
        assert_eq!(resumed.occurred_at_unix_ms(), occurred_at);
        assert_eq!(resumed.envelope(), envelope.as_slice());
        assert_eq!(resumed.work_units_used(), 2);

        // max=3: the third unit commits under the original TransactionId.
        let result = committed(
            restarted
                .commit_item_mint(&authority, &harness.node, &mut resumed)
                .await
                .map_err(debug)?,
        )?;
        assert_eq!(result.transaction_id, transaction_id);
        assert_eq!(result.item_instance_id, item_instance_id);
        assert_eq!(resumed.work_units_used(), audit::RL08_RETRY_WORK_UNITS_MAX);

        // max+1=4: the fourth unit is rejected, whether it is a commit attempt
        // or a reconciliation, on the held candidate or a re-frozen one.
        assert!(matches!(
            restarted
                .commit_item_mint(&authority, &harness.node, &mut resumed)
                .await,
            Err(ItemMintError::CapacityExceeded)
        ));
        assert!(matches!(
            restarted
                .reconcile_item_mint(&authority, &mut resumed)
                .await,
            Err(ItemMintError::CapacityExceeded)
        ));
        let mut refrozen = restarted
            .freeze_item_mint(&authority, &harness.node, request(1, 7, 1)?)
            .await
            .map_err(debug)?;
        assert_eq!(refrozen.transaction_id(), &transaction_id);
        assert_eq!(refrozen.work_units_used(), audit::RL08_RETRY_WORK_UNITS_MAX);
        assert!(matches!(
            restarted
                .reconcile_item_mint(&authority, &mut refrozen)
                .await,
            Err(ItemMintError::CapacityExceeded)
        ));
        assert!(matches!(
            restarted
                .commit_item_mint(&authority, &harness.node, &mut refrozen)
                .await,
            Err(ItemMintError::CapacityExceeded)
        ));
        assert_eq!(stored_work_units(&harness.pool, 7).await?, 3);

        // An unresolved cause exhausts the same budget across re-freezes:
        // three failed attempts, then the fourth is rejected unexecuted and
        // the cause keeps its original TransactionId unminted.
        install_receipt_rejection(&harness.pool).await?;
        let mut first_holder = restarted
            .freeze_item_mint(&authority, &harness.node, request(1, 8, 1)?)
            .await
            .map_err(debug)?;
        let unresolved = *first_holder.transaction_id();
        for _ in 0..2 {
            assert!(matches!(
                restarted
                    .commit_item_mint(&authority, &harness.node, &mut first_holder)
                    .await,
                Err(ItemMintError::Unavailable(DurabilityError::Database(_)))
            ));
        }
        drop(first_holder);
        let mut second_holder = restarted
            .freeze_item_mint(&authority, &harness.node, request(1, 8, 1)?)
            .await
            .map_err(debug)?;
        assert_eq!(second_holder.transaction_id(), &unresolved);
        assert_eq!(second_holder.work_units_used(), 2);
        assert!(matches!(
            restarted
                .commit_item_mint(&authority, &harness.node, &mut second_holder)
                .await,
            Err(ItemMintError::Unavailable(DurabilityError::Database(_)))
        ));
        remove_receipt_rejection(&harness.pool).await?;
        let mut third_holder = restarted
            .freeze_item_mint(&authority, &harness.node, request(1, 8, 1)?)
            .await
            .map_err(debug)?;
        assert_eq!(third_holder.transaction_id(), &unresolved);
        assert!(matches!(
            restarted
                .commit_item_mint(&authority, &harness.node, &mut third_holder)
                .await,
            Err(ItemMintError::CapacityExceeded)
        ));
        assert_eq!(stored_work_units(&harness.pool, 8).await?, 3);
        assert_eq!(harness.count("game_item_mint_reservations").await?, 2);
        harness.assert_minted(1).await?;
        drop(authority);
        drop(seal);
        drop(restarted);
        harness.cleanup().await
    })
}

/// Result of one charged pass, normalized for counting.
type PassResult = Result<Option<CommittedItemMint>, ItemMintError>;

fn commit_pass(outcome: Result<ItemMintOutcome, ItemMintError>) -> PassResult {
    outcome.map(|outcome| match outcome {
        ItemMintOutcome::Committed(result) | ItemMintOutcome::AlreadyCommitted(result) => {
            Some(result)
        }
    })
}

#[test]
fn rl08_concurrent_freezers_and_passes_share_one_reservation_and_budget() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "budget_race").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let second_root = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
        assert!(second_root.maintain_ready_once().await?);
        let second_authority = second_root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        // Two concurrent freezers of one cause resume one reservation.
        let (first, second) = join_two(
            harness
                .root
                .freeze_item_mint(&authority, &harness.node, request(1, 7, 1)?),
            second_root.freeze_item_mint(&second_authority, &harness.node, request(1, 7, 1)?),
        )
        .await;
        let mut first = first.map_err(debug)?;
        let mut second = second.map_err(debug)?;
        assert_eq!(first.transaction_id(), second.transaction_id());
        assert_eq!(first.event_id(), second.event_id());
        assert_eq!(first.item_instance_id(), second.item_instance_id());
        assert_eq!(first.envelope(), second.envelope());
        assert_eq!(harness.count("game_item_mint_reservations").await?, 1);

        // Each holder races three passes (commit, reconcile, commit): six
        // attempts against one three-unit budget.
        let (first_passes, second_passes) = join_two(
            async {
                let mut passes: Vec<PassResult> = Vec::new();
                passes.push(commit_pass(
                    harness
                        .root
                        .commit_item_mint(&authority, &harness.node, &mut first)
                        .await,
                ));
                passes.push(
                    harness
                        .root
                        .reconcile_item_mint(&authority, &mut first)
                        .await,
                );
                passes.push(commit_pass(
                    harness
                        .root
                        .commit_item_mint(&authority, &harness.node, &mut first)
                        .await,
                ));
                passes
            },
            async {
                let mut passes: Vec<PassResult> = Vec::new();
                passes.push(commit_pass(
                    second_root
                        .commit_item_mint(&second_authority, &harness.node, &mut second)
                        .await,
                ));
                passes.push(
                    second_root
                        .reconcile_item_mint(&second_authority, &mut second)
                        .await,
                );
                passes.push(commit_pass(
                    second_root
                        .commit_item_mint(&second_authority, &harness.node, &mut second)
                        .await,
                ));
                passes
            },
        )
        .await;
        let mut accepted = 0;
        let mut results = Vec::new();
        for pass in first_passes.into_iter().chain(second_passes) {
            match pass {
                Ok(result) => {
                    accepted += 1;
                    results.extend(result);
                }
                Err(ItemMintError::CapacityExceeded) => {}
                Err(error) => return Err(format!("unexpected pass error: {error:?}").into()),
            }
        }
        eprintln!("ITEM-MINT-PG: concurrent passes accepted {accepted} of 6");
        assert_eq!(accepted, i32::from(audit::RL08_RETRY_WORK_UNITS_MAX));
        assert!(!results.is_empty());
        assert!(results.iter().all(|result| *result == results[0]));
        assert_eq!(&results[0].transaction_id, first.transaction_id());
        assert_eq!(stored_work_units(&harness.pool, 7).await?, 3);
        assert_eq!(harness.count("game_item_mint_reservations").await?, 1);
        harness.assert_minted(1).await?;
        drop(second_authority);
        drop(second_root);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

fn summary(samples: &mut [Duration]) -> String {
    samples.sort();
    let micros = |value: Duration| value.as_micros();
    format!(
        "{{\"n\":{},\"min_us\":{},\"median_us\":{},\"p90_us\":{},\"max_us\":{}}}",
        samples.len(),
        micros(samples[0]),
        micros(samples[samples.len() / 2]),
        micros(samples[samples.len() * 9 / 10]),
        micros(samples[samples.len() - 1]),
    )
}

#[test]
fn rl08_reconciliation_work_is_measured_on_real_postgresql() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "measure").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        const SAMPLES: u32 = 30;
        let (mut freeze, mut commit, mut replay, mut reconcile_hit, mut reconcile_miss) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut freeze_resume = Vec::new();
        for draw in 0..SAMPLES {
            let started = Instant::now();
            let mut candidate = harness
                .root
                .freeze_item_mint(&authority, &harness.node, request(1, 7, draw)?)
                .await
                .map_err(debug)?;
            freeze.push(started.elapsed());
            let mut missing = harness
                .root
                .freeze_item_mint(&authority, &harness.node, request(1, 8, draw)?)
                .await
                .map_err(debug)?;
            let started = Instant::now();
            assert_eq!(
                harness
                    .root
                    .reconcile_item_mint(&authority, &mut missing)
                    .await
                    .map_err(debug)?,
                None
            );
            reconcile_miss.push(started.elapsed());
            let started = Instant::now();
            let resumed = harness
                .root
                .freeze_item_mint(&authority, &harness.node, request(1, 8, draw)?)
                .await
                .map_err(debug)?;
            freeze_resume.push(started.elapsed());
            assert_eq!(resumed.transaction_id(), missing.transaction_id());
            assert_eq!(resumed.work_units_used(), 1);
            let started = Instant::now();
            let result = committed(
                harness
                    .root
                    .commit_item_mint(&authority, &harness.node, &mut candidate)
                    .await
                    .map_err(debug)?,
            )?;
            commit.push(started.elapsed());
            let started = Instant::now();
            assert_eq!(
                harness
                    .root
                    .reconcile_item_mint(&authority, &mut candidate)
                    .await
                    .map_err(debug)?,
                Some(result.clone())
            );
            reconcile_hit.push(started.elapsed());
            let started = Instant::now();
            assert_eq!(
                already(
                    harness
                        .root
                        .commit_item_mint(&authority, &harness.node, &mut candidate)
                        .await
                        .map_err(debug)?
                )?,
                result
            );
            replay.push(started.elapsed());
        }
        harness.assert_minted(i64::from(SAMPLES)).await?;
        let all = commit
            .iter()
            .chain(&freeze)
            .chain(&freeze_resume)
            .chain(&replay)
            .chain(&reconcile_hit)
            .chain(&reconcile_miss)
            .max()
            .copied()
            .ok_or("no samples")?;
        assert!(all < crate::durability::DB_PASS_DEADLINE);
        // Each commit/reconcile call is two passes: the committed RL-08
        // charge (3 statements) and then the pass's own work.
        eprintln!(
            "RL08-MEASURE {{\"freeze_new\":{},\"freeze_resume\":{},\"commit_new\":{},\
             \"commit_replay\":{},\"reconcile_committed\":{},\
             \"reconcile_not_committed\":{},\
             \"statements_excluding_begin_commit\":{{\"charge\":3,\"commit_new\":\"3+29\",\
             \"commit_replay\":\"3+21\",\"reconcile\":\"3+4\",\"freeze_new\":8,\
             \"freeze_resume\":7,\"freeze_resume_terminal\":5}},\"pass_deadline_ms\":{}}}",
            summary(&mut freeze),
            summary(&mut freeze_resume),
            summary(&mut commit),
            summary(&mut replay),
            summary(&mut reconcile_hit),
            summary(&mut reconcile_miss),
            crate::durability::DB_PASS_DEADLINE.as_millis(),
        );
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn concurrent_same_cause_on_two_roots_mints_exactly_once() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "concurrent").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let second_root = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
        assert!(second_root.maintain_ready_once().await?);
        let second_authority = second_root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        let mut first = harness
            .root
            .freeze_item_mint(&authority, &harness.node, request(1, 7, 1)?)
            .await
            .map_err(debug)?;
        let mut second = second_root
            .freeze_item_mint(&second_authority, &harness.node, request(1, 7, 1)?)
            .await
            .map_err(debug)?;
        let (first_outcome, second_outcome) = join_two(
            harness
                .root
                .commit_item_mint(&authority, &harness.node, &mut first),
            second_root.commit_item_mint(&second_authority, &harness.node, &mut second),
        )
        .await;
        let results = match (
            first_outcome.map_err(debug)?,
            second_outcome.map_err(debug)?,
        ) {
            (ItemMintOutcome::Committed(winner), ItemMintOutcome::AlreadyCommitted(loser))
            | (ItemMintOutcome::AlreadyCommitted(loser), ItemMintOutcome::Committed(winner)) => {
                (winner, loser)
            }
            outcomes => return Err(format!("unexpected concurrent outcomes: {outcomes:?}").into()),
        };
        assert_eq!(results.0, results.1);
        harness.assert_minted(1).await?;
        drop(second_authority);
        drop(second_root);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

// ---------------------------------------------------------------------------
// Stage D1 (test-only): committed creature death -> one MINT.
//
// The death comes only from the Foundation carrier's committed lethal path and
// Combat's projection (`CombatDeathFixture`); its typed death key is the sole
// death input of `ItemMintCause::from_creature_death`. The loot table is the
// `VSL_COMBAT_FIXTURE_PROFILE` single deterministic entry at draw ordinal 0:
// no probability, quantity rule or production content is decided here, and
// nothing below is reachable from production code.
// ---------------------------------------------------------------------------

const D1_SECOND_CHANNEL: u8 = 44;
const D1_FIXTURE_REVISION: &str = "VSL_COMBAT_FIXTURE_PROFILE/v1";
const D1_LETHAL_OCCURRENCE: &str = "fixture:vsl-combat.strike.lethal";

fn d1_fixture(channel: u8) -> TestResult<CombatDeathFixture> {
    CombatDeathFixture::new(
        WorldId::decode(&id(WORLD)).map_err(debug)?,
        ChannelId::decode(&id(channel)).map_err(debug)?,
        ScopeOwnershipGeneration::new(1).map_err(debug)?,
    )
    .map_err(|error| debug(error).into())
}

/// The one fixture loot entry of a committed death, drawn at ordinal 0. The
/// Ground placement is the corpse position and a fixture encoding of the death
/// key; both are test evidence, not a production placement rule.
fn d1_loot_request(
    death: CreatureDeathOccurrenceKey,
    corpse: MovementLocalPosition,
) -> ItemMintRequest {
    let mut spatial_position = Vec::new();
    spatial_position.extend_from_slice(&corpse.x.to_be_bytes());
    spatial_position.extend_from_slice(&corpse.y.to_be_bytes());
    spatial_position.extend_from_slice(&corpse.floor.to_be_bytes());
    let mut corpse_ref = Vec::new();
    corpse_ref.extend_from_slice(death.world_id().as_bytes());
    corpse_ref.extend_from_slice(death.channel_id().as_bytes());
    corpse_ref.extend_from_slice(&death.scope_ownership_generation().get().to_be_bytes());
    corpse_ref.extend_from_slice(&death.actor_local_id().to_be_bytes());
    corpse_ref.extend_from_slice(&death.actor_local_generation().to_be_bytes());
    ItemMintRequest {
        cause: ItemMintCause::from_creature_death(
            death,
            definition(
                "LootTable",
                "fixture:vsl-combat.loot.single",
                D1_FIXTURE_REVISION,
            ),
            "fixture:vsl-combat.drop".into(),
            0,
        ),
        item: definition("ItemType", "fixture:vsl-combat.item", D1_FIXTURE_REVISION),
        quantity: 1,
        ground: GroundPlacement {
            spatial_position,
            corpse_ref,
            map_revision: "fixture:vsl-combat.map.r1".into(),
            content_revision: D1_FIXTURE_REVISION.into(),
            native_room_placement_context: b"fixture:vsl-combat.room".to_vec(),
        },
        content_revision: D1_FIXTURE_REVISION.into(),
        ruleset_revision: "fixture:vsl-combat.ruleset.r1".into(),
        sim_revision: "fixture:vsl-combat.sim.r1".into(),
    }
}

/// Commits (or identically replays) the fixture's lethal occurrence, projects
/// the death and derives its loot request. Returns the death key with it.
fn d1_kill(
    fixture: &mut CombatDeathFixture,
) -> TestResult<(CreatureDeathOccurrenceKey, ItemMintRequest)> {
    let lethal = fixture
        .strike(D1_LETHAL_OCCURRENCE, CombatDeathFixture::HEALTH)
        .map_err(debug)?;
    assert_eq!(
        (lethal.health_before, lethal.health_after),
        (CombatDeathFixture::HEALTH, 0)
    );
    let (death, corpse) = fixture.project_death().map_err(debug)?;
    Ok((death, d1_loot_request(death, corpse)))
}

/// D1 wire-up: death -> fixture loot entry -> freeze -> commit under the
/// live fence of `node`.
async fn d1_mint_death(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fixture: &mut CombatDeathFixture,
) -> TestResult<(ItemMintCandidate, ItemMintOutcome)> {
    let (_, request) = d1_kill(fixture)?;
    let mut candidate = harness
        .root
        .freeze_item_mint(authority, node, request)
        .await
        .map_err(debug)?;
    let outcome = harness
        .root
        .commit_item_mint(authority, node, &mut candidate)
        .await
        .map_err(debug)?;
    Ok((candidate, outcome))
}

async fn d1_reservations_for(
    harness: &Harness,
    death: CreatureDeathOccurrenceKey,
) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_item_mint_reservations \
          WHERE death_world_id = encode($1,'hex')::uuid \
            AND death_channel_id = encode($2,'hex')::uuid \
            AND death_scope_ownership_generation = $3::text::numeric(20,0) \
            AND death_actor_local_id = $4 \
            AND death_actor_local_generation = $5::text::numeric(20,0) \
            AND loot_table_family = 'LootTable' \
            AND loot_table_production_key = 'fixture:vsl-combat.loot.single' \
            AND loot_table_revision_ref = $6 \
            AND loot_purpose_key = 'fixture:vsl-combat.drop' \
            AND draw_ordinal = 0",
    )
    .bind(death.world_id().as_bytes().as_slice())
    .bind(death.channel_id().as_bytes().as_slice())
    .bind(death.scope_ownership_generation().get().to_string())
    .bind(i64::from(death.actor_local_id()))
    .bind(death.actor_local_generation().to_string())
    .bind(D1_FIXTURE_REVISION)
    .fetch_one(&harness.pool)
    .await?)
}

async fn d1_assert_ground_item(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    result: &CommittedItemMint,
    channel: u8,
) -> TestResult {
    let item = harness
        .root
        .read_item_instance(authority, result.item_instance_id)
        .await
        .map_err(debug)?
        .ok_or("minted item must be readable")?;
    assert_eq!(item.world_id, WorldId::decode(&id(WORLD)).map_err(debug)?);
    assert_eq!(
        item.channel_id,
        ChannelId::decode(&id(channel)).map_err(debug)?
    );
    assert_eq!(item.runtime_scope_ownership_generation, 1);
    assert_eq!(
        item.definition,
        definition("ItemType", "fixture:vsl-combat.item", D1_FIXTURE_REVISION)
    );
    assert_eq!(item.quantity, 1);
    assert_eq!(item.minted_transaction_id, result.transaction_id);
    Ok(())
}

#[test]
fn d1_one_creature_death_mints_exactly_one_item() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "d1_once").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        let mut fixture = d1_fixture(CHANNEL)?;
        let (death, request) = d1_kill(&mut fixture)?;
        assert_eq!(
            death.world_id(),
            WorldId::decode(&id(WORLD)).map_err(debug)?
        );
        assert_eq!(
            death.channel_id(),
            ChannelId::decode(&id(CHANNEL)).map_err(debug)?
        );
        assert_eq!(death.scope_ownership_generation().get(), 1);
        let mut candidate = harness
            .root
            .freeze_item_mint(&authority, &harness.node, request)
            .await
            .map_err(debug)?;
        // The reservation stores the full typed cause tuple of this death.
        assert_eq!(d1_reservations_for(&harness, death).await?, 1);
        let result = committed(
            harness
                .root
                .commit_item_mint(&authority, &harness.node, &mut candidate)
                .await
                .map_err(debug)?,
        )?;
        assert_candidate_result(&candidate, &result);
        harness.assert_minted(1).await?;
        assert_eq!(harness.count("game_item_mint_reservations").await?, 1);
        d1_assert_ground_item(&harness, &authority, &result, CHANNEL).await?;
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn d1_replayed_or_refrozen_death_resolves_to_the_same_item() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "d1_replay").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        let mut fixture = d1_fixture(CHANNEL)?;
        let (death, request) = d1_kill(&mut fixture)?;
        let mut first = harness
            .root
            .freeze_item_mint(&authority, &harness.node, request.clone())
            .await
            .map_err(debug)?;

        // The owner replays the identical lethal occurrence and Combat
        // re-projects: the same death key and the same loot request.
        let (replayed_death, replayed_request) = d1_kill(&mut fixture)?;
        assert_eq!(replayed_death, death);
        assert_eq!(replayed_request, request);
        let mut refrozen = harness
            .root
            .freeze_item_mint(&authority, &harness.node, replayed_request)
            .await
            .map_err(debug)?;
        assert_eq!(refrozen.transaction_id(), first.transaction_id());
        assert_eq!(refrozen.event_id(), first.event_id());
        assert_eq!(refrozen.item_instance_id(), first.item_instance_id());
        assert_eq!(refrozen.envelope(), first.envelope());

        let result = committed(
            harness
                .root
                .commit_item_mint(&authority, &harness.node, &mut first)
                .await
                .map_err(debug)?,
        )?;
        assert_eq!(
            already(
                harness
                    .root
                    .commit_item_mint(&authority, &harness.node, &mut refrozen)
                    .await
                    .map_err(debug)?
            )?,
            result
        );
        assert_eq!(
            harness
                .root
                .reconcile_item_mint(&authority, &mut refrozen)
                .await
                .map_err(debug)?,
            Some(result.clone())
        );
        assert_eq!(d1_reservations_for(&harness, death).await?, 1);
        assert_eq!(harness.count("game_item_mint_reservations").await?, 1);
        harness.assert_minted(1).await?;
        d1_assert_ground_item(&harness, &authority, &result, CHANNEL).await?;
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn d1_stale_generation_death_is_refused_after_the_scope_moves() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "d1_stale").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        let mut fixture = d1_fixture(CHANNEL)?;
        let (death, request) = d1_kill(&mut fixture)?;
        let mut pending = harness
            .root
            .freeze_item_mint(&authority, &harness.node, request.clone())
            .await
            .map_err(debug)?;

        // The scope moves to another node: generation 1 ends before commit.
        let node2 = register(&harness.root, 2).await?;
        let moved = harness
            .writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([8_u8; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                command: AssignmentCommand::Replace {
                    scope: scope()?,
                    predecessor: harness.assignment.predecessor(),
                    target: node2.fact(),
                },
            })
            .await
            .map_err(debug)?;
        let AssignmentOutcome::Committed(moved) = moved else {
            return Err(format!("unexpected replacement outcome: {moved:?}").into());
        };
        assert_eq!(moved.assignment.ownership_generation, 2);

        // D52: the generation-1 death's pending MINT is refused on the former
        // and the new holder, from the held candidate and from a re-freeze.
        for holder in [&harness.node, &node2] {
            assert!(matches!(
                harness
                    .root
                    .commit_item_mint(&authority, holder, &mut pending)
                    .await,
                Err(ItemMintError::AuthorityRejected)
            ));
            assert!(matches!(
                harness
                    .root
                    .freeze_item_mint(&authority, holder, request.clone())
                    .await,
                Err(ItemMintError::AuthorityRejected)
            ));
        }
        // The moved owner cannot re-derive the death from its stale carrier.
        fixture
            .advance_owner(ScopeOwnershipGeneration::new(2).map_err(debug)?)
            .map_err(debug)?;
        assert_eq!(fixture.project_death(), Err(CarrierError::WrongScope));

        assert_eq!(d1_reservations_for(&harness, death).await?, 1);
        harness.assert_minted(0).await?;
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn d1_administrative_despawn_produces_no_death_and_no_mint() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "d1_despawn").await?;

        // Despawn of a living creature, and despawn after a committed lethal
        // hit but before the death is projected: neither yields a death key,
        // so the wire-up has no cause to freeze.
        let mut living = d1_fixture(CHANNEL)?;
        living.despawn().map_err(debug)?;
        assert!(living.project_death().is_err());

        let mut struck = d1_fixture(CHANNEL)?;
        struck
            .strike(D1_LETHAL_OCCURRENCE, CombatDeathFixture::HEALTH)
            .map_err(debug)?;
        struck.despawn().map_err(debug)?;
        assert!(struck.project_death().is_err());
        assert!(d1_kill(&mut struck).is_err());

        assert_eq!(harness.count("game_item_mint_reservations").await?, 0);
        harness.assert_minted(0).await?;
        harness.cleanup().await
    })
}

#[test]
fn d1_two_creature_deaths_have_distinct_keys_and_items() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "d1_two").await?;
        // One fixture creature per Channel carrier: assign a second Channel of
        // the same World to the same node.
        sqlx::query(
            "INSERT INTO game_control_scope_grants \
             (control_role, world_id, channel_id, operation) \
             SELECT session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, operation \
               FROM unnest(ARRAY[1, 2, 3]::SMALLINT[]) AS operation",
        )
        .bind(id(WORLD).as_slice())
        .bind(id(D1_SECOND_CHANNEL).as_slice())
        .execute(&harness.pool)
        .await?;
        let second = harness
            .writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([9_u8; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                command: AssignmentCommand::Assign {
                    scope: RuntimeScopeRefV1::channel(
                        WorldId::decode(&id(WORLD)).map_err(debug)?,
                        ChannelId::decode(&id(D1_SECOND_CHANNEL)).map_err(debug)?,
                    ),
                    target: harness.node.fact(),
                },
            })
            .await
            .map_err(debug)?;
        let AssignmentOutcome::Committed(second) = second else {
            return Err(format!("unexpected assignment outcome: {second:?}").into());
        };
        assert_eq!(second.assignment.ownership_generation, 1);

        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut first_creature = d1_fixture(CHANNEL)?;
        let mut second_creature = d1_fixture(D1_SECOND_CHANNEL)?;
        let first_death = d1_kill(&mut first_creature)?.0;
        let second_death = d1_kill(&mut second_creature)?.0;
        assert_ne!(first_death, second_death);

        let (first_candidate, first_outcome) =
            d1_mint_death(&harness, &authority, &harness.node, &mut first_creature).await?;
        let (second_candidate, second_outcome) =
            d1_mint_death(&harness, &authority, &harness.node, &mut second_creature).await?;
        let first_result = committed(first_outcome)?;
        let second_result = committed(second_outcome)?;
        assert_candidate_result(&first_candidate, &first_result);
        assert_candidate_result(&second_candidate, &second_result);
        assert_ne!(first_result.transaction_id, second_result.transaction_id);
        assert_ne!(first_result.event_id, second_result.event_id);
        assert_ne!(
            first_result.item_instance_id,
            second_result.item_instance_id
        );
        assert_eq!(d1_reservations_for(&harness, first_death).await?, 1);
        assert_eq!(d1_reservations_for(&harness, second_death).await?, 1);
        harness.assert_minted(2).await?;
        d1_assert_ground_item(&harness, &authority, &first_result, CHANNEL).await?;
        d1_assert_ground_item(&harness, &authority, &second_result, D1_SECOND_CHANNEL).await?;
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

// ---------------------------------------------------------------------------
// D3-1: DUR-03 §39.4 "Corpse container amendment (D3)". The corpse's own
// MINT (`commit_corpse_mint`): its two additive receipt columns, the
// deferred `materialized_at` trigger and its narrow immutability exception,
// and the per-scope `oteryn:corpse-cap:` advisory-locked, commit-time
// COMBAT01-CORPSES-PER-SCOPE = 64 recount. The loot-into-corpse-container
// admission gate and its GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX = 16
// ceiling (migration 0013) are proven directly against the schema: no Rust
// freeze/commit caller exists yet for that shape (DUR-03 §39.4 itself defers
// the `OneItemMintV1.destination` proto widening to child D3-6, and the
// composing caller is D3-2's `combat/death_reward.rs`).
// ---------------------------------------------------------------------------

const CORPSE_ACTOR_GENERATION: u64 = 1;

/// Poll every future to completion together (single-threaded interleaving,
/// same posture as `join_two`), for N concurrent corpse-MINT commits racing
/// the same per-scope advisory lock.
async fn join_all<F: Future>(tasks: Vec<F>) -> Vec<F::Output> {
    let mut futures: Vec<_> = tasks.into_iter().map(Box::pin).collect();
    let mut outputs: Vec<Option<F::Output>> = futures.iter().map(|_| None).collect();
    std::future::poll_fn(move |context| {
        let mut all_ready = true;
        for (index, future) in futures.iter_mut().enumerate() {
            if outputs[index].is_none() {
                match future.as_mut().poll(context) {
                    Poll::Ready(value) => outputs[index] = Some(value),
                    Poll::Pending => all_ready = false,
                }
            }
        }
        if all_ready {
            Poll::Ready(
                outputs
                    .iter_mut()
                    .map(|value| value.take().expect("every future polled ready"))
                    .collect(),
            )
        } else {
            Poll::Pending
        }
    })
    .await
}

/// Canonical hyphenated UUID text of `value`, for raw SQL literals.
fn uuid_literal(value: [u8; 16]) -> String {
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

fn corpse_request(actor: u32, top_damage_seed: u8) -> TestResult<(ItemMintRequest, [u8; 16])> {
    Ok((
        ItemMintRequest {
            cause: ItemMintCause::for_test(
                WorldId::decode(&id(WORLD)).map_err(debug)?,
                ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
                ScopeOwnershipGeneration::new(CORPSE_ACTOR_GENERATION).map_err(debug)?,
                actor,
                1,
                definition("LootTable", "fixture:loot.alpha", "loot-r1"),
                CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
                0,
            ),
            item: definition("ItemType", "fixture:corpse.rat", "corpse-r1"),
            quantity: 1,
            ground: GroundPlacement {
                spatial_position: vec![9, 9, 0],
                corpse_ref: id(3).to_vec(),
                map_revision: "map-1".into(),
                content_revision: "content-1".into(),
                native_room_placement_context: id(6).to_vec(),
            },
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
        },
        id(top_damage_seed),
    ))
}

async fn mint_corpse(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    actor: u32,
) -> TestResult<CommittedItemMint> {
    let (request, top_damage) = corpse_request(actor, 150)?;
    let mut candidate = harness
        .root
        .freeze_item_mint(authority, &harness.node, request)
        .await
        .map_err(debug)?;
    let outcome = harness
        .root
        .commit_corpse_mint(authority, &harness.node, &mut candidate, top_damage)
        .await
        .map_err(debug)?;
    committed(outcome)
}

async fn corpse_receipt_row(
    harness: &Harness,
    item_instance_id: [u8; 16],
) -> TestResult<sqlx::postgres::PgRow> {
    Ok(sqlx::query(
        "SELECT corpse_top_damage_character_id::text, materialized_at, loot_purpose_key \
           FROM game_item_mint_receipts WHERE item_instance_id = encode($1,'hex')::uuid",
    )
    .bind(item_instance_id.as_slice())
    .fetch_one(&harness.pool)
    .await?)
}

async fn live_corpse_count(harness: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_item_ground_locations g \
           JOIN game_item_mint_receipts r ON r.item_instance_id = g.item_instance_id \
           JOIN game_item_instances i ON i.item_instance_id = g.item_instance_id \
          WHERE r.loot_purpose_key = $1 AND i.lifecycle = 1",
    )
    .bind(CORPSE_MATERIALIZATION_PURPOSE_KEY)
    .fetch_one(&harness.pool)
    .await?)
}

#[test]
fn corpse_mint_writes_top_damage_and_materialized_at_only_via_trigger() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "corpsemint").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        let (request, top_damage) = corpse_request(1, 150)?;
        let mut candidate = harness
            .root
            .freeze_item_mint(&authority, &harness.node, request)
            .await
            .map_err(debug)?;
        let result = committed(
            harness
                .root
                .commit_corpse_mint(&authority, &harness.node, &mut candidate, top_damage)
                .await
                .map_err(debug)?,
        )?;
        assert_candidate_result(&candidate, &result);
        harness.assert_minted(1).await?;

        let row = corpse_receipt_row(&harness, result.item_instance_id).await?;
        assert_eq!(
            row.try_get::<String, _>("loot_purpose_key")?,
            CORPSE_MATERIALIZATION_PURPOSE_KEY
        );
        assert_eq!(
            row.try_get::<String, _>("corpse_top_damage_character_id")?,
            uuid_literal(top_damage)
        );
        let materialized_at: i64 = row.try_get("materialized_at")?;
        assert!(
            materialized_at > 0,
            "materialized_at must be set by the trigger"
        );
        // The Rust candidate never carries materialized_at at all (it is not
        // a field of ItemMintCandidate/ItemMintRequest); the only way it
        // could have become non-NULL is the deferred
        // game_item_mint_receipt_materialize_corpse trigger.

        // The one-way NULL -> value transition already happened; a further
        // change (even to the same column, even by the privileged test role)
        // is refused: materialized_at is set exactly once, only by the
        // trigger, never again.
        let second_write = sqlx::query(
            "UPDATE game_item_mint_receipts SET materialized_at = materialized_at + 1 \
              WHERE item_instance_id = encode($1,'hex')::uuid",
        )
        .bind(result.item_instance_id.as_slice())
        .execute(&harness.pool)
        .await;
        assert!(
            second_write.is_err(),
            "materialized_at must be immutable once set"
        );
        let unchanged = corpse_receipt_row(&harness, result.item_instance_id).await?;
        assert_eq!(
            unchanged.try_get::<i64, _>("materialized_at")?,
            materialized_at
        );

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn commit_corpse_mint_rejects_a_non_corpse_cause() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "corpsereject").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        let mut candidate = harness
            .root
            .freeze_item_mint(&authority, &harness.node, request(1, 7, 1)?)
            .await
            .map_err(debug)?;
        let outcome = harness
            .root
            .commit_corpse_mint(&authority, &harness.node, &mut candidate, id(150))
            .await;
        assert!(matches!(outcome, Err(ItemMintError::InvalidInput)));
        harness.assert_minted(0).await?;

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// D3 §4.2's required concurrency test: N concurrent corpse-MINT commits
/// against a scope already holding 63 live corpses produce exactly one
/// success and N-1 `CapacityExceeded` refusals, never more than 64 live
/// corpses and never a lost update.
#[test]
fn concurrent_corpse_mints_at_capacity_produce_exactly_one_success() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "corpsecap").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        // Seed the scope to exactly 63 live corpses.
        for actor in 1..=63_u32 {
            mint_corpse(&harness, &authority, actor).await?;
        }
        assert_eq!(live_corpse_count(&harness).await?, 63);

        // N = 3 concurrent corpse-MINT commits, each on its own connection
        // (its own DurabilityRoot), racing for the last of 64 slots.
        const N: u32 = 3;
        let mut roots = Vec::new();
        for _ in 0..N {
            let root = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
            assert!(root.maintain_ready_once().await?);
            roots.push(root);
        }
        let mut authorities = Vec::new();
        for root in &roots {
            authorities.push(root.open_character_authority(&seal).await.map_err(debug)?);
        }
        let mut candidates = Vec::new();
        for (index, root) in roots.iter().enumerate() {
            let (request, top_damage) = corpse_request(64 + index as u32, 151)?;
            let candidate = root
                .freeze_item_mint(&authorities[index], &harness.node, request)
                .await
                .map_err(debug)?;
            candidates.push((candidate, top_damage));
        }

        let mut tasks = Vec::new();
        for ((root, authority), (candidate, top_damage)) in roots
            .iter()
            .zip(authorities.iter())
            .zip(candidates.iter_mut())
        {
            tasks.push(root.commit_corpse_mint(authority, &harness.node, candidate, *top_damage));
        }
        let outcomes = join_all(tasks).await;

        let successes = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, Ok(ItemMintOutcome::Committed(_))))
            .count();
        let capacity_refusals = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, Err(ItemMintError::CapacityExceeded)))
            .count();
        assert_eq!(successes, 1, "outcomes: {outcomes:?}");
        assert_eq!(
            capacity_refusals,
            (N - 1) as usize,
            "outcomes: {outcomes:?}"
        );
        assert_eq!(live_corpse_count(&harness).await?, 64);

        drop(authorities);
        drop(authority);
        drop(seal);
        for root in roots {
            drop(root);
        }
        harness.cleanup().await
    })
}

/// Five raw-SQL statements forging one loot entry's whole logical MINT
/// (reservation, receipt, audit event, item and its corpse container entry),
/// mirroring `insert_mint_with_corpse_attribution`'s container branch
/// exactly, for a death that already committed `corpse` as its own
/// CORPSE_MATERIALIZATION MINT. Test-only: D3-1 ships the admission gate and
/// this is the only way to exercise it before a real Rust caller exists
/// (D3-2/D3-6).
fn forge_corpse_loot_entry_statements(
    world: [u8; 16],
    channel: [u8; 16],
    actor: u32,
    corpse: [u8; 16],
    ordinal: u64,
    seed: u8,
) -> Vec<String> {
    let world_text = uuid_literal(world);
    let channel_text = uuid_literal(channel);
    let corpse_text = uuid_literal(corpse);
    let item = uuid_literal(id(seed));
    let tx = uuid_literal(id(seed.wrapping_add(1)));
    let ev = uuid_literal(id(seed.wrapping_add(2)));
    let node = uuid_literal(id(seed.wrapping_add(3)));
    vec![
        format!(
            "INSERT INTO game_item_mint_reservations \
               (death_world_id, death_channel_id, death_scope_ownership_generation, \
                death_actor_local_id, death_actor_local_generation, loot_table_family, \
                loot_table_production_key, loot_table_revision_ref, loot_purpose_key, \
                draw_ordinal, intent_binding, transaction_id, event_id, item_instance_id, \
                occurred_at, envelope, fence_scope_ownership_generation, fence_holder_node_id, \
                fence_holder_registration_revision, work_units_used, reserved_at) \
             VALUES \
               ('{world_text}', '{channel_text}', {CORPSE_ACTOR_GENERATION}, {actor}, 1, \
                'LootTable', 'fixture:loot.alpha', 'loot-r1', 'fixture:purpose.drop', {ordinal}, \
                decode(repeat('cd',33),'hex'), '{tx}', '{ev}', '{item}', 1000, \
                decode(repeat('ab',16),'hex'), {CORPSE_ACTOR_GENERATION}, '{node}', 0, 0, 900)"
        ),
        format!(
            "INSERT INTO game_item_instances \
               (item_instance_id, world_id, definition_family, definition_production_key, \
                definition_revision_ref, quantity, lifecycle, minted_transaction_id) \
             VALUES ('{item}', '{world_text}', 'ItemType', 'fixture:corpse-loot', 'rev-1', 1, 1, \
                     '{tx}')"
        ),
        format!(
            "INSERT INTO game_item_mint_receipts \
               (death_world_id, death_channel_id, death_scope_ownership_generation, \
                death_actor_local_id, death_actor_local_generation, loot_table_family, \
                loot_table_production_key, loot_table_revision_ref, loot_purpose_key, \
                draw_ordinal, intent_binding, transaction_id, event_id, item_instance_id, \
                occurred_at, envelope_sha256, committed_at, destination_parent_item_instance_id, \
                destination_ordinal) \
             VALUES \
               ('{world_text}', '{channel_text}', {CORPSE_ACTOR_GENERATION}, {actor}, 1, \
                'LootTable', 'fixture:loot.alpha', 'loot-r1', 'fixture:purpose.drop', {ordinal}, \
                decode(repeat('cd',33),'hex'), '{tx}', '{ev}', '{item}', 1000, \
                sha256(decode(repeat('ab',16),'hex')), 1000, '{corpse_text}', {ordinal})"
        ),
        format!(
            "INSERT INTO game_item_audit_outbox \
               (event_id, transaction_id, transaction_ordinal, transaction_count, \
                event_type_id, schema_revision, retention_profile_id, item_instance_id, \
                occurred_at, expires_at, envelope, envelope_sha256, publication_state) \
             VALUES \
               ('{ev}', '{tx}', 1, 1, 2, 1, 'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1', '{item}', \
                1000, 7776001000, decode(repeat('ab',16),'hex'), \
                sha256(decode(repeat('ab',16),'hex')), 1)"
        ),
        format!(
            "INSERT INTO game_item_corpse_container_entries \
               (item_instance_id, world_id, parent_item_instance_id, placement_ordinal, \
                placed_transaction_id) \
             VALUES ('{item}', '{world_text}', '{corpse_text}', {ordinal}, '{tx}')"
        ),
    ]
}

async fn run_forged_entry(harness: &Harness, statements: Vec<String>) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    for statement in &statements {
        sqlx::query(sqlx::AssertSqlSafe(statement.clone()))
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX` = 16 (D131, decision §4.2): the
/// 16th loot entry is admitted, the 17th is rejected by the database, and the
/// parent-live-receipt check (the corpse must carry a live
/// CORPSE_MATERIALIZATION receipt for the same death) is proven throughout.
#[test]
fn corpse_container_entry_enforces_the_capacity_ceiling() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "corpseentries").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        let corpse = mint_corpse(&harness, &authority, 1).await?;

        for ordinal in 1..=16_u64 {
            let seed = 20 + (ordinal as u8) * 4;
            let statements = forge_corpse_loot_entry_statements(
                id(WORLD),
                id(CHANNEL),
                1,
                corpse.item_instance_id,
                ordinal,
                seed,
            );
            run_forged_entry(&harness, statements).await?;
        }
        assert_eq!(
            harness.count("game_item_corpse_container_entries").await?,
            16
        );

        let seed17 = 20 + 17 * 4;
        let statements17 = forge_corpse_loot_entry_statements(
            id(WORLD),
            id(CHANNEL),
            1,
            corpse.item_instance_id,
            17,
            seed17,
        );
        let rejected = run_forged_entry(&harness, statements17).await;
        assert!(
            rejected.is_err(),
            "the 17th corpse container entry must be rejected"
        );
        assert_eq!(
            harness.count("game_item_corpse_container_entries").await?,
            16
        );

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// The container-entry admission gate refuses a loot entry whose parent has
/// no live CORPSE_MATERIALIZATION receipt for the same death (§39.4's
/// parent-live-receipt check), even though every other shape of the forged
/// row is otherwise well-formed.
#[test]
fn corpse_container_entry_requires_a_live_parent_receipt_for_the_same_death() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "corpseparent").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        // An ordinary (non-corpse) Ground item stands in for a "parent" that
        // never materialized a corpse at all.
        let mut plain = harness
            .root
            .freeze_item_mint(&authority, &harness.node, request(1, 9, 1)?)
            .await
            .map_err(debug)?;
        let plain_result = committed(
            harness
                .root
                .commit_item_mint(&authority, &harness.node, &mut plain)
                .await
                .map_err(debug)?,
        )?;

        let statements = forge_corpse_loot_entry_statements(
            id(WORLD),
            id(CHANNEL),
            9,
            plain_result.item_instance_id,
            1,
            60,
        );
        let rejected = run_forged_entry(&harness, statements).await;
        assert!(
            rejected.is_err(),
            "a loot entry must not parent an item with no live corpse receipt"
        );
        assert_eq!(
            harness.count("game_item_corpse_container_entries").await?,
            0
        );

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// A raw forged INSERT of a Ground row for an item, mirroring the shape
/// `insert_mint_with_corpse_attribution`'s Ground branch produces (0010).
fn forge_ground_location_statement(world: [u8; 16], channel: [u8; 16], item: [u8; 16]) -> String {
    format!(
        "INSERT INTO game_item_ground_locations \
           (item_instance_id, world_id, channel_id, runtime_scope_ownership_generation, \
            spatial_position, corpse_ref, map_revision, content_revision, \
            native_room_placement_context) \
         VALUES ('{}', '{}', '{}', {CORPSE_ACTOR_GENERATION}, decode('0909','hex'), \
                 decode(repeat('03',16),'hex'), 'map-1', 'content-1', decode(repeat('06',16),'hex'))",
        uuid_literal(item),
        uuid_literal(world),
        uuid_literal(channel)
    )
}

/// The single-location invariant (a fresh ItemInstance is never both on
/// Ground and a corpse container entry, DUR-03 §39.4), one insertion order:
/// a corpse container entry row is inserted before a forged Ground row for
/// the very same item, in one transaction. `game_item_corpse_container_entry
/// _proven` and the extended `game_item_ground_insertion_guard` are both
/// deferred constraint triggers -- each sees the whole transaction's writes
/// as of commit, regardless of statement order -- so either one's new
/// cross-check independently rejects the whole transaction at commit,
/// including the container entry the same statement batch admitted moments
/// earlier.
#[test]
fn forged_ground_location_for_a_corpse_container_entry_item_is_rejected_at_commit() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "corpsedual1").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        let corpse = mint_corpse(&harness, &authority, 1).await?;
        let seed = 84u8;
        let item = id(seed);
        let mut statements = forge_corpse_loot_entry_statements(
            id(WORLD),
            id(CHANNEL),
            1,
            corpse.item_instance_id,
            1,
            seed,
        );
        statements.push(forge_ground_location_statement(
            id(WORLD),
            id(CHANNEL),
            item,
        ));
        let rejected = run_forged_entry(&harness, statements).await;
        assert!(
            rejected.is_err(),
            "an item already holding a corpse container entry must not also take a Ground location"
        );
        assert_eq!(
            harness.count("game_item_corpse_container_entries").await?,
            0
        );
        assert_eq!(
            harness.count("game_item_ground_locations").await?,
            1,
            "only the corpse's own Ground row from mint_corpse"
        );

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// The reverse insertion order: a forged Ground row for the item is
/// inserted first, then its corpse container entry row, in one transaction.
/// As in the sibling test above, both extended checks are deferred and see
/// the whole transaction's writes as of commit regardless of order, so the
/// transaction is rejected there just the same.
#[test]
fn forged_corpse_container_entry_for_a_ground_item_is_rejected_at_commit() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "corpsedual2").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        let corpse = mint_corpse(&harness, &authority, 1).await?;
        let seed = 88u8;
        let item = id(seed);
        let mut statements = forge_corpse_loot_entry_statements(
            id(WORLD),
            id(CHANNEL),
            1,
            corpse.item_instance_id,
            1,
            seed,
        );
        // Splice the Ground row in right after its item/receipt rows (all
        // three deferred triggers here only ever evaluate at commit, so
        // statement order within the transaction does not itself matter;
        // this ordering states the scenario as "Ground lands first").
        let ground_statement = forge_ground_location_statement(id(WORLD), id(CHANNEL), item);
        statements.insert(3, ground_statement);
        let rejected = run_forged_entry(&harness, statements).await;
        assert!(
            rejected.is_err(),
            "an item already on Ground must not also take a corpse container entry"
        );
        assert_eq!(
            harness.count("game_item_corpse_container_entries").await?,
            0
        );
        assert_eq!(
            harness.count("game_item_ground_locations").await?,
            1,
            "only the corpse's own Ground row from mint_corpse"
        );

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// D132/§4.3: the committed top-damage `CharacterId` is bound to this
/// corpse's frozen intent even though it is only ever supplied at commit. A
/// replay of the same frozen candidate carrying a different declared winner
/// (e.g. a reconnect whose recomputation disagreed with the first attempt)
/// must never silently return the first winner's outcome as its own.
#[test]
fn commit_corpse_mint_rejects_a_replay_with_a_different_top_damage_winner() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "corpsereplay").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        let (request, top_damage) = corpse_request(1, 150)?;
        let mut candidate = harness
            .root
            .freeze_item_mint(&authority, &harness.node, request)
            .await
            .map_err(debug)?;
        let first = committed(
            harness
                .root
                .commit_corpse_mint(&authority, &harness.node, &mut candidate, top_damage)
                .await
                .map_err(debug)?,
        )?;

        let other_winner = id(151);
        let replay = harness
            .root
            .commit_corpse_mint(&authority, &harness.node, &mut candidate, other_winner)
            .await;
        assert!(
            matches!(replay, Err(ItemMintError::ConflictingCause)),
            "{replay:?}"
        );

        // The original winner is still the one durably committed; the
        // rejected replay wrote nothing.
        let row = corpse_receipt_row(&harness, first.item_instance_id).await?;
        assert_eq!(
            row.try_get::<String, _>("corpse_top_damage_character_id")?,
            uuid_literal(top_damage)
        );
        harness.assert_minted(1).await?;

        // The same top-damage value, replayed, still returns the original
        // outcome cleanly -- as the retained AlreadyCommitted result, not a
        // fresh commit.
        let replay_same = harness
            .root
            .commit_corpse_mint(&authority, &harness.node, &mut candidate, top_damage)
            .await
            .map_err(debug)?;
        assert_eq!(already(replay_same)?, first);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// A raw forged INSERT of a `CORPSE_MATERIALIZATION` receipt (Ground-only,
/// mirroring `insert_mint_with_corpse_attribution`'s corpse branch) at a
/// caller-supplied `draw_ordinal`, for the DB-level half of the D3 §4.1
/// draw_ordinal = 0 sentinel check.
fn forge_corpse_receipt_statements(
    world: [u8; 16],
    channel: [u8; 16],
    actor: u32,
    draw_ordinal: u64,
    seed: u8,
) -> Vec<String> {
    let world_text = uuid_literal(world);
    let channel_text = uuid_literal(channel);
    let item = uuid_literal(id(seed));
    let tx = uuid_literal(id(seed.wrapping_add(1)));
    let ev = uuid_literal(id(seed.wrapping_add(2)));
    let top_damage = uuid_literal(id(seed.wrapping_add(3)));
    vec![
        format!(
            "INSERT INTO game_item_instances \
               (item_instance_id, world_id, definition_family, definition_production_key, \
                definition_revision_ref, quantity, lifecycle, minted_transaction_id) \
             VALUES ('{item}', '{world_text}', 'ItemType', 'fixture:corpse.rat', 'corpse-r1', 1, 1, \
                     '{tx}')"
        ),
        format!(
            "INSERT INTO game_item_mint_receipts \
               (death_world_id, death_channel_id, death_scope_ownership_generation, \
                death_actor_local_id, death_actor_local_generation, loot_table_family, \
                loot_table_production_key, loot_table_revision_ref, loot_purpose_key, \
                draw_ordinal, intent_binding, transaction_id, event_id, item_instance_id, \
                occurred_at, envelope_sha256, committed_at, corpse_top_damage_character_id) \
             VALUES \
               ('{world_text}', '{channel_text}', {CORPSE_ACTOR_GENERATION}, {actor}, 1, \
                'LootTable', 'fixture:loot.alpha', 'loot-r1', \
                '{CORPSE_MATERIALIZATION_PURPOSE_KEY}', {draw_ordinal}, \
                decode(repeat('cd',33),'hex'), '{tx}', '{ev}', '{item}', 1000, \
                sha256(decode(repeat('ab',16),'hex')), 1000, '{top_damage}')"
        ),
    ]
}

/// D3 §4.1: the corpse's own MINT is always the reserved draw_ordinal = 0
/// sentinel, enforced at both layers -- `commit_corpse_mint` refuses a
/// non-zero draw_ordinal candidate before ever reaching the database, and
/// the schema's own CHECK refuses a forged CORPSE_MATERIALIZATION receipt at
/// any other ordinal, in case Rust's own check is ever bypassed or wrong.
#[test]
fn corpse_mint_draw_ordinal_must_be_zero() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "corpseordinal").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        let (mut bad_request, top_damage) = corpse_request(1, 150)?;
        bad_request.cause = ItemMintCause::for_test(
            WorldId::decode(&id(WORLD)).map_err(debug)?,
            ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
            ScopeOwnershipGeneration::new(CORPSE_ACTOR_GENERATION).map_err(debug)?,
            1,
            1,
            definition("LootTable", "fixture:loot.alpha", "loot-r1"),
            CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
            1,
        );
        let mut candidate = harness
            .root
            .freeze_item_mint(&authority, &harness.node, bad_request)
            .await
            .map_err(debug)?;
        let outcome = harness
            .root
            .commit_corpse_mint(&authority, &harness.node, &mut candidate, top_damage)
            .await;
        assert!(
            matches!(outcome, Err(ItemMintError::InvalidInput)),
            "{outcome:?}"
        );
        harness.assert_minted(0).await?;

        let statements = forge_corpse_receipt_statements(id(WORLD), id(CHANNEL), 2, 1, 70);
        let rejected = run_forged_entry(&harness, statements).await;
        assert!(
            rejected.is_err(),
            "a CORPSE_MATERIALIZATION receipt with draw_ordinal != 0 must be rejected"
        );
        assert_eq!(harness.count("game_item_mint_receipts").await?, 0);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
