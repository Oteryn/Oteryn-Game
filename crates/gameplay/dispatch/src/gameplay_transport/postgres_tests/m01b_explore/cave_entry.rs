use super::*;

#[test]
fn m01b_dragon_corpse_native_cave_entry_credits_once_and_replays() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e_fixtures::scenario_thread::run(
        "m01b-dragon-corpse-entry",
        move || {
            super::super::quest_world_callbacks_fixtures::scenario(
                "m01b/dragon-corpse/s1/entry",
                Position {
                    x: 32147,
                    y: 32207,
                    floor: -7,
                },
                false,
            )
        },
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_dragon_corpse_entry duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
