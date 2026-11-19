//! M07 catapults use the same retained-tool, level and personal-credit authority as ballistae.
use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

const SOURCE:Position=Position{x:32909,y:31733,floor:-7};

#[test]
fn m07_machinery_postgres_catapult_retains_tool_checks_level_and_resets()->TestResult {
    super::m07_machinery_burn_fixtures::run("m07/machinery-catapult-01",SOURCE)
}
#[test]
fn m07_machinery_postgres_catapult_refuses_level49_without_credit_or_tool_charge()->TestResult {
    super::m07_machinery_burn_fixtures::run("m07/machinery-catapult-01-level49",SOURCE)
}
