//! I14: newly admitted mana potion and fish use the real wire, durable owner and replay.
#[test]
fn i14_mana_potion_and_fish_commit_through_wire_dispatch_on_postgres()
-> crate::bestiary_postgres_harness::TestResult {
    crate::gameplay_transport::item_use::item_use_postgres_tests::run_matching_crystal_consumables()
}
