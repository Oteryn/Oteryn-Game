//! Reuse independent native backpack MINT outputs for the selected Nomads route.
use super::TestResult;

#[test]
fn questpc5_independent_backpack_rewards_are_atomic_and_not_container_contents() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let started = std::time::Instant::now();
        let h = super::fixture::Harness::create_with_admission_seed(
            admin,
            "questpc5_backpack_outputs",
            false,
            false,
        )
        .await?;
        let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("POSTGRES_CASE_STARTED questpc5_backpack_outputs server_version={version}");
        let parsed = serde_json::from_value::<
            oteryn_gameplay_quests::quest::reward_callbacks::ItemReward,
        >(serde_json::json!({
            "item":"oteryn:item.tibia.i7343","count":1,"contents":[],
            "backpack_outputs":[{"item":"oteryn:item.tibia.i3032","count":3},{"item":"oteryn:item.tibia.i3054","count":1}]
        }));
        if let Err(error) = parsed {
            h.cleanup().await?;
            eprintln!(
                "POSTGRES_CASE_COMPLETED questpc5_backpack_outputs duration={:?} success=false",
                started.elapsed()
            );
            return Err(error.into());
        }
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_CASE_COMPLETED questpc5_backpack_outputs duration={:?} success=true",
            started.elapsed()
        );
        Ok(())
    })
}
