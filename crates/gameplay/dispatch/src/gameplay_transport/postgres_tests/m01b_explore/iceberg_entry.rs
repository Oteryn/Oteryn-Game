use super::*;

#[test]
fn m01b_chakoya_native_personal_iceberg_entry_requires_current_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-iceberg-entry", || {
        super::super::quest_world_callbacks_fixtures::scenario(
            "m01b/chakoya-iceberg-mini-world-change/s1/entry",
            Position {
                x: 32622,
                y: 32585,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_iceberg_entry duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
