use super::*;

#[test]
fn m01b_minotaur_native_hell_entry_credits_once_and_replays() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e_fixtures::scenario_thread::run(
        "m01b-minotaur-hell-entry",
        move || {
            super::super::quest_world_callbacks_fixtures::scenario(
                "m01b/minotaur-hell/s1/minohell1",
                Position {
                    x: 32131,
                    y: 32066,
                    floor: -12,
                },
                false,
            )
        },
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_minotaur_hell_entry duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
