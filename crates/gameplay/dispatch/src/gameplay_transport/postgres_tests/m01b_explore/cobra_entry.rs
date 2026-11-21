use super::*;
use super::super::m01b_explore_fixtures::cobra_entry::*;

#[test]
fn m01b_grave_danger_cobra_native_visit_requires_twelve_graves_and_both_boss_stages() -> TestResult
{
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-cobra-visit", || {
        super::super::quest_world_callbacks_fixtures::scenario(
            "m01b/grave-danger/s4/entry",
            Position {
                y: START.y - 1,
                ..START
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_cobra_visit duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
