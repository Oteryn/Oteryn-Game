use super::{TestResult, parameter_spell_fixtures::scenario};

#[test]
fn spell_cooldowns_start_with_remaining_ms_without_ack_correction() -> TestResult {
    scenario(101)
}

#[test]
fn spell_cooldowns_publish_once_with_the_committed_cast() -> TestResult {
    scenario(102)
}

#[test]
fn spell_cooldowns_pair_gift_of_life_with_rescued_health() -> TestResult {
    scenario(103)
}

#[test]
fn spell_cooldowns_rejoin_preserves_exact_deadlines() -> TestResult {
    scenario(104)
}

#[test]
fn spell_cooldowns_snapshot_saturates_permanent_deadlines() -> TestResult {
    scenario(105)
}

#[test]
fn spell_cooldowns_snapshot_requires_the_bound_lease() -> TestResult {
    scenario(106)
}
