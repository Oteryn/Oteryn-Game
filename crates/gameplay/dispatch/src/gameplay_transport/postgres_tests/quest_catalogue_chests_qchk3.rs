//! Source-placement reward qualification for the QCHK-3 quest chests.
use super::{TestResult, quest_catalogue_chests::phase_a_cohort};

#[test]
fn quest_catalogue_chests_qchk3_pits_of_inferno_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["the_pits_of_inferno_quest"])
}

#[test]
fn quest_catalogue_chests_qchk3_explorer_society_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["the_explorer_society_quest"])
}

#[test]
fn quest_catalogue_chests_qchk3_steel_helmet_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["steel_helmet_quest"])
}
