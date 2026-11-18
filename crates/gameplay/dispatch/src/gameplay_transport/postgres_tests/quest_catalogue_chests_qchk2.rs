use super::{TestResult, quest_catalogue_chests_fixtures::phase_a_cohort};

#[test]
fn quest_catalogue_chests_qchk2_secret_service_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["secret_service_quest"])
}

#[test]
fn quest_catalogue_chests_qchk2_shadows_of_yalahar_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["shadows_of_yalahar_quest"])
}
