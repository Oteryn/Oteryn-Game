//! Selected outfit routes use native NPC MINT, consumptive items and appearance facts.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

fn run_case(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let start = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run(name, || {
        super::quest_world_callbacks_fixtures::scenario(
            name,
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_OUTFIT_ITEMS POSTGRES_CASE_COMPLETED case={name} duration={:?} success={}",
        start.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn questpc7_fiend_personal_tokens_require_actual_native_addons() -> TestResult {
    run_case("questpc7-outfits-fiend")
}
#[test]
fn questpc7_inquisition_personal_tokens_require_actual_native_addons() -> TestResult {
    run_case("questpc7-outfits-inquisition")
}
