//! Crystal 201189f8 src/game/game.cpp:1964-1970 and data-global/lib/quests/svargrond_arena.lua:193-222.
use super::TestResult;

#[test]
fn arena_spectator_cannot_move_carried_items_through_the_pit_floor() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "qcx61-spectator",
        super::arena_match_fixtures::spectator_transfer,
    )
}
