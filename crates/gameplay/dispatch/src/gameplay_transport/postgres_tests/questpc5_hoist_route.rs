//! Source hoist registration: all three physical source selectors are admitted.
use super::TestResult;

#[test]
fn questpc5_hoist_source_registration() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let started = std::time::Instant::now();
        let h = super::fixture::Harness::create_with_admission_seed(
            admin,
            "questpc5_hoist_route",
            false,
            false,
        )
        .await?;
        eprintln!(
            "POSTGRES_CASE_STARTED hoist_complete_route {:?}",
            started.elapsed()
        );
        let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load()?;
        for uid in [3059, 3060, 3061] {
            assert!(
                callbacks.iter().any(|c| c.selector.unique == Some(uid)),
                "source hoist callback {uid} is absent"
            );
        }
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_CASE_COMPLETED hoist_callback_registration {:?}",
            started.elapsed()
        );
        Ok(())
    })
}
