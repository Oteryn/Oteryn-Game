//! DROMECUBE schema qualification. Runtime commit/replay cases will accompany the routed writer.
use super::*;

#[test]
fn drome_concoction_schema_is_auto_mounted_and_migrated() -> TestResult {
    run(async |admin| {
        let h = Harness::create(admin, "drome_concoction_schema", false).await?;
        let columns: Vec<String> = sqlx::query_scalar(
            "SELECT column_name FROM information_schema.columns \
             WHERE table_schema='public' AND table_name='game_character_concoction_effects' \
             ORDER BY ordinal_position",
        )
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(
            columns,
            [
                "character_id",
                "concoction",
                "active_until",
                "ready_at",
                "activated_transaction_id"
            ]
        );
        Ok(())
    })
}
