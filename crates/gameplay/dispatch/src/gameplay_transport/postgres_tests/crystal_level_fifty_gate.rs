//! Source level-50 gate using native XP and framed USE.

use super::TestResult;

#[test]
fn crystal_level_fifty_gate_refuses_49_and_opens_at_50() -> TestResult {
    super::m06_keys_fixtures::source_key_door(32874, 31974, -12, 5102, 1050)
}
