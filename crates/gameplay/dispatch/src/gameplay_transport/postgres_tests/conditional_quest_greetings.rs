//! Conditional donor greetings cannot infer an initial quest stage from a failed copy load.
//! Crystal 201189f: a_sleeping_dragon.lua:49-55; jack_springer.lua:61-70.
//! Exercise the actual NPC command route, including its ordinary greeting fallback.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn conditional_quest_greetings_missing_copy_refuse_in_real_npc_path() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("CONDITIONAL_QUEST_GREETINGS_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("conditional-quest-greetings", || {
        super::quest_world_callbacks_fixtures::scenario(
            "conditional-quest-greetings",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}
