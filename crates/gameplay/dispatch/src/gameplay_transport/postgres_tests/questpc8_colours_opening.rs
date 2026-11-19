//! Chosen blue path: exact envelope exchange and self-marking, never peer pairing.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc8_colours_native_envelope_and_self_marking_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-colours-opening", || {
        super::quest_world_callbacks_fixtures::scenario(
            "authored-colours-opening",
            Position {
                x: 33354,
                y: 31410,
                floor: -8,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_FINISHED questpc8_colours_opening duration={:?} result={result:?}",
        started.elapsed()
    );
    result
}
