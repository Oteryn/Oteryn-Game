//! IMBUEITEMS PostgreSQL migration contract cases. The routed shrine/etcher cases live in the
//! sibling `imbuement` module; these assertions keep the new durable outcome and Albinius gate
//! schema visible in every PostgreSQL lane.
use super::fixture::{Harness, run};
use super::TestResult;

#[test]
fn inscription_receipt_accepts_only_the_atomic_action_shape() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    run(async move |admin| {
        let harness = Harness::create_with_admission_seed(admin, "imbue_inscription_schema", true, false).await?;
        let row: (bool, bool, bool) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM information_schema.columns WHERE table_name='game_item_imbuement_records' AND column_name='output_production_key'), \
                    EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='game_item_imbuement_records'::regclass AND conname='game_item_imbuement_records_action_check'), \
                    EXISTS(SELECT 1 FROM pg_proc WHERE proname='game_item_imbuement_write' AND pg_get_functiondef(oid) LIKE '%oteryn:item.tibia.i51442%')",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(row, (true, true, true));
        harness.cleanup().await
    })
}

#[test]
fn albinius_gate_relations_are_migrated_together() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    run(async move |admin| {
        let harness = Harness::create_with_admission_seed(admin, "imbue_albinius_schema", true, false).await?;
        let available: bool = sqlx::query_scalar(
            "SELECT to_regclass('game_character_quest_states') IS NOT NULL \
                 AND to_regclass('game_item_container_entries') IS NOT NULL \
                 AND to_regclass('game_item_instances') IS NOT NULL",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert!(available);
        harness.cleanup().await
    })
}
