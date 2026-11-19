//! Crystal 201189f8 movements_arena_trophy.lua:19-25 and svargrond_arena.lua:175-190.
use super::TestResult;

#[test]
fn arena_trophy_look_projects_the_original_victor_name() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "qcx61-trophy-look",
        super::arena_match_fixtures::trophy_look,
    )
}
