//! Native daily construction circuit: held tools, exact machines, deliveries and captured deaths.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;


#[test]
fn questpc8_venore_native_daily_circuit_and_exact_repeat_boundary() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-venore", || {
        super::quest_world_callbacks_fixtures::scenario(
            "authored-venore",
            Position {
                x: 33016,
                y: 32100,
                floor: -6,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_FINISHED questpc8_venore_native_daily_circuit duration={:?} result={result:?}",
        started.elapsed()
    );
    result
}
