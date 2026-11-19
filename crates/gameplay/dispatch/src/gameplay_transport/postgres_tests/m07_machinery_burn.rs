//! Native Machinery trial: exact reusable tool, live level and once-per-site credit.
use super::m07_machinery_burn_fixtures::{run};

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

const NAME:&str="m07/machinery-ballista-01";
const SOURCE:Position=Position{x:32824,y:31708,floor:-7};

#[test]
fn m07_machinery_postgres_ballista_retains_tool_checks_level_and_resets()->TestResult {
    run(NAME,SOURCE)
}
#[test]
fn m07_machinery_postgres_ballista_refuses_level49_without_credit_or_tool_charge()->TestResult {
    run("m07/machinery-ballista-01-level49",SOURCE)
}
