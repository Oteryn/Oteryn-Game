use super::*;

#[test]
fn m01b_elvenbane_dragon_floor_requires_first_native_visit() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-elvenbane-dragon", || {
            super::super::quest_world_callbacks_fixtures::scenario(
                "m01b/elvenbane/s2/entry",
                Position {
                    x: 32591,
                    y: 31645,
                    floor: -5,
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_elvenbane_dragon duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
