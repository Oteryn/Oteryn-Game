use super::*;

#[test]
fn m01b_behemoth_native_floor_entry_requires_level_eighty_and_current_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e_fixtures::scenario_thread::run(
        "m01b-behemoth-floor-entry",
        move || {
            super::super::quest_world_callbacks_fixtures::scenario(
                "m01b/behemoth/s1/entry",
                Position {
                    x: 33235,
                    y: 31695,
                    floor: -12,
                },
                false,
            )
        },
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_behemoth_floor_entry duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
