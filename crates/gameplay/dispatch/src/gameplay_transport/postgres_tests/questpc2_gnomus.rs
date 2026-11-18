//! Initial Gnomus ordnance admission only; no escort or warzone completion seeds.

use super::TestResult;


#[test]
fn questpc2_gnomus_ordnance_requires_live_confirmation_and_commits_initial_tracks() -> TestResult {
    super::qengine_rewards_fixtures::scenario("questpc2-gnomus-ordnance")
}
