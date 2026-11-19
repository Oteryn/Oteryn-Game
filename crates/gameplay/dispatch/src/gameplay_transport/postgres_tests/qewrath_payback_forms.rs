//! Crystal 201189f8 data-global/scripts/quests/wrath_of_the_emperor/creaturescripts_zalamon_kill.lua:1-42,

use super::TestResult;
use super::qewrath_payback_forms_fixtures::{LEVER, OUTSIDE_TIMEOUT};

#[test]
fn payback_advances_exact_deaths_and_releases_the_room_on_victory_or_timeout() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("qewrath-payback", || {
        for name in ["qewrath-payback-victory", "qewrath-payback-timeout"] {
            super::quest_world_callbacks_fixtures::scenario(name, LEVER, false)?;
        }
        Ok(())
    })
}

#[test]
fn payback_lever_admits_the_staged_player_while_the_operator_stays_outside() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(OUTSIDE_TIMEOUT, || {
        super::quest_world_callbacks_fixtures::scenario(OUTSIDE_TIMEOUT, LEVER, false)
    })
}
