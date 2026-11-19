//! Four Albinius portal offerings earned after the real five-tome contribution.

use super::TestResult;


#[test]
fn questpc2_albinius_portals_require_fifty_exact_items_and_live_confirmation() -> TestResult {
    super::qengine_rewards_fixtures::scenario("questpc2-albinius-portals-exact")
}

#[test]
fn questpc2_albinius_portals_combined_source_ifs_commit_all_four_offerings() -> TestResult {
    super::qengine_rewards_fixtures::scenario("questpc2-albinius-portals-combined")
}
