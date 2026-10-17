//! MAIL cases share the canonical target's routed PostgreSQL 17.6 database.
//! Without routing, the harness reports PRE-ROUTING / NONCANONICAL; that path
//! does not qualify PostgreSQL behavior. A configured run executes every body.

use crate::Database;
use crate::bestiary_postgres_harness::{TestResult, configured_admin, runtime};

/// Prove that this database ran the exact standing migration through the real
/// native migrator. No private schema copy can satisfy this checksum match.
async fn require_mail_migration(pool: &sqlx::PgPool) -> TestResult {
    // Observe the actual database before the body; this shared owned check runs
    // inside every MAIL wrapper's existing result/cleanup boundary. No URL or
    // credential is printed, and a failed observation still runs native cleanup.
    let (database, version): (String, String) =
        sqlx::query_as("SELECT current_database(), current_setting('server_version_num')")
            .fetch_one(pool)
            .await?;
    eprintln!("MAIL-NATIVE-PG database={database} server_version_num={version}");
    let expected_checksum = sqlx::migrate!("./migrations")
        .iter()
        .find(|migration| migration.version == 101)
        .map(|migration| migration.checksum.to_vec())
        .ok_or("MAIL requires standing migration 0101 in the actual SQLx migrator")?;
    let installed: Option<(Vec<u8>, bool)> =
        sqlx::query_as("SELECT checksum, success FROM _sqlx_migrations WHERE version = 101")
            .fetch_optional(pool)
            .await?;
    if installed != Some((expected_checksum, true)) {
        return Err("MAIL requires installed migration 0101 with its exact checksum".into());
    }
    Ok(())
}

fn run(tag: &'static str, case: impl AsyncFnOnce(sqlx::PgPool) -> TestResult) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let database = Database::create(admin, tag).await?;
        let result = async {
            let pool = sqlx::PgPool::connect(&database.url).await?;
            let result = async {
                require_mail_migration(&pool).await?;
                case(pool.clone()).await
            }
            .await;
            pool.close().await;
            result
        }
        .await;
        database.cleanup().await?;
        result?;
        eprintln!("native MAIL recipient/text migration0101 PASS: {tag}");
        Ok(())
    })
}

#[test]
fn mail_recipient_same_world_self_and_unknown_are_privacy_safe() -> TestResult {
    run(
        "mail_world",
        crate::durability::mail::tests::mail_recipient_same_world_self_and_unknown_are_privacy_safe,
    )
}

#[test]
fn mail_stale_hint_rechecks_rename_lifecycle_and_world_independently() -> TestResult {
    run(
        "mail_hint",
        crate::durability::mail::tests::mail_stale_hint_rechecks_rename_lifecycle_and_world_independently,
    )
}

#[test]
fn mail_recipient_root_share_lock_blocks_terminal_change_until_transaction_ends() -> TestResult {
    run(
        "mail_lock",
        crate::durability::mail::tests::mail_recipient_root_share_lock_blocks_terminal_change_until_transaction_ends,
    )
}

#[test]
fn text_storage_delete_retains_tombstone_and_rewrite_advances() -> TestResult {
    run(
        "mail_text_delete",
        crate::durability::item_text::tests::text_storage_delete_retains_tombstone_and_rewrite_advances,
    )
}

#[test]
fn text_storage_rate_refusal_preserves_text_and_revision() -> TestResult {
    run(
        "mail_text_rate",
        crate::durability::item_text::tests::text_storage_rate_refusal_preserves_text_and_revision,
    )
}

#[test]
fn text_storage_rollback_loses_neither_atomicity_nor_command_retry() -> TestResult {
    run(
        "mail_text_rollback",
        crate::durability::item_text::tests::text_storage_rollback_loses_neither_atomicity_nor_command_retry,
    )
}

#[path = "character_inbox_reward_claim_postgres_cases.rs"]
mod reward_claim_inbox;

#[test]
fn text_runtime_storage_acl_keeps_receipts_and_live_revision_private() -> TestResult {
    run(
        "mail_text_acl",
        crate::durability::item_text::tests::text_runtime_storage_acl_keeps_receipts_and_live_revision_private,
    )
}

// The retirement tests reuse the already-mounted native MINT/TRANSFER harness.
// Its explicit death/backpack Content fixture is not a MAIL Content admission.
// Text metadata uses the real storage half under the runtime role. Retirement
// itself goes through native ItemUse of the last unit, its receipt and value
// audit; no lifecycle seed, trigger bypass or clock/deadline tampering is used.
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_text::{
    ItemTextWriteRequest, RetiredItemTextCleanup, cleanup_retired_item_text,
};
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass, ItemTransferOutcome};
use crate::durability::item_use::{ItemUseFlaskPlan, ItemUseKind, ItemUseOutcome, ItemUseRequest};
use crate::foundation::WorldId;
use crate::item_transfer_postgres_cases as native_items;

const TEXT_RETIRE_FOOD: &str = "oteryn:item.tibia.i3577";

fn text_retirement_definition() -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: TEXT_RETIRE_FOOD.into(),
        revision_ref: "definition-r1".into(),
    }
}

fn text_retirement_world() -> TestResult<WorldId> {
    WorldId::decode(&native_items::id(native_items::WORLD))
        .map_err(|error| native_items::debug(error).into())
}

async fn text_runtime_tx(
    pool: &sqlx::PgPool,
) -> TestResult<sqlx::Transaction<'static, sqlx::Postgres>> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let role: String = sqlx::query_scalar("SELECT current_user")
        .fetch_one(&mut *tx)
        .await?;
    assert_eq!(role, "oteryn_game_runtime");
    Ok(tx)
}

async fn mint_text_retirement_item(h: &native_items::Harness) -> TestResult<[u8; 16]> {
    let seal = h.recovery.seal_current().map_err(native_items::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(native_items::debug)?;
    let backpack = h.mint(&authority, native_items::BACKPACK, 1).await?;
    assert!(matches!(
        h.transfer(
            &authority,
            native_items::fence()?,
            native_items::to_slot(
                native_items::command(11)?,
                backpack,
                native_items::backpack_facts(),
            ),
        )
        .await
        .map_err(native_items::debug)?,
        ItemTransferOutcome::Committed(_)
    ));
    let item = h
        .mint_definition(&authority, text_retirement_definition(), 1)
        .await?;
    let facts = ItemDefinitionFacts {
        definition: text_retirement_definition(),
        stack: ItemStackClass::Stackable {
            proven_maximum: None,
        },
        container_capacity: None,
        container_slot_equip_pattern: false,
    };
    assert!(matches!(
        h.transfer(
            &authority,
            native_items::fence()?,
            native_items::to_backpack(native_items::command(12)?, item, facts),
        )
        .await
        .map_err(native_items::debug)?,
        ItemTransferOutcome::Committed(_)
    ));
    Ok(item)
}

async fn store_retirement_text(
    h: &native_items::Harness,
    item: [u8; 16],
    command: u64,
    text: &str,
) -> TestResult {
    let request = ItemTextWriteRequest {
        command: native_items::command(command)?,
        item_instance_id: item,
        definition: text_retirement_definition(),
        text: text.into(),
    };
    let outcome = crate::durability::item_text::tests::store_fixture_request(
        &h.pool,
        &native_items::id(native_items::CHARACTER),
        request,
    )
    .await?;
    assert!(outcome.changed);
    Ok(())
}

async fn retire_text_item(h: &native_items::Harness, item: [u8; 16]) -> TestResult {
    let seal = h.recovery.seal_current().map_err(native_items::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(native_items::debug)?;
    let occurred_at: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&h.pool)
            .await?;
    let transaction = native_items::id(210);
    let event = native_items::id(211);
    let outcome = h
        .root
        .commit_item_use(
            &authority,
            &h.node,
            native_items::fence()?,
            ItemUseRequest {
                kind: ItemUseKind::Food,
                command_id: 30,
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
                transaction_id: transaction,
                event_id: event,
                occurred_at_unix_ms: occurred_at,
                server_build_id: "oteryn-game-server-test".into(),
                source_item_instance_id: item,
                source_definition: text_retirement_definition(),
                source_quantity_before: 1,
                flask: ItemUseFlaskPlan::None,
            },
        )
        .await
        .map_err(native_items::debug)?;
    assert!(matches!(outcome, ItemUseOutcome::Written { .. }));
    let state: (i64, i16) = sqlx::query_as(
        "SELECT quantity::bigint,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(item.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(state, (0, 2));
    let proof: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_item_use_records WHERE transaction_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=encode($1,'hex')::uuid AND event_id=encode($2,'hex')::uuid)",
    )
    .bind(transaction.as_slice())
    .bind(event.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(proof, (1, 1));
    Ok(())
}

async fn text_retirement_footprint(
    pool: &sqlx::PgPool,
    item: [u8; 16],
) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_item_texts WHERE item_instance_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_item_text_revisions WHERE item_instance_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_item_text_write_receipts WHERE game_session_id=encode($2,'hex')::uuid)",
    )
    .bind(item.as_slice())
    .bind(native_items::id(native_items::SESSION).as_slice())
    .fetch_one(pool)
    .await?)
}

type TextReceiptSnapshot = (String, Vec<u8>, String, bool, String);

async fn text_retirement_receipts(pool: &sqlx::PgPool) -> TestResult<Vec<TextReceiptSnapshot>> {
    Ok(sqlx::query_as(
        "SELECT command_id::text,request_binding,text_revision::text,changed,written_at::text FROM game_item_text_write_receipts WHERE game_session_id=encode($1,'hex')::uuid ORDER BY command_id",
    )
    .bind(native_items::id(native_items::SESSION).as_slice())
    .fetch_all(pool)
    .await?)
}

fn run_text_retirement(
    tag: &'static str,
    case: impl AsyncFnOnce(&native_items::Harness, [u8; 16]) -> TestResult,
) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = native_items::Harness::create(admin, tag).await?;
        let result = async {
            require_mail_migration(&h.pool).await?;
            let mut connection = h.root.try_acquire_ready().map_err(native_items::debug)?;
            sqlx::query("SET ROLE oteryn_game_runtime")
                .execute(&mut *connection)
                .await?;
            let role: String = sqlx::query_scalar("SELECT current_user")
                .fetch_one(&mut *connection)
                .await?;
            assert_eq!(role, "oteryn_game_runtime");
            reward_claim_inbox_mint::return_connection(connection).await?;
            let item = mint_text_retirement_item(&h).await?;
            case(&h, item).await
        }
        .await;
        h.cleanup().await?;
        result?;
        eprintln!(
            "native MAIL TEXT retirement migration0101 body + cleanup PASS: {tag}; text metadata fixture, no ingress"
        );
        Ok(())
    })
}

#[test]
fn text_retirement_cleanup_requires_actual_retired_same_world_item() -> TestResult {
    run_text_retirement("mail_text_retired_world", async |h, item| {
        store_retirement_text(h, item, 41, "metadata").await?;
        let baseline = text_retirement_footprint(&h.pool, item).await?;
        assert_eq!(baseline, (1, 1, 1));
        let world = text_retirement_world()?;
        let mut tx = text_runtime_tx(&h.pool).await?;
        assert_eq!(
            cleanup_retired_item_text(&mut tx, world, item).await?,
            RetiredItemTextCleanup::NotEligible
        );
        assert_eq!(
            cleanup_retired_item_text(&mut tx, world, native_items::id(250)).await?,
            RetiredItemTextCleanup::NotEligible
        );
        tx.commit().await?;
        assert_eq!(text_retirement_footprint(&h.pool, item).await?, baseline);
        retire_text_item(h, item).await?;
        let mut tx = text_runtime_tx(&h.pool).await?;
        let foreign = WorldId::decode(&native_items::id(249)).map_err(native_items::debug)?;
        assert_eq!(
            cleanup_retired_item_text(&mut tx, foreign, item).await?,
            RetiredItemTextCleanup::NotEligible
        );
        tx.commit().await?;
        assert_eq!(text_retirement_footprint(&h.pool, item).await?, baseline);
        let mut tx = text_runtime_tx(&h.pool).await?;
        assert_eq!(
            cleanup_retired_item_text(&mut tx, world, item).await?,
            RetiredItemTextCleanup::Cleaned {
                text_removed: true,
                revision_removed: true,
            }
        );
        tx.commit().await?;
        assert_eq!(text_retirement_footprint(&h.pool, item).await?, (0, 0, 1));
        Ok(())
    })
}

#[test]
fn text_retirement_cleanup_is_atomic_serialized_and_keeps_command_receipt() -> TestResult {
    run_text_retirement("mail_text_retired_atomic", async |h, item| {
        store_retirement_text(h, item, 41, "metadata").await?;
        retire_text_item(h, item).await?;
        let world = text_retirement_world()?;
        let original = text_retirement_receipts(&h.pool).await?;
        assert_eq!(original.len(), 1);
        let mut first = text_runtime_tx(&h.pool).await?;
        assert_eq!(
            cleanup_retired_item_text(&mut first, world, item).await?,
            RetiredItemTextCleanup::Cleaned {
                text_removed: true,
                revision_removed: true,
            }
        );
        // The first successful call still owns the native item lock and both
        // deletions are uncommitted. A second real TX cannot pass that lock.
        let mut blocked = text_runtime_tx(&h.pool).await?;
        sqlx::query("SET LOCAL lock_timeout='100ms'")
            .execute(&mut *blocked)
            .await?;
        let error = sqlx::query(
            "SELECT * FROM game_item_text_cleanup_retired(encode($1,'hex')::uuid,encode($2,'hex')::uuid)",
        )
        .bind(world.as_bytes().as_slice())
        .bind(item.as_slice())
        .fetch_one(&mut *blocked)
        .await
        .err()
        .ok_or("second text cleanup passed the retained item lock")?;
        assert_eq!(
            error
                .as_database_error()
                .and_then(|error| error.code())
                .as_deref(),
            Some("55P03")
        );
        blocked.rollback().await?;
        first.rollback().await?;
        assert_eq!(text_retirement_footprint(&h.pool, item).await?, (1, 1, 1));
        assert_eq!(text_retirement_receipts(&h.pool).await?, original);
        let mut tx = text_runtime_tx(&h.pool).await?;
        assert_eq!(
            cleanup_retired_item_text(&mut tx, world, item).await?,
            RetiredItemTextCleanup::Cleaned {
                text_removed: true,
                revision_removed: true,
            }
        );
        tx.commit().await?;
        let mut tx = text_runtime_tx(&h.pool).await?;
        assert_eq!(
            cleanup_retired_item_text(&mut tx, world, item).await?,
            RetiredItemTextCleanup::Cleaned {
                text_removed: false,
                revision_removed: false,
            }
        );
        tx.commit().await?;
        assert_eq!(text_retirement_footprint(&h.pool, item).await?, (0, 0, 1));
        assert_eq!(text_retirement_receipts(&h.pool).await?, original);
        Ok(())
    })
}

#[test]
fn text_retirement_cleanup_removes_only_the_retired_empty_text_tombstone() -> TestResult {
    run_text_retirement("mail_text_retired_empty", async |h, item| {
        store_retirement_text(h, item, 41, "metadata").await?;
        store_retirement_text(h, item, 42, "").await?;
        assert_eq!(text_retirement_footprint(&h.pool, item).await?, (0, 1, 2));
        let original = text_retirement_receipts(&h.pool).await?;
        let world = text_retirement_world()?;
        let mut tx = text_runtime_tx(&h.pool).await?;
        assert_eq!(
            cleanup_retired_item_text(&mut tx, world, item).await?,
            RetiredItemTextCleanup::NotEligible
        );
        tx.commit().await?;
        let revision: String = sqlx::query_scalar(
            "SELECT last_revision::text FROM game_item_text_revisions WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(item.as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(revision, "2");
        retire_text_item(h, item).await?;
        let mut tx = text_runtime_tx(&h.pool).await?;
        assert_eq!(
            cleanup_retired_item_text(&mut tx, world, item).await?,
            RetiredItemTextCleanup::Cleaned {
                text_removed: false,
                revision_removed: true,
            }
        );
        tx.commit().await?;
        assert_eq!(text_retirement_footprint(&h.pool, item).await?, (0, 0, 2));
        assert_eq!(text_retirement_receipts(&h.pool).await?, original);
        Ok(())
    })
}

#[path = "character_inbox_reward_claim_mint_postgres_cases.rs"]
mod reward_claim_inbox_mint;

// D1377c: native history only. Each item below is minted/transferred by the
// existing native harness; TEXT metadata is written by the real private storage
// half under runtime role. This does not install the held TEXT ingress, provide
// qualified letter Content, or turn a historical outcome into current authority.
use crate::durability::DurabilityRoot;
use crate::durability::item_text::{
    FrozenItemTextWriteBinding, ItemTextWriteError, ItemTextWriteOutcome,
};
use std::future::{Future, poll_fn};
use std::pin::{Pin, pin};
use std::task::Poll;
use std::time::Duration;

fn text_history_request(
    item: [u8; 16],
    command: u64,
    text: &str,
) -> TestResult<ItemTextWriteRequest> {
    Ok(ItemTextWriteRequest {
        command: native_items::command(command)?,
        item_instance_id: item,
        definition: text_retirement_definition(),
        text: text.into(),
    })
}

fn text_history_binding(request: &ItemTextWriteRequest) -> TestResult<FrozenItemTextWriteBinding> {
    Ok(request.freeze_receipt_binding(native_items::fence()?.character_id))
}

async fn text_history_runtime_root(root: &DurabilityRoot) -> TestResult {
    let mut connection = root.try_acquire_ready().map_err(native_items::debug)?;
    sqlx::query("SET ROLE oteryn_game_runtime")
        .execute(&mut *connection)
        .await?;
    let role: String = sqlx::query_scalar("SELECT current_user")
        .fetch_one(&mut *connection)
        .await?;
    assert_eq!(role, "oteryn_game_runtime");
    reward_claim_inbox_mint::return_connection(connection).await
}

async fn text_history_snapshot_fixture(root: &DurabilityRoot, isolation: &str) -> TestResult {
    let mut connection = root.try_acquire_ready().map_err(native_items::debug)?;
    sqlx::query("SELECT set_config('default_transaction_isolation', $1, false)")
        .bind(isolation)
        .execute(&mut *connection)
        .await?;
    reward_claim_inbox_mint::return_connection(connection).await
}

fn run_text_history(
    tag: &'static str,
    case: impl AsyncFnOnce(&native_items::Harness, [u8; 16]) -> TestResult,
) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = native_items::Harness::create(admin, tag).await?;
        let result = async {
            require_mail_migration(&h.pool).await?;
            let version: String = sqlx::query_scalar("SHOW server_version_num")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(version, "170006", "native PostgreSQL 17.6 required");
            text_history_runtime_root(&h.root).await?;
            let item = mint_text_retirement_item(&h).await?;
            case(&h, item).await
        }
        .await;
        h.cleanup().await?;
        result?;
        eprintln!(
            "native MAIL TEXT history migration0101 body + cleanup PASS: {tag}; storage metadata fixture, no ingress"
        );
        Ok(())
    })
}

async fn stage_text_history(
    h: &native_items::Harness,
    request: ItemTextWriteRequest,
) -> TestResult<(
    sqlx::Transaction<'static, sqlx::Postgres>,
    ItemTextWriteOutcome,
)> {
    crate::durability::item_text::tests::stage_history_fixture_request(
        &h.pool,
        &native_items::id(native_items::CHARACTER),
        request,
    )
    .await
}

/// Observe the native reader waiting on this exact physical transaction's
/// advisory key. Poll the real future while observing PostgreSQL; elapsed time
/// alone, an unpolled future, or a lock in another database cannot pass this case.
async fn wait_text_history_command_lock<F: Future>(
    mut reading: Pin<&mut F>,
    pool: &sqlx::PgPool,
    blocker_pid: i32,
) -> TestResult {
    loop {
        let mut blocked = pin!(
            sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity a JOIN pg_locks l ON l.pid=a.pid \
                 WHERE a.datname=current_database() AND a.wait_event_type='Lock' \
                 AND l.locktype='advisory' AND NOT l.granted \
                 AND $1=ANY(pg_blocking_pids(a.pid)))",
            )
            .bind(blocker_pid)
            .fetch_one(pool)
        );
        let observed = poll_fn(|context| {
            if reading.as_mut().poll(context).is_ready() {
                return Poll::Ready(Err::<bool, Box<dyn std::error::Error>>(
                    "TEXT history returned before the original transaction released its command lock"
                        .into(),
                ));
            }
            blocked
                .as_mut()
                .poll(context)
                .map(|result| result.map_err(Into::into))
        })
        .await?;
        if observed {
            return Ok(());
        }
        tokio::task::yield_now().await;
    }
}

#[test]
fn text_history_distinguishes_absent_unchanged_and_conflicting_original_bindings() -> TestResult {
    run_text_history("mail_text_history_binding", async |h, item| {
        let seal = h.recovery.seal_current().map_err(native_items::debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(native_items::debug)?;
        let empty = text_history_request(item, 40, "")?;
        let empty_binding = text_history_binding(&empty)?;
        assert_eq!(
            h.root
                .reconcile_item_text_rewrite(&authority, &empty_binding)
                .await?,
            None
        );
        let (tx, unchanged) = stage_text_history(h, empty).await?;
        assert_eq!(
            unchanged,
            ItemTextWriteOutcome {
                text_revision: 0,
                changed: false,
                replayed: false,
            }
        );
        tx.commit().await?;
        assert_eq!(
            h.root
                .reconcile_item_text_rewrite(&authority, &empty_binding)
                .await?,
            Some(ItemTextWriteOutcome {
                replayed: true,
                ..unchanged
            })
        );
        let original = text_history_request(item, 41, "original private metadata")?;
        let frozen = text_history_binding(&original)?;
        let (tx, written) = stage_text_history(h, original).await?;
        assert_eq!(written.text_revision, 1);
        assert!(written.changed);
        tx.commit().await?;
        let receipts = text_retirement_receipts(&h.pool).await?;
        let footprint = text_retirement_footprint(&h.pool, item).await?;
        assert_eq!(footprint, (1, 1, 2));
        let mut changed_item = text_history_request(item, 41, "original private metadata")?;
        changed_item.item_instance_id = native_items::id(249);
        let mut changed_definition = text_history_request(item, 41, "original private metadata")?;
        changed_definition.definition.revision_ref = "different-definition".into();
        let foreign_character = crate::domain::CharacterId::from_bytes(native_items::id(250))
            .map_err(native_items::debug)?;
        for conflict in [
            text_history_binding(&text_history_request(item, 41, "changed private metadata")?)?,
            text_history_binding(&changed_item)?,
            text_history_binding(&changed_definition)?,
            text_history_request(item, 41, "original private metadata")?
                .freeze_receipt_binding(foreign_character),
        ] {
            assert!(matches!(
                h.root
                    .reconcile_item_text_rewrite(&authority, &conflict)
                    .await,
                Err(ItemTextWriteError::ConflictingCommand)
            ));
        }
        let other_command = text_history_binding(&text_history_request(item, 42, "later")?)?;
        assert_eq!(
            h.root
                .reconcile_item_text_rewrite(&authority, &other_command)
                .await?,
            None
        );
        assert_eq!(
            h.root
                .reconcile_item_text_rewrite(&authority, &frozen)
                .await?,
            Some(ItemTextWriteOutcome {
                replayed: true,
                ..written
            })
        );
        assert_eq!(text_retirement_receipts(&h.pool).await?, receipts);
        assert_eq!(text_retirement_footprint(&h.pool, item).await?, footprint);
        Ok(())
    })
}

#[test]
fn text_history_waits_for_the_original_command_commit_before_returning_receipt() -> TestResult {
    run_text_history("mail_text_history_commit", async |h, item| {
        let seal = h.recovery.seal_current().map_err(native_items::debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(native_items::debug)?;
        let original = text_history_request(item, 41, "pending commit metadata")?;
        let frozen = text_history_binding(&original)?;
        let (mut tx, written) = stage_text_history(h, original).await?;
        let blocker: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *tx)
            .await?;
        assert_eq!(text_retirement_footprint(&h.pool, item).await?, (0, 0, 0));
        // A real nondefault Root connection must refuse rather than interpret
        // a snapshot taken before waiting on this transaction. Restore the
        // ordinary profile on the same observed holder, then test the wait.
        text_history_snapshot_fixture(&h.root, "repeatable read").await?;
        assert!(matches!(
            h.root
                .reconcile_item_text_rewrite(&authority, &frozen)
                .await,
            Err(ItemTextWriteError::Unavailable(_))
        ));
        text_history_snapshot_fixture(&h.root, "read committed").await?;
        let mut reading = Box::pin(h.root.reconcile_item_text_rewrite(&authority, &frozen));
        tokio::time::timeout(
            Duration::from_secs(5),
            wait_text_history_command_lock(reading.as_mut(), &h.pool, blocker),
        )
        .await??;
        // The physical COMMIT is real. This test coordinates its completion;
        // it does not claim to inject a lost network acknowledgement.
        tx.commit().await?;
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(5), reading).await??,
            Some(ItemTextWriteOutcome {
                replayed: true,
                ..written
            })
        );
        let receipts = text_retirement_receipts(&h.pool).await?;
        assert_eq!(receipts.len(), 1);
        assert_eq!(text_retirement_footprint(&h.pool, item).await?, (1, 1, 1));
        assert_eq!(
            h.root
                .reconcile_item_text_rewrite(&authority, &frozen)
                .await?,
            Some(ItemTextWriteOutcome {
                replayed: true,
                ..written
            })
        );
        assert_eq!(text_retirement_receipts(&h.pool).await?, receipts);
        Ok(())
    })
}

#[test]
fn text_history_waits_for_rollback_and_keeps_the_exact_request_retryable() -> TestResult {
    run_text_history("mail_text_history_rollback", async |h, item| {
        let seal = h.recovery.seal_current().map_err(native_items::debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(native_items::debug)?;
        let original = text_history_request(item, 41, "pending rollback metadata")?;
        let frozen = text_history_binding(&original)?;
        let (mut tx, rolled_back) = stage_text_history(h, original).await?;
        assert!(rolled_back.changed);
        let blocker: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *tx)
            .await?;
        let mut reading = Box::pin(h.root.reconcile_item_text_rewrite(&authority, &frozen));
        tokio::time::timeout(
            Duration::from_secs(5),
            wait_text_history_command_lock(reading.as_mut(), &h.pool, blocker),
        )
        .await??;
        tx.rollback().await?;
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(5), reading).await??,
            None
        );
        assert_eq!(text_retirement_footprint(&h.pool, item).await?, (0, 0, 0));
        let (tx, retry) = stage_text_history(
            h,
            text_history_request(item, 41, "pending rollback metadata")?,
        )
        .await?;
        tx.commit().await?;
        assert_eq!(retry, rolled_back);
        assert_eq!(
            h.root
                .reconcile_item_text_rewrite(&authority, &frozen)
                .await?,
            Some(ItemTextWriteOutcome {
                replayed: true,
                ..retry
            })
        );
        assert_eq!(text_retirement_footprint(&h.pool, item).await?, (1, 1, 1));
        Ok(())
    })
}

#[test]
fn text_history_survives_real_retirement_with_fresh_authority_on_a_new_root() -> TestResult {
    run_text_history("mail_text_history_new_root", async |h, item| {
        let original = text_history_request(item, 41, "retained history metadata")?;
        let frozen = text_history_binding(&original)?;
        let (tx, written) = stage_text_history(h, original).await?;
        tx.commit().await?;
        retire_text_item(h, item).await?;
        let mut cleanup = text_runtime_tx(&h.pool).await?;
        assert_eq!(
            cleanup_retired_item_text(&mut cleanup, text_retirement_world()?, item).await?,
            RetiredItemTextCleanup::Cleaned {
                text_removed: true,
                revision_removed: true,
            }
        );
        cleanup.commit().await?;
        assert_eq!(text_retirement_footprint(&h.pool, item).await?, (0, 0, 1));
        let receipts = text_retirement_receipts(&h.pool).await?;
        let seal = h.recovery.seal_current().map_err(native_items::debug)?;
        let old_root_authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(native_items::debug)?;
        let new_root = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(new_root.maintain_ready_once().await?);
        text_history_runtime_root(&new_root).await?;
        // Even the same physical database does not make the old Root's issued
        // capability valid on this new actual Root instance.
        assert!(matches!(
            new_root
                .reconcile_item_text_rewrite(&old_root_authority, &frozen)
                .await,
            Err(ItemTextWriteError::AuthorityRejected)
        ));
        let new_root_authority = new_root
            .open_character_authority(&seal)
            .await
            .map_err(native_items::debug)?;
        assert_eq!(
            new_root
                .reconcile_item_text_rewrite(&new_root_authority, &frozen)
                .await?,
            Some(ItemTextWriteOutcome {
                replayed: true,
                ..written
            })
        );
        assert_eq!(text_retirement_receipts(&h.pool).await?, receipts);
        assert_eq!(text_retirement_footprint(&h.pool, item).await?, (0, 0, 1));
        Ok(())
    })
}

#[test]
fn text_history_stale_database_recovery_exposes_neither_receipt_conflict_nor_absence() -> TestResult
{
    run_text_history("mail_text_history_recovery", async |h, item| {
        let original = text_history_request(item, 41, "recovery protected metadata")?;
        let committed = text_history_binding(&original)?;
        let (tx, _) = stage_text_history(h, original).await?;
        tx.commit().await?;
        let conflicting = text_history_binding(&text_history_request(item, 41, "different")?)?;
        let absent = text_history_binding(&text_history_request(item, 42, "new")?)?;
        let receipts = text_retirement_receipts(&h.pool).await?;
        let seal = h.recovery.seal_current().map_err(native_items::debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(native_items::debug)?;
        let predecessor = crate::character_recovery_fence::recovery_record_digest(seal.record())
            .map_err(native_items::debug)?;
        // Deliberate recovery mismatch only: the real external generation-one
        // seal stays held while the test's administrative connection advances
        // the database admission. This does not seed item value or TEXT history,
        // disable a guard, or impersonate a production recovery publisher.
        sqlx::query("INSERT INTO game_character_recovery_admissions(authority_scope_id,recovery_generation,recovery_event_id,predecessor_generation,predecessor_digest,issued_at,issuer_identity,reconciled_at) VALUES('character-primary',2,encode($1,'hex')::uuid,1,$2,200,'game-ops',200)")
            .bind(native_items::id(12).as_slice()).bind(predecessor.as_slice())
            .execute(&h.pool).await?;
        for binding in [committed, conflicting, absent] {
            assert!(matches!(
                h.root
                    .reconcile_item_text_rewrite(&authority, &binding)
                    .await,
                Err(ItemTextWriteError::Unavailable(_))
            ));
        }
        assert_eq!(text_retirement_receipts(&h.pool).await?, receipts);
        assert_eq!(text_retirement_footprint(&h.pool, item).await?, (1, 1, 1));
        Ok(())
    })
}
