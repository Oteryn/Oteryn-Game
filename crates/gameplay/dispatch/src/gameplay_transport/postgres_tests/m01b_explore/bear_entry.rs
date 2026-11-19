use super::*;

#[test]
fn m01b_bear_native_basement_entry_credits_once_before_key_and_stone() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e_fixtures::scenario_thread::run(
        "m01b-bear-basement-entry",
        move || {
            super::super::quest_world_callbacks_fixtures::scenario(
                "m01b/bear-room/s1/entry",
                Position {
                    x: 32145,
                    y: 32102,
                    floor: -11,
                },
                false,
            )
        },
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_bear_basement_entry duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
