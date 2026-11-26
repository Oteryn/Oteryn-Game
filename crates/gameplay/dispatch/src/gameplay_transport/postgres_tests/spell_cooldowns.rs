use super::{TestResult, parameter_spell_fixtures};

#[test]
fn spell_cooldowns_follow_committed_indexed_and_spoken_casts() -> TestResult {
    parameter_spell_fixtures::scenario(parameter_spell_fixtures::COOLDOWN_CASE)
}
