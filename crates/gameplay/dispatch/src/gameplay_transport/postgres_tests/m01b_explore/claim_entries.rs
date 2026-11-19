use super::*;

#[test]
fn m01b_frost_dragon_native_lair_entry_requires_current_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-frost-dragon-entry", move || {
            super::super::quest_world_callbacks_fixtures::scenario(
                "m01b/frost-dragon/s1/okolnir",
                Position {
                    x: 32146,
                    y: 31431,
                    floor: -8,
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_frost_dragon_entry duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
