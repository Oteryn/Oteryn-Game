use super::*;

#[test]
fn unnatural_dance_postgres_full_wire_route_achievement_replay_stale_and_native_gate() -> TestResult
{
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e_fixtures::scenario_thread::run("unnatural-dance", || {
        super::scenario(
            "unnatural-selection-dance-2",
            Position {
                x: 32991,
                y: 31496,
                floor: -1,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m40-unnatural-dance duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
