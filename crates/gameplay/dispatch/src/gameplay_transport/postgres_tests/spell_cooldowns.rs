use super::{TestResult, parameter_spell_fixtures};

#[test]
fn spell_cooldowns_follow_committed_indexed_and_spoken_casts() -> TestResult {
    parameter_spell_fixtures::scenario(parameter_spell_fixtures::COOLDOWN_CASE)
}

#[test]
fn spell_cooldowns_restore_running_deadlines_on_rejoin() -> TestResult {
    parameter_spell_fixtures::scenario(parameter_spell_fixtures::COOLDOWN_REJOIN)
}

#[test]
fn spell_cooldowns_follow_the_recipient_gift_of_life_commit() -> TestResult {
    parameter_spell_fixtures::scenario(parameter_spell_fixtures::COOLDOWN_GIFT)
}

#[test]
fn spell_cooldowns_send_the_first_liveness_ack_correction() -> TestResult {
    parameter_spell_fixtures::scenario(parameter_spell_fixtures::COOLDOWN_RTT)
}

#[test]
fn spell_cooldowns_retry_a_failed_publication_after_cast_commit() -> TestResult {
    parameter_spell_fixtures::scenario(parameter_spell_fixtures::COOLDOWN_RETRY)
}
