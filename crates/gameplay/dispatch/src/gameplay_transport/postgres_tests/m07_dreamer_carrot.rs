//! Literal UID2241 on synthetic source geometry; real STEP and fenced carried inputs.
//! This qualifies the owner, not a recovered production-map UID correspondence.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn m07_carrot_premium_entry_consumes_exactly_one_and_replays() -> TestResult {
    trial("m07-carrot-premium")
}
#[test]
fn m07_carrot_free_account_keeps_its_offering_and_gets_source_refusal() -> TestResult {
    trial("m07-carrot-free")
}
fn trial(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("m07-carrot", move || {
        super::quest_world_callbacks_fixtures::scenario(
            name,
            Position {
                x: 100,
                y: 100,
                floor: -9,
            },
            false,
        )
    })
}
