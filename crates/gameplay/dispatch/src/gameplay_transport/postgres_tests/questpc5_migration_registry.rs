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
            (
                999,
                include_str!(
                    "../../../../../../apps/game-server/migrations/0999_quest_reward_callbacks.sql"
                ),
            ),
            (
                1000,
                include_str!(
                    "../../../../../../apps/game-server/migrations/1000_quest_reward_payments.sql"
                ),
            ),
            (
                1001,
                include_str!(
                    "../../../../../../apps/game-server/migrations/1001_quest_reward_participants.sql"
                ),
            ),
            (
                1002,
                include_str!(
                    "../../../../../../apps/game-server/migrations/1002_quest_reward_state_operations.sql"
                ),
            ),
            (
                1003,
                include_str!(
                    "../../../../../../apps/game-server/migrations/1003_quest_reward_target_sets.sql"
                ),
            ),
            (
                1004,
                include_str!(
                    "../../../../../../apps/game-server/migrations/1004_quest_paid_item_exchanges.sql"
                ),
            ),
            (
                1005,
                include_str!(
                    "../../../../../../apps/game-server/migrations/1005_quest_carried_use_proofs.sql"
                ),
            ),
            (
                1006,
                include_str!(
                    "../../../../../../apps/game-server/migrations/1006_quest_owned_base_predicates.sql"
                ),
            ),
            (
                1200,
                include_str!("../../../../../../apps/game-server/migrations/1200_quest_finish.sql"),
            ),
            (
                1201,
                include_str!(
                    "../../../../../../apps/game-server/migrations/1201_corruptor_lifecycle.sql"
                ),
            ),
            (
                1202,
                include_str!(
                    "../../../../../../apps/game-server/migrations/1202_questpc_npc_revision.sql"
                ),
            ),
            (
                1203,
                include_str!(
                    "../../../../../../apps/game-server/migrations/1203_questpc_viola.sql"
                ),
            ),
            (
                1204,
                include_str!(
                    "../../../../../../apps/game-server/migrations/1204_questpc_viola_book.sql"
                ),
            ),
            (
                1205,
                include_str!(
                    "../../../../../../apps/game-server/migrations/1205_fathers_burden_items.sql"
                ),
            ),
            (
                1270,
                include_str!(
                    "../../../../../../apps/game-server/migrations/1270_spell_static_map_fields.sql"
                ),
            ),
            (
                1271,
                include_str!(
                    "../../../../../../apps/game-server/migrations/1271_quest_world_reward_sources.sql"
                ),
            ),
            (
                1272,
                include_str!(
                    "../../../../../../apps/game-server/migrations/1272_fathers_burden_xp_guard.sql"
                ),
            ),
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
            sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations WHERE version=969")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(old, 0, "old shared number is absent");
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
        let father: String = sqlx::query_scalar(
            "SELECT pg_get_functiondef('game_father_values_proven()'::regprocedure)",
        )
        .fetch_one(&h.pool)
        .await?;
        assert!(
            father.contains("x.amount=expected_xp"),
            "local ambiguity repair belongs to the lane extension"
        );
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_CASE_COMPLETED questpc5_migration_registry duration={:?}",
            started.elapsed()
        );
        Ok(())
    })
}
