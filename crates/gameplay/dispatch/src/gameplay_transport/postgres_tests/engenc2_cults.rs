//! Real callback admission, all Cults native phases, and the existing durable quest writer.
use super::TestResult;
use super::engenc2_cults_fixtures::encounter_scenario;

#[test]
fn engenc2_cults_callback_native_phases_and_settlement_postgres() -> TestResult {
    encounter_scenario(false, None)
}

#[test]
fn qmrevfix_cults_second_lever_after_victory_postgres() -> TestResult {
    encounter_scenario(true, None)
}
