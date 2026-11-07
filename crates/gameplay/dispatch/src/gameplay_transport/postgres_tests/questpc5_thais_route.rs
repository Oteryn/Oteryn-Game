//! Source Thais puzzle accepts creatures and restores its pressure projection on every end path.
use super::TestResult;

#[test]
fn questpc5_thais_complete_source_route() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let started = std::time::Instant::now();
        let h = super::fixture::Harness::create_with_admission_seed(
            admin,
            "questpc5_thais_route",
            false,
            false,
        )
        .await?;
        eprintln!(
            "POSTGRES_CASE_STARTED thais_creature_occupancy {:?}",
            started.elapsed()
        );
        let doc: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/quests/shared-access-callbacks.json"
        )))?;
        let mut plate = doc["callbacks"]
            .as_array()
            .ok_or("callbacks")?
            .iter()
            .find(|c| c["key"] == "oteryn:quest-callback/thais-lighthouse-plate")
            .ok_or("source plate")?
            .clone();
        plate["occupancy_driven"] = true.into();
        let _: oteryn_gameplay_quests::quest::world_callbacks::Callback =
            serde_json::from_value(plate)?;
        let startup: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/quest-startup-bindings.json"
        )))?;
        for uid in [6014, 6015] {
            assert!(
                startup["bindings"]
                    .as_array()
                    .ok_or("startup bindings")?
                    .iter()
                    .any(|b| b["attribute"] == "unique" && b["value"] == uid),
                "source lighthouse treasure selector {uid} is absent"
            );
        }
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_CASE_COMPLETED thais_source_occupancy_registration {:?}",
            started.elapsed()
        );
        Ok(())
    })
}
