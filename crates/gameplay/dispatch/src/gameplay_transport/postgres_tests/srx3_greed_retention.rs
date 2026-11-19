//! Real entry timers, a canonical death, and an interrupted native soul transform.
use super::TestResult;

#[test]
fn srx3_greed_original_callbacks_survive_timer_and_transform_cancellation_postgres() -> TestResult {
    super::srx3_greed_callbacks_fixtures::run_source_entry(true)
}
