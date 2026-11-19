use super::{TestResult, vampire_hunt_fixtures};

#[test]
fn vampire_hunt_uses_all_three_altar_items_for_weakened_count_only() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    vampire_hunt_fixtures::vampire_hunt_uses_all_three_altar_items_for_weakened_count_only_scenario(
    )
}

#[test]
fn vampire_hunt_routes_both_native_ground_targets_to_their_summon_branches() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    vampire_hunt_fixtures::vampire_hunt_routes_both_native_ground_targets_to_their_summon_branches_scenario()
}
