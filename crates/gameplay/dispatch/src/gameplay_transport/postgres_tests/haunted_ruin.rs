use super::{TestResult, haunted_ruin_fixtures};

#[test]
fn haunted_ruin_holy_water_sets_mission_spawns_ghost_and_opens_timed_door() -> TestResult {
    haunted_ruin_fixtures::haunted_ruin_holy_water_sets_mission_spawns_ghost_and_opens_timed_door_scenario()
}

#[test]
fn haunted_ruin_holy_water_reaches_the_native_ground_and_resets_the_door() -> TestResult {
    haunted_ruin_fixtures::haunted_ruin_holy_water_reaches_the_native_ground_and_resets_the_door_scenario()
}
