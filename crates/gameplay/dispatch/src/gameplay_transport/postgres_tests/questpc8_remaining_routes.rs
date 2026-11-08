//! Full native routes for the remaining collection and travel programmes.
use super::{TestResult, quest_world_callbacks};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

fn scenario(name: &'static str, position: Position) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc8-remaining-route", move || {
        quest_world_callbacks::scenario(name, position, false)
    });
    eprintln!("POSTGRES_CASE_FINISHED questpc8_remaining_route {name} duration={:?} result={result:?}", started.elapsed());
    result
}

#[test]
fn questpc8_remaining_trader_complete_native_journey() -> TestResult {
    scenario("authored-trader", Position { x: 32307, y: 32834, floor: -7 })
}
#[test]
fn questpc8_remaining_treasure_island_paid_ship_tools_cave_return() -> TestResult {
    scenario("authored-treasure-island", Position { x: 32131, y: 32913, floor: -7 })
}
#[test]
fn questpc8_remaining_anniversary_rags_five_dragons_reward() -> TestResult {
    scenario("authored-anniversary", Position { x: 32359, y: 32234, floor: -7 })
}
