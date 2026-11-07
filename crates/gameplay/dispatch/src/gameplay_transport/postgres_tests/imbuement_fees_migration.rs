//! Upgrade historical receipts without rewriting them or allowing old prices on new rows.
use super::TestResult;
use super::fixture::{Harness, run};
use sqlx::Acquire;

#[test]
fn imbuement_fee_migration_preserves_history_and_rejects_new_legacy_fees() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    run(async move |admin| {
        let harness = Harness::create_with_admission_seed(admin, "imbuement_fee_upgrade", true, false).await?;
        let mut tx = harness.pool.begin().await?;
        // A private schema isolates the pre-upgrade fee constraints from the fully migrated
        // production tables. The real migration below must work with populated old tables.
        sqlx::raw_sql(
            "CREATE SCHEMA imbuement_fee_upgrade;
             SET LOCAL search_path TO imbuement_fee_upgrade, public;
             CREATE TABLE game_item_imbuement_records (
               action SMALLINT, tier SMALLINT, materials JSONB, fee_gold_units BIGINT,
               output_definition_family TEXT, output_production_key TEXT, output_revision_ref TEXT,
               CONSTRAINT game_item_imbuement_records_check CHECK (
                 fee_gold_units=CASE WHEN action=2 THEN 15000
                   WHEN tier=1 THEN 5000 WHEN tier=2 THEN 30000 ELSE 200000 END));
             CREATE TABLE game_item_fee_burns (
               fee_gold_units BIGINT,
               CONSTRAINT game_item_fee_burns_purchase_cause
                 CHECK (fee_gold_units IN (5000,30000,200000,15000)));
             INSERT INTO game_item_imbuement_records(action,tier,materials,fee_gold_units)
               VALUES (1,1,'[null]',5000),(1,2,'[null]',30000),(1,3,'[null]',200000),
                      (2,1,'[]',15000);
             INSERT INTO game_item_imbuement_records
               VALUES (3,2,'[null]',30000,'Item','scroll','r1'),
                      (3,3,'[null]',200000,'Item','scroll','r1');
             INSERT INTO game_item_fee_burns VALUES (5000),(30000),(200000),(15000);",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../apps/game-server/migrations/0298_imbuement_current_fees.sql"
        )))
        .execute(&mut *tx)
        .await?;
        let history: Vec<i64> = sqlx::query_scalar(
            "SELECT fee_gold_units FROM game_item_imbuement_records ORDER BY action,tier",
        )
        .fetch_all(&mut *tx)
        .await?;
        assert_eq!(history, [5000, 30000, 200000, 15000, 30000, 200000]);
        let burns: Vec<i64> = sqlx::query_scalar(
            "SELECT fee_gold_units FROM game_item_fee_burns ORDER BY fee_gold_units",
        )
        .fetch_all(&mut *tx)
        .await?;
        assert_eq!(burns, [5000, 15000, 30000, 200000]);
        for (tier, gold) in [(1_i16, 7500_i64), (2, 60000), (3, 250000)] {
            sqlx::query("INSERT INTO game_item_imbuement_records(action,tier,materials,fee_gold_units) VALUES(1,$1,'[null]',$2)")
                .bind(tier).bind(gold).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_fee_burns VALUES($1)")
                .bind(gold).execute(&mut *tx).await?;
        }
        sqlx::raw_sql(
            "INSERT INTO game_item_imbuement_records(action,tier,materials,fee_gold_units) VALUES(2,1,'[]',15000);
             INSERT INTO game_item_imbuement_records VALUES(3,2,'[null]',60000,'Item','scroll','r1'),(3,3,'[null]',250000,'Item','scroll','r1');
             INSERT INTO game_item_fee_burns VALUES(15000);",
        ).execute(&mut *tx).await?;
        for sql in [
            "INSERT INTO game_item_imbuement_records(action,tier,materials,fee_gold_units) VALUES(1,1,'[null]',5000)",
            "INSERT INTO game_item_imbuement_records(action,tier,materials,fee_gold_units) VALUES(1,2,'[null]',30000)",
            "INSERT INTO game_item_imbuement_records(action,tier,materials,fee_gold_units) VALUES(1,3,'[null]',200000)",
            "INSERT INTO game_item_imbuement_records(action,tier,materials,fee_gold_units) VALUES(2,1,'[]',30000)",
            "INSERT INTO game_item_imbuement_records VALUES(3,2,'[null]',30000,'Item','scroll','r1')",
            "INSERT INTO game_item_imbuement_records VALUES(3,3,'[null]',200000,'Item','scroll','r1')",
            "INSERT INTO game_item_fee_burns VALUES(5000)",
            "INSERT INTO game_item_fee_burns VALUES(30000)",
            "INSERT INTO game_item_fee_burns VALUES(200000)",
        ] {
            let mut attempt = tx.begin().await?;
            let error = sqlx::query(sql)
                .execute(&mut *attempt)
                .await
                .expect_err("new receipts must reject obsolete or incorrect fees");
            assert_eq!(error.as_database_error().and_then(|e| e.code()).as_deref(), Some("23514"));
            attempt.rollback().await?;
        }
        tx.rollback().await?;
        harness.cleanup().await
    })
}
