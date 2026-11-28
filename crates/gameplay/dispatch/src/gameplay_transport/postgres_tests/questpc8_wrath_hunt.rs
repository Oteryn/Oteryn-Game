//! Selected Wrath stage6: native imperial deaths, exact4/6 quotas, real Zlak.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc8_wrath_imperial_hunt_native_exact_quotas_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-wrath-hunt", || {
            for case in ["authored-wrath-hunt", "authored-wrath-hunt#legacy"] {
                super::quest_world_callbacks_fixtures::scenario(
                    case,
                    Position {
                        x: 33082,
                        y: 31220,
                        floor: -7,
                    },
                    false,
                )?;
            }
            Ok(())
        });
    eprintln!(
        "POSTGRES_CASE_FINISHED questpc8_wrath_imperial_hunt duration={:?} result={result:?}",
        started.elapsed()
    );
    result
}
