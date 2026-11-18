//! Tereban's complete eight-material hand-in programme through NPC dispatch.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;


#[test]
fn fathers_burden_eight_handins_cape_experience_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("fathers-burden-handins", || {
        super::quest_world_callbacks_fixtures::scenario(
            "fathers-burden-handins",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    println!(
        "POSTGRES_CASE_COMPLETED fathers_burden_eight_handins_cape_experience_and_replay duration={:?}",
        started.elapsed()
    );
    Ok(())
}
