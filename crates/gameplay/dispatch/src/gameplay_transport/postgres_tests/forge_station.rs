use super::{
    TestResult,
    item_forge_fixtures::support::{self, Case},
};

#[test]
fn forge_postgres_refuses_conversion_without_station_use() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("forge-station-required", || {
        support::scenario(Case::StationRequired)
    })
}
