//! Literal chest USE on the real connection: durable MINT, quest refresh and XP settlement.
use super::quest_chest_postgres_tests_fixtures::scenario_with_multiple;

use super::TestResult;

#[test]
fn quest_chest_use_dispatch_mints_once_refreshes_and_pays_xp_once() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "chest dispatch e2e panicked")?
        .map_err(Into::into)
}

fn scenario() -> TestResult {
    scenario_with_multiple(false)
}
