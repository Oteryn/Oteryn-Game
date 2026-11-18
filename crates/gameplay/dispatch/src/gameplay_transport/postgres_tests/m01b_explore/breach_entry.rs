use super::*;

#[test]
fn m01b_heart_native_zao_breach_requires_level_and_current_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-heart-zao-entry", move || {
            super::super::quest_world_callbacks_fixtures::scenario(
                "m01b/heart-of-destruction/s1/entry",
                Position {
                    x: 32118,
                    y: 31355,
                    floor: -13,
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_heart_zao_entry duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
