//! Both faction journeys must issue missions, exchange real items and unlock their shops.
use super::TestResult;

fn journey(case: &'static str, required: &[&str]) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::fixture::run(async |admin| {
        let started = std::time::Instant::now();
        let h =
            super::fixture::Harness::create_quest_reward(admin, "l2_djinn_binding", true).await?;
        let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("POSTGRES_CASE_STARTED {case}-binding server_version={version}");
        let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
        let present = required
            .iter()
            .all(|key| callbacks.iter().any(|callback| callback.key.ends_with(key)));
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_BINDING_PROBE {case} duration={:?}",
            started.elapsed()
        );
        assert!(
            present,
            "complete Djinn mission bindings absent: {required:?}"
        );
        Ok(())
    })?;
    // Presence never qualifies a faction: its entire native journey must run.
    super::qengine_rewards::scenario(case)
}

#[test]
fn questl2_djinn_efreet_full_player_journey() -> TestResult {
    journey(
        "qengine-rewards-l2-djinn-efreet",
        &[
            "/l2-efreet-enlist",
            "/l2-efreet-lamp-mission",
            "/l2-efreet-finish",
        ],
    )
}

#[test]
fn questl2_djinn_marid_full_player_journey() -> TestResult {
    journey(
        "qengine-rewards-l2-djinn-marid",
        &[
            "/l2-marid-cookbook",
            "/l2-marid-spy-report",
            "/l2-marid-finish",
        ],
    )
}
