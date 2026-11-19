//! Actual pinned Greed entry and native spawn chain; no seeded catalogue.
use super::srx3_greed_callbacks_fixtures::{run_source_entry};

use super::TestResult;

#[test]
fn srx3_greed_actual_source_entry_and_native_minion_chain_postgres() -> TestResult {
    run_source_entry(false)
}
