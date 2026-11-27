use crate::bestiary_postgres_harness::{TestResult, configured_admin};

#[test]
fn crystal_pear_debits_nested_inventory_through_wire_dispatch() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    crate::gameplay_transport::item_use::item_use_postgres_tests::run_crystal_ordinary_food_wire(admin, 3584, 60)
}

#[test]
fn crystal_confection_feeds_for_converted_seconds_through_wire_dispatch() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    crate::gameplay_transport::item_use::item_use_postgres_tests::run_crystal_ordinary_food_wire(
        admin, 48254, 780,
    )
}
