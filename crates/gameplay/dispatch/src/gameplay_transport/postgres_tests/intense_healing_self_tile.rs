use super::TestResult;
use super::fixture::run;
use super::item_look_fixtures::large_stack;
use super::rune_use_bridge_fixtures::{Case, run_case};

#[test]
fn rune_use_bridge_postgres_intense_healing_self_tile() -> TestResult {
    large_stack(|| run(async |admin| run_case(admin, Case::IntenseHealingSelfTile).await))
}
