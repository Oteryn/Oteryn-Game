use super::*;

#[test]
fn m01b_ghoul_native_outer_passage_credits_once_before_key_door() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e_fixtures::scenario_thread::run(
        "m01b-ghoul-outer-passage",
        move || {
            super::super::quest_world_callbacks_fixtures::scenario(
                "m01b/ghoul-room/s1/entry",
                Position {
                    x: 32507,
                    y: 32175,
                    floor: -14,
                },
                false,
            )
        },
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_ghoul_outer_passage duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
