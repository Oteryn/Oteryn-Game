//! Crystal 201189f8 Svargrond Arena south portal, movements_arena_pit.lua:1-21,70-72.
use super::TestResult;

#[test]
fn south_portal_concedes_the_live_arena_without_a_leave_command() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("qcx61_south_portal", || {
        super::arena_match::south_portal_concession()
    })
}
