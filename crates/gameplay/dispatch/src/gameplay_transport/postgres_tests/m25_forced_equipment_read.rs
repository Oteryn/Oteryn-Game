//! A live physical PUSH witness qualifies the grace participant's source-ground equipment read.
use super::TestResult;

#[test]
fn forced_reconnectable_push_reads_equipment_on_qualified_ground() -> TestResult {
    super::m25_ahau_exit_fixtures::push_equipment_scenario()
}
