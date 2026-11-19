//! Reuses genuine Characters in the party-rite fixture to test the carried
use super::TestResult;

#[test]
fn lord_chamber_checks_each_station_holders_carried_item_before_group_travel() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("qespike-lord-stations", || {
        super::desert_party_rite_fixtures::scenario_with_station_custody(false, Some(4))
    })
}

#[test]
fn world_party_custody_transfers_two_players_and_dispatches_their_exits() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("qespike-two-station-round-trip", || {
        super::desert_party_rite_fixtures::scenario_with_station_custody(false, Some(2))
    })
}
