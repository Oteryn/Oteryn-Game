//! Test-only access to the existing harness's private connection configuration.
use super::{DurabilityRoot, Harness, TestResult};

#[allow(
    dead_code,
    reason = "used by the dispatch child tests sharing this fixture"
)]
pub(crate) fn reopen_root(harness: &Harness) -> TestResult<DurabilityRoot> {
    Ok(DurabilityRoot::connect_test_runtime(&harness.database.url)?)
}
