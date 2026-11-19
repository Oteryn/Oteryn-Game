//! Crystal 201189f8 data-global/lib/quests/svargrond_arena.lua:217-232.
use super::TestResult;

#[test]
fn native_arena_entry_retires_only_the_original_top_ground_item() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "qcx61-pit-ground-reset",
        super::arena_match_fixtures::native_pit_ground_reset,
    )
}
