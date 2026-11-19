//! M16: real Sterling dialogue commits a personal fact, then the read adapter re-proves its controller.

use super::TestResult;

#[test]
fn questpc5_personal_map_query_reproves_controller_and_reads_postgres_facts() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-questpc5-personal-map-query")
}
