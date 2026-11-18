//! Forgotten Knowledge compatibility checks against TibiaWiki spoiler rev. 1200256.
use super::TestResult;

#[test]
fn albinius_ice_offering_requires_contactman_and_fifty_fish() -> TestResult {
    super::qengine_rewards::scenario("questpc2-albinius-portals-exact")
}
