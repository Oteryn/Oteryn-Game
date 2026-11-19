//! Arkulius' branches share one bounded custody snapshot (Crystal 201189f8,
use super::TestResult;
use super::qespike_arkulius_fixtures::{NAME, PAYOUT, START};

#[test]
fn arkulius_dialogue_reads_carried_input_with_eleven_program_item_identities() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, START, false)
    })
}

#[test]
fn arkulius_dialogue_payout_admits_native_reward_before_consuming_the_input() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(PAYOUT, || {
        super::quest_world_callbacks_fixtures::scenario(PAYOUT, START, false)
    })
}
