use super::*;

#[test]
fn m01b_hero_native_voice_approach_requires_current_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-hero-voice-entry", move || {
            super::super::quest_world_callbacks_fixtures::scenario(
                "m01b/hero-of-rathleton/s1/entry",
                Position {
                    x: 33533,
                    y: 31951,
                    floor: -14,
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_hero_voice_entry duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
