//! Mandatory NPC introduction, native shrine bindings and atomic assembly.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc8_mystery_assembly_consumes_all_three_or_none() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-mystery-assembly", || {
        super::quest_world_callbacks_fixtures::scenario(
            "authored-mystery-assembly",
            Position {
                x: 33836,
                y: 32352,
                floor: -4,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_mystery_assembly_consumes_all_three_or_none duration={:?}",
        started.elapsed()
    );
    Ok(())
}
