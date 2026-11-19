use super::{TestResult, goshnar_fluid_fixtures};

#[test]
fn goshnar_blood_use_with_checks_world_policy_then_moves_two_staged_players() -> TestResult {
    goshnar_fluid_fixtures::goshnar_blood_use_with_checks_world_policy_then_moves_two_staged_players_scenario()
}

#[test]
fn goshnar_blood_use_respects_the_open_pvp_stance_of_a_landing_occupant() -> TestResult {
    goshnar_fluid_fixtures::goshnar_blood_use_respects_the_open_pvp_stance_of_a_landing_occupant_scenario()
}

#[test]
fn goshnar_blood_use_accepts_a_native_gravestone_with_an_inscription() -> TestResult {
    goshnar_fluid_fixtures::goshnar_blood_use_accepts_a_native_gravestone_with_an_inscription_scenario()
}
