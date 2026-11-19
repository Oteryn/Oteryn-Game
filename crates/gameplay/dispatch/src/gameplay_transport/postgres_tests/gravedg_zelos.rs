use super::TestResult;
use super::engenc2_cults_fixtures::{PhaseScenario, phase_scenario};

#[test]
fn gravedg_zelos_two_players_regeneration_and_timeout_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    phase_scenario(PhaseScenario::Zelos {
        full_party: false,
    })
}

#[test]
fn gravedg_zelos_ten_player_lever_and_timeout_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    phase_scenario(PhaseScenario::Zelos {
        full_party: true,
    })
}
