//! Storage preparation only: native MINT/TRANSFER, installed migration and real
//! PostgreSQL guards. No fixture disables constraints/triggers. Only the admin
//! seeds imbuements; the runtime must remain unable to apply/checkpoint/clear.

use crate::item_transfer_postgres_cases::{
    BACKPACK, Harness, STONE, TestResult, backpack_facts, command, configured_admin, debug,
    definition, facts, fence, id, runtime, to_backpack, to_slot,
};
use oteryn_gameplay_domain::domain::currency::Coin;
use oteryn_gameplay_domain::domain::item_definition::ItemStackClass;
use oteryn_gameplay_durability::durability::imbuement::IMBUEMENT_MIGRATION_VERSION;
use oteryn_gameplay_durability::durability::npc_trade::{
    NpcTradeCause, NpcTradeCoins, NpcTradeError, NpcTradeRequest, NpcTradeSide,
};
use oteryn_gameplay_items::imbuement::{IMBUEMENT_DURATION_MS, IMBUEMENT_TIER_RULES};
use sqlx::Row;

const INSERT_SLOT: &str = "INSERT INTO game_item_imbuements \
    (item_instance_id, slot, category, imbuement_key, imbuement_revision, tier, \
     remaining_ms, applying_occurrence_id, applying_transaction_id) \
    VALUES (encode($1,'hex')::uuid, $2, $3, 'oteryn:imbuement.void', 'imbue-r1', $4, \
            $5, encode($6,'hex')::uuid, encode($7,'hex')::uuid)";

fn sqlstate(error: sqlx::Error, expected: &str) -> TestResult {
    let sqlx::Error::Database(error) = error else {
        return Err(format!("expected database rejection, got {error:?}").into());
    };
    assert_eq!(error.code().as_deref(), Some(expected), "{error}");
    Ok(())
}

async fn snapshot(pool: &sqlx::PgPool, item: [u8; 16]) -> TestResult<Vec<String>> {
    // The primary key and slot CHECK bound this complete snapshot to three rows.
    Ok(sqlx::query_scalar(
        "SELECT to_jsonb(b)::text FROM game_item_imbuements b \
         WHERE item_instance_id = encode($1,'hex')::uuid ORDER BY slot",
    )
    .bind(item.as_slice())
    .fetch_all(pool)
    .await?)
}

#[test]
fn imbuement_postgres_storage_bounds_bindings_and_restart() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "imbuement_storage").await?;
        let result: TestResult = async {
            let installed: bool =
                sqlx::query_scalar("SELECT success FROM _sqlx_migrations WHERE version = $1")
                    .bind(IMBUEMENT_MIGRATION_VERSION)
                    .fetch_one(&harness.pool)
                    .await?;
            assert!(installed);
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let item = harness.mint(&authority, STONE, 1).await?;
            for (slot, rules) in IMBUEMENT_TIER_RULES.iter().enumerate() {
                sqlx::query(INSERT_SLOT)
                    .bind(item.as_slice())
                    .bind(i16::try_from(slot)?)
                    .bind(i16::try_from(slot)?)
                    .bind(i16::from(rules.tier as u8))
                    .bind(i64::try_from(IMBUEMENT_DURATION_MS)?)
                    .bind(id(100 + u8::try_from(slot)?).to_vec())
                    .bind(id(110 + u8::try_from(slot)?).to_vec())
                    .execute(&harness.pool)
                    .await?;
            }
            // Each invalid input changes just one fact on an otherwise valid row.
            let other = harness.mint(&authority, STONE, 1).await?;
            for (slot, category, tier, remaining, occurrence, transaction, code) in [
                (
                    -1_i16,
                    0_i16,
                    1_i16,
                    72_000_000_i64,
                    id(120),
                    id(130),
                    "23514",
                ),
                (3, 0, 1, 72_000_000, id(120), id(130), "23514"),
                (0, -1, 1, 72_000_000, id(120), id(130), "23514"),
                (0, 20, 1, 72_000_000, id(120), id(130), "23514"),
                (0, 0, 0, 72_000_000, id(120), id(130), "23514"),
                (0, 0, 4, 72_000_000, id(120), id(130), "23514"),
                (0, 0, 1, -1, id(120), id(130), "23514"),
                (0, 0, 1, 72_000_001, id(120), id(130), "23514"),
                (0, 0, 1, 0, id(120), id(130), "23514"),
                (0, 0, 1, 72_000_000, [0; 16], id(130), "23514"),
                (0, 0, 1, 72_000_000, id(120), [0; 16], "23514"),
                (0, 0, 1, 72_000_000, id(100), id(130), "23505"),
                (0, 0, 1, 72_000_000, id(120), id(110), "23505"),
            ] {
                let error = sqlx::query(INSERT_SLOT)
                    .bind(other.as_slice())
                    .bind(slot)
                    .bind(category)
                    .bind(tier)
                    .bind(remaining)
                    .bind(occurrence.as_slice())
                    .bind(transaction.as_slice())
                    .execute(&harness.pool)
                    .await
                    .err()
                    .ok_or("invalid imbuement must reject")?;
                sqlstate(error, code)?;
            }
            sqlx::query(INSERT_SLOT)
                .bind(other.as_slice())
                .bind(0_i16)
                .bind(0_i16)
                .bind(1_i16)
                .bind(72_000_000_i64)
                .bind(id(120).to_vec())
                .bind(id(130).to_vec())
                .execute(&harness.pool)
                .await?;
            sqlx::query(
                "UPDATE game_item_imbuements SET remaining_ms = 0 \
                         WHERE item_instance_id = encode($1,'hex')::uuid",
            )
            .bind(other.as_slice())
            .execute(&harness.pool)
            .await?;
            // Slot 1 is vacant: only the retained category can reject this row.
            let error = sqlx::query(INSERT_SLOT)
                .bind(other.as_slice())
                .bind(1_i16)
                .bind(0_i16)
                .bind(1_i16)
                .bind(72_000_000_i64)
                .bind(id(121).to_vec())
                .bind(id(131).to_vec())
                .execute(&harness.pool)
                .await
                .err()
                .ok_or("category remains occupied at zero")?;
            sqlstate(error, "23505")?;
            for (column, value) in [
                ("item_instance_id", "encode($2,'hex')::uuid"),
                ("slot", "2"),
                ("category", "4"),
                ("imbuement_key", "'oteryn:imbuement.strike'"),
                ("imbuement_revision", "'imbue-r2'"),
                ("tier", "2"),
                ("applying_occurrence_id", "encode($2,'hex')::uuid"),
                ("applying_transaction_id", "encode($2,'hex')::uuid"),
            ] {
                let statement = sqlx::AssertSqlSafe(format!(
                    "UPDATE game_item_imbuements SET {column} = {value} \
                     WHERE item_instance_id = encode($1,'hex')::uuid AND slot = 0 \
                     AND octet_length($2::bytea) = 16"
                ));
                let error = sqlx::query(statement)
                    .bind(item.as_slice())
                    .bind(other.as_slice())
                    .execute(&harness.pool)
                    .await
                    .err()
                    .ok_or("binding changes must reject")?;
                sqlstate(error, "23514")?;
            }
            sqlx::query(
                "UPDATE game_item_imbuements SET remaining_ms = 60000 \
                         WHERE item_instance_id = encode($1,'hex')::uuid AND slot = 0",
            )
            .bind(item.as_slice())
            .execute(&harness.pool)
            .await?;
            let error = sqlx::query(
                "UPDATE game_item_imbuements SET remaining_ms = 60001 \
                                    WHERE item_instance_id = encode($1,'hex')::uuid AND slot = 0",
            )
            .bind(item.as_slice())
            .execute(&harness.pool)
            .await
            .err()
            .ok_or("time cannot increase")?;
            sqlstate(error, "23514")?;
            sqlx::query(
                "UPDATE game_item_imbuements SET remaining_ms = 0 \
                         WHERE item_instance_id = encode($1,'hex')::uuid AND slot = 0",
            )
            .bind(item.as_slice())
            .execute(&harness.pool)
            .await?;
            let error = sqlx::query(
                "DELETE FROM game_item_imbuements \
                                    WHERE item_instance_id = encode($1,'hex')::uuid AND slot = 0",
            )
            .bind(item.as_slice())
            .execute(&harness.pool)
            .await
            .err()
            .ok_or("zero is not expiry authority")?;
            sqlstate(error, "23514")?;
            // A retained zero row still occupies both the slot and its category.
            for (slot, category) in [(0_i16, 4_i16), (1, 0)] {
                let error = sqlx::query(INSERT_SLOT)
                    .bind(item.as_slice())
                    .bind(slot)
                    .bind(category)
                    .bind(1_i16)
                    .bind(72_000_000_i64)
                    .bind(id(120).to_vec())
                    .bind(id(130).to_vec())
                    .execute(&harness.pool)
                    .await
                    .err()
                    .ok_or("retained row is occupied")?;
                sqlstate(error, "23505")?;
            }
            let before_move = snapshot(&harness.pool, item).await?;
            let backpack = harness.mint(&authority, BACKPACK, 1).await?;
            harness
                .committed(&authority, to_slot(command(1)?, backpack, backpack_facts()))
                .await?;
            harness
                .committed(
                    &authority,
                    to_backpack(
                        command(2)?,
                        item,
                        facts(STONE, ItemStackClass::NonStackable),
                    ),
                )
                .await?;
            // Re-open the database connection: state follows ItemInstance across a move/reload.
            let pool = sqlx::PgPool::connect(&harness.database.url).await?;
            assert_eq!(snapshot(&pool, item).await?, before_move);
            let rows = sqlx::query(
                "SELECT slot, tier, remaining_ms, imbuement_revision \
                                    FROM game_item_imbuements \
                                    WHERE item_instance_id = encode($1,'hex')::uuid ORDER BY slot",
            )
            .bind(item.as_slice())
            .fetch_all(&pool)
            .await?;
            pool.close().await;
            assert_eq!(rows.len(), 3);
            for (slot, row) in rows.iter().enumerate() {
                assert_eq!(row.try_get::<i16, _>("slot")?, i16::try_from(slot)?);
                assert_eq!(row.try_get::<i16, _>("tier")?, i16::try_from(slot + 1)?);
                assert_eq!(
                    row.try_get::<i64, _>("remaining_ms")?,
                    if slot == 0 { 0 } else { 72_000_000 }
                );
                assert_eq!(row.try_get::<String, _>("imbuement_revision")?, "imbue-r1");
            }
            // Both active and entirely zero-time retained rows refuse a sale.
            for expire_all in [false, true] {
                if expire_all {
                    sqlx::query(
                        "UPDATE game_item_imbuements SET remaining_ms = 0 \
                         WHERE item_instance_id = encode($1,'hex')::uuid",
                    )
                    .bind(item.as_slice())
                    .execute(&harness.pool)
                    .await?;
                }
                let before_sale = snapshot(&harness.pool, item).await?;
                let footprint = harness.footprint().await?;
                let item_state = harness.item_state(item).await?;
                let revision = harness.character_revision().await?;
                let trade = harness
                    .root
                    .commit_npc_trade(
                        &authority,
                        &harness.node,
                        fence()?,
                        NpcTradeRequest {
                            cause: NpcTradeCause {
                                npc_key: "fixture:npc.trader".into(),
                                catalogue_revision: 1,
                                offer_index: 0,
                                side: NpcTradeSide::Sell,
                                occurrence: id(140),
                            },
                            command: command(3)?,
                            quantity: 1,
                            unit_price: 0,
                            offer_count: 1,
                            item: facts(STONE, ItemStackClass::NonStackable),
                            transaction_id: id(141),
                            event_id: id(142),
                            occurred_at_unix_ms: 1_790_000_000_000,
                            output_item_instance_ids: [id(143), id(144), id(145)],
                            coins: NpcTradeCoins {
                                gold: definition("Item", Coin::Gold.production_key()),
                                platinum: definition("Item", Coin::Platinum.production_key()),
                                crystal: definition("Item", Coin::Crystal.production_key()),
                            },
                            backpack: backpack_facts(),
                        },
                    )
                    .await;
                assert!(
                    matches!(trade, Err(NpcTradeError::ItemNotFound)),
                    "{trade:?}"
                );
                assert_eq!(snapshot(&harness.pool, item).await?, before_sale);
                assert_eq!(harness.footprint().await?, footprint);
                assert_eq!(harness.item_state(item).await?, item_state);
                assert_eq!(harness.character_revision().await?, revision);
                assert_eq!(harness.count("game_npc_trades").await?, 0);
            }
            Ok(())
        }
        .await;
        harness.cleanup().await?;
        result
    })
}

#[test]
fn imbuement_postgres_runtime_remains_read_only() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "imbuement_grants").await?;
        let result: TestResult = async {
            for role in [
                "SET LOCAL ROLE oteryn_game_runtime",
                "SET LOCAL ROLE oteryn_game_control",
            ] {
                let mut tx = harness.pool.begin().await?;
                sqlx::query(role).execute(&mut *tx).await?;
                let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_imbuements")
                    .fetch_one(&mut *tx)
                    .await?;
                assert_eq!(count, 0);
                tx.rollback().await?;
                for statement in [
                    "INSERT INTO game_item_imbuements DEFAULT VALUES",
                    "UPDATE game_item_imbuements SET remaining_ms = 0",
                    "DELETE FROM game_item_imbuements",
                    "TRUNCATE game_item_imbuements",
                ] {
                    let mut tx = harness.pool.begin().await?;
                    sqlx::query(role).execute(&mut *tx).await?;
                    let error = sqlx::query(statement)
                        .execute(&mut *tx)
                        .await
                        .err()
                        .ok_or("uncomposed runtime writes must remain closed")?;
                    sqlstate(error, "42501")?;
                    tx.rollback().await?;
                }
            }
            let error = sqlx::query("TRUNCATE game_item_imbuements")
                .execute(&harness.pool)
                .await
                .err()
                .ok_or("truncate must reject for the owner too")?;
            sqlstate(error, "23514")?;
            Ok(())
        }
        .await;
        harness.cleanup().await?;
        result
    })
}
