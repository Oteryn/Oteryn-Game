//! Prove installed migration numbers/checksums and preserve canonical guard arms.
use super::TestResult;
use sha2::{Digest, Sha384};

#[test]
fn questpc5_registered_migration_numbers_checksums_and_canonical_guards() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let started = std::time::Instant::now();
        let h = super::fixture::Harness::create_with_admission_seed(
            admin,
            "questpc5_migration_registry",
            false,
            false,
        )
        .await?;
        let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("POSTGRES_CASE_STARTED questpc5_migration_registry server_version={version}");
        let expected: &[(i64, &str)] = &[
            (999, include_str!("../../../../../../apps/game-server/migrations/0999_quest_reward_callbacks.sql")),
            (1000, include_str!("../../../../../../apps/game-server/migrations/1000_quest_reward_payments.sql")),
            (1001, include_str!("../../../../../../apps/game-server/migrations/1001_quest_reward_participants.sql")),
            (1002, include_str!("../../../../../../apps/game-server/migrations/1002_quest_reward_state_operations.sql")),
            (1003, include_str!("../../../../../../apps/game-server/migrations/1003_quest_reward_target_sets.sql")),
            (1004, include_str!("../../../../../../apps/game-server/migrations/1004_quest_paid_item_exchanges.sql")),
            (1005, include_str!("../../../../../../apps/game-server/migrations/1005_quest_carried_use_proofs.sql")),
            (1006, include_str!("../../../../../../apps/game-server/migrations/1006_quest_owned_base_predicates.sql")),
            (1007, include_str!("../../../../../../apps/game-server/migrations/1007_quest_carried_payment_reapplication.sql")),
            (1008, include_str!("../../../../../../apps/game-server/migrations/1008_quest_carried_membership_projection.sql")),
            (1270, include_str!("../../../../../../apps/game-server/migrations/1270_spell_static_map_fields.sql")),
            (1271, include_str!("../../../../../../apps/game-server/migrations/1271_quest_world_reward_sources.sql")),
            (1272, include_str!("../../../../../../apps/game-server/migrations/1272_quest_finish.sql")),
            (1273, include_str!("../../../../../../apps/game-server/migrations/1273_corruptor_lifecycle.sql")),
            (1274, include_str!("../../../../../../apps/game-server/migrations/1274_questpc_npc_revision.sql")),
            (1275, include_str!("../../../../../../apps/game-server/migrations/1275_questpc_viola.sql")),
            (1276, include_str!("../../../../../../apps/game-server/migrations/1276_questpc_viola_book.sql")),
            (1310, include_str!("../../../../../../apps/game-server/migrations/1310_quest_npc_program_receipts.sql")),
            (1311, include_str!("../../../../../../apps/game-server/migrations/1311_marlin_trophy_exchange.sql")),
            (1312, include_str!("../../../../../../apps/game-server/migrations/1312_quest_npc_item_sources.sql")),
            (1313, include_str!("../../../../../../apps/game-server/migrations/1313_npc_conversation_delay.sql")),
            (1314, include_str!("../../../../../../apps/game-server/migrations/1314_npc_receipt_track_shape.sql")),
            (1315, include_str!("../../../../../../apps/game-server/migrations/1315_npc_fathers_burden_items.sql")),
        ];
        for (version, sql) in expected {
            let (success, checksum): (bool, Vec<u8>) =
                sqlx::query_as("SELECT success,checksum FROM _sqlx_migrations WHERE version=$1")
                    .bind(version)
                    .fetch_one(&h.pool)
                    .await?;
            assert!(success, "registered migration {version}");
            assert_eq!(
                checksum,
                Sha384::digest(sql.as_bytes()).to_vec(),
                "exact SQLx checksum {version}"
            );
        }
        let old: i64 =
            sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations WHERE version=969 OR version BETWEEN 1200 AND 1205")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(old, 0, "retired shared migration numbers are absent");
        assert!(
            !include_str!("../../../../../../apps/game-server/migrations/1272_quest_finish.sql")
                .contains("CREATE TABLE game_quest_npc_program_receipts"),
            "canonical NPC response storage belongs only to main migration1310"
        );
        let guard: String = sqlx::query_scalar(
            "SELECT pg_get_functiondef('game_quest_reward_complete()'::regprocedure)",
        )
        .fetch_one(&h.pool)
        .await?;
        assert!(
            guard.contains("game_quest_reward_grant_keys"),
            "canonical target-set guard must survive the world extension"
        );
        assert!(
            guard.contains("r.cause_kind='use'"),
            "literal world source extension is installed"
        );
        for name in [
            "1277_fathers_burden_items.sql",
            "1278_fathers_burden_xp_guard.sql",
            "1300_questpc8_reward_backpack_outputs.sql",
        ] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../apps/game-server/migrations")
                .join(name);
            assert!(!path.exists(), "foreign migration or obsolete repair: {name}");
        }
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_CASE_COMPLETED questpc5_migration_registry duration={:?}",
            started.elapsed()
        );
        Ok(())
    })
}
