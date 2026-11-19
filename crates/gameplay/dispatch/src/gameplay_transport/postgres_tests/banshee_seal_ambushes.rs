use super::TestResult;

#[test]
fn banshee_hidden_flame_spawns_the_ambush_once_and_teleports() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::banshee_seal_ambushes_fixtures::run_hidden_flame()
}

#[test]
fn banshee_demonrage_spawns_two_warlocks_only_on_first_step() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::banshee_seal_ambushes_fixtures::run_demonrage()
}
