use crate::corpse_transfer_postgres_cases::mint_corpse_with_definition;
use crate::durability::item_decay_retire::CORPSE_DECAY_AFTER_MS;
use crate::foundation::{ChannelId, WorldId};
use crate::item_transfer_postgres_cases::{
    CHANNEL, CHARACTER, Database, Harness, TestResult, WORLD, configured_admin, debug, definition,
    id, runtime,
};
use oteryn_gameplay_durability::GAME_MIGRATOR;
use sqlx::Connection;

const MIGRATION: i64 = 3338;
const CORPSE: &str = "oteryn:item.tibia.i7535";
const PRIOR_FUNCTIONS: [&str; 3] = [
    "game_summer_corpse_chain(text)",
    "game_source_corpse_durations_v1(text)",
    "game_corpse_retirement_deadline(uuid)",
];

#[test]
fn koshei_upgrade_preserves_unscheduled_corpse_deadline() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            runtime().map_err(debug)?.block_on(async {
                let (database, mut connection) =
                    Database::create_unmigrated(admin, "koshei_upgrade").await?;
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
                let harness = Harness::open(database, "koshei_upgrade").await?;
                let result = exercise(&harness, &prior).await;
                harness.cleanup().await?;
                result
            })
            .map_err(debug)
        })?
        .join()
        .map_err(|_| "Koshei upgrade worker failed")?
        .map_err(Into::into)
}

async fn exercise(harness: &Harness, prior: &[String]) -> TestResult {
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
        definition("Item", CORPSE),
        definition("Item", CORPSE),
    )
    .await?;
    assert_eq!(
        harness.count("game_spell_field_temporal_schedules").await?,
        0
    );

    let migration = GAME_MIGRATOR
        .iter()
        .find(|migration| migration.version == MIGRATION)
        .ok_or("Koshei corpse migration missing")?;
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
    let [corpse] = schedule.corpses.as_slice() else {
        return Err("expected only the pre-upgrade Koshei corpse".into());
    };
    assert_eq!(corpse.corpse_item_instance_id, old);
    assert_eq!(
        corpse
            .decay_at_unix_ms
            .map(|deadline| deadline - corpse.materialized_at_unix_ms),
        Some(CORPSE_DECAY_AFTER_MS),
        "the existing Koshei corpse keeps its mint-time retirement deadline",
    );
    eprintln!("KOSHEI_CORPSE_UPGRADE_PG_EXECUTED");
    Ok(())
}
