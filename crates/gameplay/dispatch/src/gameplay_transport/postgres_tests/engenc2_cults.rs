use super::TestResult;
use super::engenc2_cults_fixtures::{PhaseScenario, phase_scenario};

#[test]
fn engenc2_cults_callback_native_phases_and_settlement_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    phase_scenario(PhaseScenario::Cults { reenter: false })
}

#[test]
fn qmrevfix_cults_second_lever_after_victory_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    phase_scenario(PhaseScenario::Cults { reenter: true })
}
