use super::TestResult;
use super::engenc2_cults_fixtures::{PhaseScenario, phase_scenario};

#[test]
fn gravedg_lich_knights_two_players_sirs_switch_credit_and_exit_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    phase_scenario(PhaseScenario::Sirs)
}

#[test]
fn gravedg_lich_knights_two_players_osam_channel_and_timeout_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    phase_scenario(PhaseScenario::EarlOsam)
}

#[test]
fn gravedg_lich_knights_two_players_krule_cycles_forms_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    phase_scenario(PhaseScenario::DukeKrule)
}

#[test]
fn gravedg_count_two_players_shield_timeout_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    phase_scenario(PhaseScenario::CountVlarkorth)
}

#[test]
fn gravedg_azaram_two_players_soul_phase_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    phase_scenario(PhaseScenario::LordAzaram)
}

#[test]
fn gravedg_azaram_five_player_lever_and_credit_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    phase_scenario(PhaseScenario::FivePlayerCredit {
        azaram: true,
    })
}

#[test]
fn gravedg_count_five_player_lever_and_credit_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    phase_scenario(PhaseScenario::FivePlayerCredit {
        azaram: false,
    })
}
