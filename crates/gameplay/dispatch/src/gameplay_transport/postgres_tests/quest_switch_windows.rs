//! Shared deadlines through native dispatch and its Channel owner.

use super::TestResult;

#[test]
fn timed_encounter_moves_the_admitted_group_at_expiry_and_resets_the_lever() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route_fixtures::timed_group_transfer(admin)
}

#[test]
fn timed_encounter_admits_and_ejects_only_the_top_players_on_two_pads() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route_fixtures::timed_pair_transfer(admin)
}

#[test]
fn ashmunrah_step_counts_only_the_stepping_characters_durable_switches() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route_fixtures::personal_switch_threshold(admin)
}

#[test]
fn reversible_switch_expires_and_an_old_deadline_cannot_close_a_reopened_window() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route_fixtures::timed_passage(admin)
}

#[test]
fn cathedral_selected_note_advances_the_mission_and_expires_from_the_wall() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route_fixtures::cathedral_note(admin)
}

#[test]
fn lord_chamber_last_switch_starts_the_window_and_expiry_resets_the_entire_circuit() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route_fixtures::lord_switches(admin)
}
