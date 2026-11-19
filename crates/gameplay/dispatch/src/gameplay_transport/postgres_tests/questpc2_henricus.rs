//! Henricus's source recruitment and first mission; no interrogation completion seed.

use super::TestResult;


#[test]
fn questpc2_henricus_recruitment_and_first_mission_are_topic_fenced_and_atomic() -> TestResult {
    super::qengine_rewards_fixtures::scenario("questpc2-henricus-start")
}
