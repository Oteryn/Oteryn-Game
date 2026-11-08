//! Secret Service qualification requires issued objects, returns and faction exclusion.
use super::TestResult;

#[test]
fn questl2_secret_service_avin_full_player_journey() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::fixture::run(async |admin| {
        let started = std::time::Instant::now();
        let h = super::fixture::Harness::create_quest_reward(admin, "l2_secret_binding", true)
            .await?;
        let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("POSTGRES_CASE_STARTED qengine-rewards-l2-secret-avin-binding server_version={version}");
        let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
        let required = [
            "/l2-secret-avin-enlist",
            "/l2-secret-avin-issue",
            "/l2-secret-avin-return",
            "/l2-secret-avin-finish",
        ];
        let missing: Vec<_> = required
            .into_iter()
            .filter(|key| !callbacks.iter().any(|callback| callback.key.ends_with(key)))
            .collect();
        h.cleanup().await?;
        eprintln!("POSTGRES_BINDING_PROBE qengine-rewards-l2-secret-avin duration={:?}", started.elapsed());
        assert!(missing.is_empty(), "complete Secret Service bindings absent: {missing:?}");
        Ok(())
    })?;
    // A registry row cannot qualify this quest: execute the entire native journey.
    super::qengine_rewards::scenario("qengine-rewards-l2-secret-avin")
}
