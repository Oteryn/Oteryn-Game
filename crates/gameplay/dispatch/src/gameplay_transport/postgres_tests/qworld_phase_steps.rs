//! M33 drives both retained Feaster portal phases through real STEP dispatch.

use super::TestResult;

#[test]
fn m33_feaster_initial_phase_uses_first_portal_and_live_requirements() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qworld-phase-0")
}
#[test]
fn m33_feaster_purification_phase_uses_second_portal_and_replays_once() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qworld-phase-5")
}
