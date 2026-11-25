use super::{
    TestResult, l6_paradox_story_fixtures as story, paradox_tower_journey_fixtures::{FORFEIT, NAME},
    quest_world_callbacks_fixtures as wire,
};

#[test]
fn paradox_tower_full_journey() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        wire::scenario(NAME, story::start(), false)
    })
}

#[test]
fn paradox_tower_full_journey_forfeits_talons_after_the_riddle() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run(FORFEIT, || {
        wire::scenario(FORFEIT, story::start(), false)
    })
}
