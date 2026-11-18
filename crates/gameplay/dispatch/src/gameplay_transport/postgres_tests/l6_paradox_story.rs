//! Selected ordered contacts at original native NPC centers; no SQL progress seeds.
use super::TestResult;

fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::qengine_rewards_fixtures::scenario(name)
}
#[test]
fn l6_paradox_story_native_ordered() -> TestResult {
    run("qengine-rewards-questpc4-paradox-story-ordered")
}
#[test]
fn l6_paradox_story_native_out_of_order() -> TestResult {
    run("qengine-rewards-questpc4-paradox-story-out-of-order")
}
#[test]
fn l6_paradox_story_native_lesson_guard_and_replay() -> TestResult {
    run("qengine-rewards-questpc4-paradox-story-lesson-guard")
}
