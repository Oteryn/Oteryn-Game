//! Regression cases with genuine PostgreSQL character/session leases and native owners.
use crate::bestiary_postgres_harness::{Harness, TestResult, WORLD, debug, id};

#[test]
fn charm_gaps_postgres_cleanse_settles_each_overdue_tick() -> TestResult {
    scenario("cleanse")
}
#[test]
fn charm_gaps_postgres_sequential_periodic_parry() -> TestResult {
    scenario("parry")
}
#[test]
fn charm_gaps_postgres_periodic_numb_uses_native_survival() -> TestResult {
    scenario("numb")
}
#[test]
fn charm_gaps_postgres_native_bite_reflections_are_sequential() -> TestResult {
    scenario("native-bite")
}
#[test]
fn charm_gaps_postgres_equipment_first_reflections_are_sequential() -> TestResult {
    scenario("native-equipment-first")
}
#[test]
fn charm_gaps_postgres_nonlethal_raw_parry_keeps_equipment_and_numb() -> TestResult {
    scenario("native-raw-lethal")
}

fn scenario(mode: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run_async(async move |admin| {
        let h = Harness::create(
            admin,
            &format!("charm_gaps_{}", mode.replace('-', "_")),
            false,
        )
        .await?;
        let result = async {
            let now = sqlx::query_scalar::<_, i64>(
                "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint",
            )
            .fetch_one(&h.pool)
            .await?;
            let current = crate::world_party_runtime_postgres_cases::committed_fresh_player(
                &h, 60, 62, 70, now,
            )
            .await?;
            let world = crate::foundation::WorldId::decode(&id(WORLD)).map_err(debug)?;
            // Assertion failures must release this case's temporary database too.
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                super::all_charms_contracts::charm_gaps_durable_player(
                    world,
                    current.current_game_session_id(),
                    current.current_character_lease(),
                    mode,
                )
            }))
            .unwrap_or_else(|_| Err("native charm scenario panicked".into()))
        }
        .await;
        h.cleanup().await?;
        result
    })
}
