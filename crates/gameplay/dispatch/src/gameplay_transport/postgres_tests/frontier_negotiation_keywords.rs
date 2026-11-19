//! Crystal 201189f8, data-global/npc/ongulf.lua:181-195.

use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn negotiation_acceptance_assigns_five_character_keywords_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        super::frontier_negotiation_keywords_fixtures::NAME,
        || {
            super::quest_world_callbacks_fixtures::scenario(
                super::frontier_negotiation_keywords_fixtures::NAME,
                Position {
                    x: 33294,
                    y: 31677,
                    floor: -15,
                },
                false,
            )
        },
    )
}
