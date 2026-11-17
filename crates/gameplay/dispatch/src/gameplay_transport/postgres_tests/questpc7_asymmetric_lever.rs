//! Source-exact Giant Smithhammer lever, including both no-op combinations.
use super::TestResult;

#[test]
fn questpc7_giant_smithhammer_asymmetric_lever_all_four_states() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let start = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("questpc7-asymmetric", || {
        super::quest_world_callbacks::scenario(
            "giant-smithhammer",
            oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 32779,
                y: 32230,
                floor: -8,
            },
            false,
        )
    })?;
    eprintln!(
        "QUESTPC7_ASYMMETRIC POSTGRES_CASE_COMPLETED duration={:?}",
        start.elapsed()
    );
    Ok(())
}
