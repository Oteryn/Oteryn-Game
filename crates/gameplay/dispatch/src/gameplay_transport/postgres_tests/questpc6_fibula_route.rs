//! Source Fibula level gate using native XP and framed USE, without seeded action attributes.

use super::TestResult;

#[test]
fn questpc6_fibula_source_expertise_level_gate_refusal_and_replay() -> TestResult {
    super::m06_keys_fixtures::source_key_door(32212, 32435, -10, 1050)
}
