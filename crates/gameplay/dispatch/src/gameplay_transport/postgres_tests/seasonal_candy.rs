//! W05d Santa Candy Cane uses the real native food owner and wire replay.
#[test]
fn srv7_santa_candy_cane_native_food_wire_consumes_once_and_replays()
-> crate::bestiary_postgres_harness::TestResult {
    crate::gameplay_transport::item_use::item_use_postgres_tests::run_santa_candy_cane()
}
