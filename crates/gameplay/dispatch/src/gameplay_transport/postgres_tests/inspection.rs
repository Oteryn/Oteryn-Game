//! INSPECT-1 through PostgreSQL admission, native map snapshots and real dispatch.
use super::{TestResult, item_look_fixtures as item_look};

#[test]
fn inspection_postgres_inventory_ground_catalogue_and_private_player_dispatch() -> TestResult {
    item_look::large_stack(|| item_look::scenario_with_inspection(Some(true)))
}
#[test]
fn inspection_postgres_unselected_capability_rejects_command() -> TestResult {
    item_look::large_stack(|| item_look::scenario_with_inspection(Some(false)))
}
#[test]
fn inspection_postgres_trade_counterpart_requires_current_offer_and_leaf_index() -> TestResult {
    super::player_trade_fixtures::run_scenario(true)
}
