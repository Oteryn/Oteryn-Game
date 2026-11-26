use crate::corpse_transfer_postgres_cases::mint_corpse_with_definition;
use crate::item_transfer_postgres_cases::{
    CHARACTER, Harness, TestResult, debug, definition, id, runtime,
};

#[test]
fn dead_rat_mint_schedules_two_transforms_and_terminal_retirement() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime().map_err(debug)?.block_on(async move {
                let harness = Harness::create(admin, "dead_rat_corpse_chain").await.map_err(debug)?;
                let result = assert_chain(&harness).await.map_err(debug);
                harness.cleanup().await.map_err(debug)?;
                result
            })
        })?
        .join()
        .map_err(|_| "Dead Rat corpse chain worker failed")?
        .map_err(Into::into)
}

async fn assert_chain(harness: &Harness) -> TestResult {
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let rat = || definition("Item", "oteryn:item.tibia.i3994");
    let corpse =
        mint_corpse_with_definition(harness, &authority, 1000, id(CHARACTER), rat(), rat()).await?;
    let stages: Vec<(String, String, i64, i64)> = sqlx::query_as(
        "SELECT s.definition_production_key, s.target_production_key, s.duration_millis,
                s.expires_at_unix_ms - m.materialized_at
         FROM game_item_mint_receipts m JOIN game_spell_field_temporal_schedules s
           USING (item_instance_id)
         WHERE m.item_instance_id = encode($1,'hex')::uuid
           AND s.source_transaction_id = m.transaction_id AND s.source_ordinal = 0
         ORDER BY s.source_stage",
    )
    .bind(corpse.as_slice())
    .fetch_all(&harness.pool)
    .await?;
    assert_eq!(
        stages,
        [
            ("oteryn:item.tibia.i3994".to_owned(), "oteryn:item.tibia.i3995".to_owned(), 300_000, 300_000),
            ("oteryn:item.tibia.i3995".to_owned(), "oteryn:item.tibia.i3996".to_owned(), 300_000, 600_000),
        ]
    );
    let lifetime: i64 = sqlx::query_scalar(
        "SELECT game_corpse_retirement_deadline(item_instance_id) - materialized_at
         FROM game_item_mint_receipts WHERE item_instance_id = encode($1,'hex')::uuid",
    )
    .bind(corpse.as_slice())
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(lifetime, 660_000);
    Ok(())
}
