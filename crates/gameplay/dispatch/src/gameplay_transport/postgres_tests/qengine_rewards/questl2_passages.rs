use super::*;
use super::super::qengine_rewards_fixtures::questl2_passages::*;

#[test]
fn questl2_passage_batch_white_raven_full_journey() -> TestResult {
    probe("qengine-rewards-l2-white-raven", &["/l2-white-raven-brooch"])
}
