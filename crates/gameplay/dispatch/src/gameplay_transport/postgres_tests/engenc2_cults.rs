use super::TestResult;
use super::engenc2_cults_fixtures::{PhaseScenario, phase_scenario};

#[test]
fn engenc2_cults_callback_native_phases_and_settlement_postgres() -> TestResult {
    phase_scenario(PhaseScenario::Cults { reenter: false })
}

#[test]
fn qmrevfix_cults_second_lever_after_victory_postgres() -> TestResult {
    phase_scenario(PhaseScenario::Cults { reenter: true })
}
