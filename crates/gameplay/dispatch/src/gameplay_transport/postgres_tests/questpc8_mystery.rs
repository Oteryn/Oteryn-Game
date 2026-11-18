//! Mandatory authored introduction through the native NPC receipt owner.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc8_mystery_lai_introduction_requires_access_and_is_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-mystery-intro", || {
        super::quest_world_callbacks_fixtures::scenario(
            "authored-mystery-intro",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_mystery_lai_introduction_requires_access_and_is_once duration={:?}",
        started.elapsed()
    );
    Ok(())
}
