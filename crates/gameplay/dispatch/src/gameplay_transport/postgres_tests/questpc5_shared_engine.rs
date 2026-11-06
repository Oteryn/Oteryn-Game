//! Shared-engine reuse must admit the source portal effect before it can execute.
use super::TestResult;

#[test]
fn questpc5_shared_portal_effect_schema_reaches_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let started = std::time::Instant::now();
        let h = super::fixture::Harness::create_with_admission_seed(
            admin,
            "questpc5_shared_portals",
            false,
            false,
        )
        .await?;
        eprintln!(
            "POSTGRES_CASE_STARTED shared_portal_effect {:?}",
            started.elapsed()
        );
        let mut callback = serde_json::to_value(
            oteryn_gameplay_quests::quest::world_callbacks::load()?
                .into_iter()
                .find(|row| row.key.ends_with("behemoth"))
                .ok_or("callback")?,
        )?;
        callback["passages"] = serde_json::json!([]);
        callback["effects"] = serde_json::json!({
            "transform":null,"facing":null,"magic":null,"item_cycle":null,"encounter":null,
            "portals":{"appearance":1949,"lifetime_us":null,"portals":[
                {"position":{"x":32232,"y":32276,"floor":-9},"destination":{"x":32225,"y":32274,"floor":-10}},
                {"position":{"x":32225,"y":32276,"floor":-10},"destination":{"x":32230,"y":32276,"floor":-9}}
            ]}
        });
        let _: oteryn_gameplay_quests::quest::world_callbacks::Callback =
            serde_json::from_value(callback)?;
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_CASE_COMPLETED shared_portal_effect {:?}",
            started.elapsed()
        );
        Ok(())
    })
}
