use super::*;

#[test]
fn m01b_ferumbras_native_arrival_requires_level_and_current_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-ferumbras-entry", move || {
            super::super::quest_world_callbacks_fixtures::scenario(
                "m01b/ferumbras-ascension/s1/entry",
                Position {
                    x: 33276,
                    y: 32390,
                    floor: -8,
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_ferumbras_entry duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
