use super::TestResult;

#[test]
fn banshee_hidden_flame_spawns_the_ambush_once_and_teleports() -> TestResult {
    super::banshee_seal_ambushes_fixtures::run_first_case()
}

#[test]
fn banshee_demonrage_spawns_two_warlocks_only_on_first_step() -> TestResult {
    super::banshee_seal_ambushes_fixtures::run_second_case()
}
