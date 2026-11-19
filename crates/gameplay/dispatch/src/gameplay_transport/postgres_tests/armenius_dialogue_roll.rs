use super::TestResult;
use super::armenius_dialogue_roll_fixtures::{NAME, TRIGGER};

#[test]
fn armenius_dialogue_roll_persists_each_route_and_earns_the_basement_achievement() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        for name in [
            "armenius-dialogue-roll-1",
            "armenius-dialogue-roll-2",
            "armenius-dialogue-roll-3",
        ] {
            super::quest_world_callbacks_fixtures::scenario(name, TRIGGER, false)?;
        }
        Ok(())
    })
}

#[test]
fn armenius_stage_two_accepts_nonliteral_messages_and_respects_earlier_exact_branches() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("armenius-stage-two", || {
        super::quest_world_callbacks_fixtures::scenario(
            "armenius-dialogue-roll-repeat-2",
            TRIGGER,
            false,
        )
    })
}
