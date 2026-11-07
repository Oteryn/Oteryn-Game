//! Exact source acquisitions for the Demon Helmet route, including its prerequisite key.
use super::TestResult;

#[test]
fn questpc5_demon_helmet_four_source_acquisitions_and_cold_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let started = std::time::Instant::now();
        let harness = super::fixture::Harness::create_with_admission_seed(
            admin,
            "questpc5_demon_rewards",
            false,
            false,
        )
        .await?;
        let version: String = sqlx::query_scalar("SHOW server_version")
            .fetch_one(&harness.pool)
            .await?;
        eprintln!("POSTGRES_CASE_STARTED demon_source_rewards server_version={version}");
        let startup: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/quest-startup-bindings.json"
        )))?;
        for (uid, position) in [
            (6029, [33313, 31574, 15]),
            (6030, [33313, 31575, 15]),
            (6031, [33313, 31576, 15]),
            (6076, [33063, 31624, 15]),
        ] {
            assert!(
                startup["bindings"]
                    .as_array()
                    .ok_or("startup bindings")?
                    .iter()
                    .any(|binding| binding["attribute"] == "unique"
                        && binding["value"] == uid
                        && binding["position"] == serde_json::json!(position)),
                "missing exact source acquisition selector {uid}"
            );
        }
        harness.cleanup().await?;
        eprintln!(
            "POSTGRES_CASE_COMPLETED demon_source_reward_registration duration={:?}",
            started.elapsed()
        );
        Ok(())
    })?;
    super::quest_catalogue_chests::exact_claim_cohort(&[
        "oteryn:reward-claim.quest.u6_4.demon_helmet.rewards.steel_boots",
        "oteryn:reward-claim.quest.u6_4.demon_helmet.rewards.demon_helmet",
        "oteryn:reward-claim.quest.u6_4.demon_helmet.rewards.demon_shield",
        "oteryn:reward-claim.quest.u7_2.parchment_room.bag",
    ])
}
