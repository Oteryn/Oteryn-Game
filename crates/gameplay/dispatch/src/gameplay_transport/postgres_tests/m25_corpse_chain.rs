//! Actual native M25 corpse MINTs retain the original source deadline chain.
use super::TestResult;
use sqlx::Row;

pub(super) async fn assert_native_corpse_chain(pool: &sqlx::PgPool, creature: &str) -> TestResult {
    let chain = match creature {
        "oteryn:creature.ahau" => [42069, 42072, 42071, 42070],
        "oteryn:creature.mitmah_vanguard" => [44687, 44686, 44685, 44684],
        _ => return Err("M25 corpse proof requires its exact native boss".into()),
    };
    let root = format!("oteryn:item.tibia.i{}", chain[0]);
    let mints = sqlx::query(
        "SELECT item_instance_id::text,transaction_id::text,materialized_at,\
         game_corpse_retirement_deadline(item_instance_id)-materialized_at AS lifetime \
         FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION' \
         AND draw_ordinal=0 AND loot_table_production_key=$1",
    )
    .bind(&root)
    .fetch_all(pool)
    .await?;
    assert_eq!(
        mints.len(),
        1,
        "first native death must create exactly one actual corpse MINT"
    );
    let mint = &mints[0];
    let materialized = mint
        .try_get::<Option<i64>, _>("materialized_at")?
        .ok_or("actual M25 corpse must be materialized before its chain is checked")?;
    assert!(materialized > 0);
    assert_eq!(mint.try_get::<Option<i64>, _>("lifetime")?, Some(670000));
    let rows = sqlx::query(
        "SELECT source_stage,source_transaction_id::text,source_ordinal,duration_millis,\
         expires_at_unix_ms,definition_family,definition_production_key,definition_revision,\
         target_family,target_production_key,target_revision,blocks_movement,\
         blocks_projectile,immovable_block_solid FROM game_spell_field_temporal_schedules \
         WHERE item_instance_id=$1::uuid ORDER BY source_stage",
    )
    .bind(mint.try_get::<String, _>("item_instance_id")?)
    .fetch_all(pool)
    .await?;
    assert_eq!(
        rows.len(),
        3,
        "native corpse MINT must schedule all three source transforms"
    );
    for (index, row) in rows.iter().enumerate() {
        assert_eq!(
            row.try_get::<i32, _>("source_stage")?,
            i32::try_from(index + 1)?
        );
        assert_eq!(
            row.try_get::<String, _>("source_transaction_id")?,
            mint.try_get::<String, _>("transaction_id")?
        );
        assert_eq!(row.try_get::<i32, _>("source_ordinal")?, 0);
        assert_eq!(
            row.try_get::<i64, _>("duration_millis")?,
            [10000, 300000, 300000][index]
        );
        assert_eq!(
            row.try_get::<i64, _>("expires_at_unix_ms")?,
            materialized + [10000, 310000, 610000][index]
        );
        assert_eq!(row.try_get::<String, _>("definition_family")?, "Item");
        assert_eq!(
            row.try_get::<String, _>("definition_production_key")?,
            format!("oteryn:item.tibia.i{}", chain[index])
        );
        assert_eq!(
            row.try_get::<String, _>("definition_revision")?,
            "definition-r1"
        );
        assert_eq!(
            row.try_get::<Option<String>, _>("target_family")?
                .as_deref(),
            Some("Item")
        );
        assert_eq!(
            row.try_get::<Option<String>, _>("target_production_key")?,
            Some(format!("oteryn:item.tibia.i{}", chain[index + 1]))
        );
        assert_eq!(
            row.try_get::<Option<String>, _>("target_revision")?
                .as_deref(),
            Some("definition-r1")
        );
        assert!(!row.try_get::<bool, _>("blocks_movement")?);
        assert!(!row.try_get::<bool, _>("blocks_projectile")?);
        assert!(!row.try_get::<bool, _>("immovable_block_solid")?);
    }
    let legacy: (Vec<i32>, Vec<i32>) = sqlx::query_as(
        "SELECT game_summer_corpse_chain('oteryn:item.tibia.i39530'),\
         game_summer_corpse_chain('oteryn:item.tibia.i54678')",
    )
    .fetch_one(pool)
    .await?;
    assert_eq!(
        legacy,
        (
            vec![39530, 39529, 39528, 39527],
            vec![54678, 54679, 54680, 54681]
        )
    );
    eprintln!("M25_ACTUAL_CORPSE_CHAIN_REACHED creature={creature} schedules=3 lifetime_ms=670000");
    Ok(())
}
