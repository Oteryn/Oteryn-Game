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
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT table_name FROM information_schema.tables WHERE table_schema='public' AND table_name IN ('game_character_concoction_receipts','game_character_drome_cube_receipts') ORDER BY table_name",
        )
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(
            tables,
            [
                "game_character_concoction_receipts",
                "game_character_drome_cube_receipts"
            ]
        );
        let cause_344: bool = sqlx::query_scalar(
            "SELECT pg_get_constraintdef(oid) LIKE '%344%' FROM pg_constraint WHERE conrelid='game_item_use_records'::regclass AND conname='game_item_use_records_cause_kind_check'",
        )
        .fetch_one(&h.pool)
        .await?;
        assert!(cause_344);
        Ok(())
    })
}
