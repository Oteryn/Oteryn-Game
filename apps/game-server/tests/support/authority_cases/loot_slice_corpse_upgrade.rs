use crate::corpse_transfer_postgres_cases::mint_corpse_with_definition;
use crate::durability::item_decay_retire::CORPSE_DECAY_AFTER_MS;
use crate::foundation::{ChannelId, WorldId};
use crate::item_transfer_postgres_cases::{
    CHANNEL, CHARACTER, Database, Harness, TestResult, WORLD, configured_admin, debug, id, runtime,
};
use oteryn_gameplay_durability::GAME_MIGRATOR;
use sqlx::Connection;

const MIGRATION: i64 = 3200;
const PRIOR_FUNCTIONS: [&str; 4] = [
    "game_summer_corpse_chain(text)",
    "game_source_corpse_durations_v1(text)",
    "game_item_plain_corpse_source(uuid)",
    "game_corpse_retirement_deadline(uuid)",
];

#[test]
fn loot_slice_upgrade_preserves_preexisting_corpse_deadline() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            runtime().map_err(debug)?.block_on(async {
                let (database, mut connection) =
                    Database::create_unmigrated(admin, "loot_slice_upgrade").await?;
                GAME_MIGRATOR.run_to(MIGRATION - 1, &mut connection).await?;
                let prior: Vec<String> = sqlx::query_scalar(
                    "SELECT pg_get_functiondef(signature::regprocedure) FROM unnest($1::text[]) AS signature",
                )
                .bind(PRIOR_FUNCTIONS.as_slice())
                .fetch_all(&mut connection)
                .await?;
                // The Rust writer requires the full migration ledger before opening.
                GAME_MIGRATOR.run(&mut connection).await?;
                connection.close().await?;
                let harness = Harness::open(database, "loot_slice_upgrade").await?;
                let result = exercise(&harness, &prior).await;
                harness.cleanup().await?;
                result
            })
            .map_err(debug)
        })?
        .join()
        .map_err(|_| "loot slice upgrade worker failed")?
        .map_err(Into::into)
}

async fn exercise(harness: &Harness, prior: &[String]) -> TestResult {
    let creatures: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../content/creatures/definitions/creatures-00500-00999.json"
    )))?;
    let spider = creatures["records"]
        .as_array()
        .and_then(|records| {
            records.iter().find(|record| {
                record["definition"]["identity"]["key"] == "oteryn:creature.giant_spider"
            })
        })
        .ok_or("Giant Spider missing from creature content")?;
    let key = spider["authoring"]["profile"]["details"]["corpse_item"]["key"]
        .as_str()
        .ok_or("Giant Spider missing its corpse item")?;
    let definition = || crate::item_transfer_postgres_cases::definition("Item", key);

    let mut tx = harness.pool.begin().await?;
    for function in prior {
        sqlx::raw_sql(sqlx::AssertSqlSafe(function.as_str()))
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let old = mint_corpse_with_definition(
        harness,
        &authority,
        1000,
        id(CHARACTER),
        definition(),
        definition(),
    )
    .await?;
    assert_eq!(
        harness.count("game_spell_field_temporal_schedules").await?,
        0
    );
    assert_eq!(lifetime(harness, old).await?, CORPSE_DECAY_AFTER_MS);

    let migration = GAME_MIGRATOR
        .iter()
        .find(|migration| migration.version == MIGRATION)
        .ok_or("loot slice migration missing")?;
    let mut tx = harness.pool.begin().await?;
    sqlx::raw_sql(migration.sql.clone())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let schedule = harness
        .root
        .read_corpse_decay_schedule(
            &authority,
            WorldId::decode(&id(WORLD)).map_err(debug)?,
            ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
        )
        .await
        .map_err(debug)?;
    assert_eq!(schedule.corpses.len(), 1);
    let corpse = &schedule.corpses[0];
    assert_eq!(corpse.corpse_item_instance_id, old);
    assert_eq!(
        corpse
            .decay_at_unix_ms
            .map(|deadline| deadline - corpse.materialized_at_unix_ms),
        Some(CORPSE_DECAY_AFTER_MS),
        "{key} has no mint-time transforms, so it must retain the legacy deadline",
    );

    let fresh = mint_corpse_with_definition(
        harness,
        &authority,
        1001,
        id(CHARACTER),
        definition(),
        definition(),
    )
    .await?;
    let durations: Vec<i64> = sqlx::query_scalar("SELECT game_source_corpse_durations_v1($1)")
        .bind(key)
        .fetch_one(&harness.pool)
        .await?;
    let (terminal, transforms) = durations.split_last().ok_or("empty corpse policy")?;
    assert!(!transforms.is_empty());
    let stages: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT s.duration_millis, s.expires_at_unix_ms - m.materialized_at
         FROM game_item_mint_receipts m JOIN game_spell_field_temporal_schedules s
           USING (item_instance_id)
         WHERE m.item_instance_id = encode($1,'hex')::uuid
           AND s.source_transaction_id = m.transaction_id AND s.source_ordinal = 0
         ORDER BY s.source_stage",
    )
    .bind(fresh.as_slice())
    .fetch_all(&harness.pool)
    .await?;
    let mut elapsed = 0;
    let expected: Vec<_> = transforms
        .iter()
        .map(|duration| {
            elapsed += duration;
            (*duration, elapsed)
        })
        .collect();
    assert_eq!(stages, expected);
    let fresh_lifetime = lifetime(harness, fresh).await?;
    assert_eq!(fresh_lifetime, elapsed + terminal);
    eprintln!(
        "LOOT_SLICE_CORPSE_UPGRADE_REACHED {key} legacy_ms={CORPSE_DECAY_AFTER_MS} fresh_ms={fresh_lifetime}"
    );
    Ok(())
}

async fn lifetime(harness: &Harness, item: [u8; 16]) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT game_corpse_retirement_deadline(item_instance_id) - materialized_at
         FROM game_item_mint_receipts WHERE item_instance_id = encode($1,'hex')::uuid",
    )
    .bind(item.as_slice())
    .fetch_one(&harness.pool)
    .await?)
}
