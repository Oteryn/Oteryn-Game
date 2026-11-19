//! Gerimor's seven initial assignments; no boss completion or reward seeds.

use super::TestResult;


#[test]
fn questpc2_gerimor_seven_assignments_require_exact_selection_and_live_confirmation() -> TestResult {
    super::qengine_rewards_fixtures::scenario("questpc2-gerimor-assignments")
}
