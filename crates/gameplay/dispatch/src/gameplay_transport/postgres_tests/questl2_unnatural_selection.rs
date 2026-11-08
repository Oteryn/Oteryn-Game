//! Unnatural Selection must traverse its real Skull, dance, crystal and beer lifecycle.
use super::TestResult;

#[test]
fn questl2_unnatural_selection_full_player_journey() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::fixture::run(async |admin| {
        let started = std::time::Instant::now();
        let h = super::fixture::Harness::create_quest_reward(admin, "l2_unnatural_binding", true)
            .await?;
        let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
            .fetch_one(&h.pool)
            .await?;
        eprintln!(
            "POSTGRES_CASE_STARTED qengine-rewards-l2-unnatural-binding server_version={version}"
        );
        let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
        let required = [
            "/l2-unnatural-recover-skull",
            "/l2-unnatural-return-tour",
            "/l2-unnatural-return-beer",
        ];
        let missing: Vec<_> = required
            .into_iter()
            .filter(|key| !callbacks.iter().any(|callback| callback.key.ends_with(key)))
            .collect();
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_BINDING_PROBE qengine-rewards-l2-unnatural duration={:?}",
            started.elapsed()
        );
        assert!(
            missing.is_empty(),
            "Unnatural Selection lifecycle bindings absent: {missing:?}"
        );
        Ok(())
    })?;
    super::qengine_rewards::scenario("qengine-rewards-l2-unnatural")
}
