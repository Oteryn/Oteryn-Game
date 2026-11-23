use super::{TestResult, srvenc_source_deaths_fixtures::scenario};

#[test]
fn cq16_black_knight_advances_active_avin_mission() -> TestResult {
    scenario(stringify!(cq16_black_knight_advances_active_avin_mission), "black_knight")
}

#[test]
fn nomad_credits_most_damage_party_instead_of_damage_roster() -> TestResult {
    scenario(stringify!(nomad_credits_most_damage_party_instead_of_damage_roster), "nomad")
}

#[test]
fn nomad_excludes_former_member_with_stale_top_damage_roster() -> TestResult {
    scenario(stringify!(nomad_excludes_former_member_with_stale_top_damage_roster), "nomad")
}
