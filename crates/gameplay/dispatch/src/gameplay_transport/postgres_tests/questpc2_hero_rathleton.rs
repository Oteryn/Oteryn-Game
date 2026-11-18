//! Existing main NPC rules, exercised through the durable command route.
//! Crystal 201189f: strange_pipe{,1,2,3}.lua:59-62.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc2_hero_four_pipe_greetings_grant_only_their_own_access_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("QUESTPC2_HERO_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("questpc2-hero-pipes", || {
        super::quest_world_callbacks_fixtures::scenario(
            "questpc2-hero-pipes",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC2_HERO_POSTGRES_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
