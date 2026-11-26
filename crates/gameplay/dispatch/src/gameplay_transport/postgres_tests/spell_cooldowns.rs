use super::{TestResult, parameter_spell_fixtures};

#[test]
fn spell_cooldowns_follow_committed_indexed_and_spoken_casts() -> TestResult {
    parameter_spell_fixtures::scenario(parameter_spell_fixtures::COOLDOWN_CASE)
}

#[test]
fn spell_cast_omits_cooldowns_without_negotiated_presentation() -> TestResult {
    parameter_spell_fixtures::scenario(parameter_spell_fixtures::COOLDOWN_WITHOUT_PRESENTATION_CASE)
}
