//! Exact Devil Helmet source keys and three once treasures.
use super::TestResult;

#[test]
fn questpc5_devil_helmet_five_source_acquisitions_and_cold_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let started = std::time::Instant::now();
        let harness = super::fixture::Harness::create_with_admission_seed(
            admin,
            "questpc5_devil_rewards",
            false,
            false,
        )
        .await?;
        let version: String = sqlx::query_scalar("SHOW server_version")
            .fetch_one(&harness.pool)
            .await?;
        eprintln!("POSTGRES_CASE_STARTED devil_source_rewards server_version={version}");
        let startup: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/quest-startup-bindings.json"
        )))?;
        for (uid, position) in [
            (5016, [32576, 32216, 15]),
            (5017, [32589, 32100, 14]),
            (6139, [32459, 32144, 15]),
            (6140, [32465, 32148, 15]),
            (6141, [32466, 32148, 15]),
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
            "POSTGRES_CASE_COMPLETED devil_source_reward_registration duration={:?}",
            started.elapsed()
        );
        Ok(())
    })?;
    super::quest_catalogue_chests::exact_claim_cohort(&[
        "oteryn:reward-claim.quest.key.id3667",
        "oteryn:reward-claim.quest.key.id3610",
        "oteryn:reward-claim.kv.devilhemet1",
        "oteryn:reward-claim.kv.devilhemet2",
        "oteryn:reward-claim.kv.devilhemet3",
    ])
}
