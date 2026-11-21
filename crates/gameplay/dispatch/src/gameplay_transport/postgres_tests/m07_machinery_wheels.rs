//! Six actual two-part war wheels share the retained-tool and personal-credit authority.
use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

const SOURCE:Position=Position{x:32964,y:31814,floor:-7};

#[test]
fn m07_machinery_postgres_wheel_retains_tool_checks_level_and_resets()->TestResult {
    super::m07_machinery_burn_fixtures::run("m07/machinery-wheel-01",SOURCE)
}
#[test]
fn m07_machinery_postgres_wheel_refuses_level49_without_credit_or_tool_charge()->TestResult {
    super::m07_machinery_burn_fixtures::run("m07/machinery-wheel-01-level49",SOURCE)
}
