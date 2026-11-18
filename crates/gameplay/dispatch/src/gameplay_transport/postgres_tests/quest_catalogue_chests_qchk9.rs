//! Exercise each QCHK-9 quest's authored reward placements through production USE.

use super::{TestResult, quest_catalogue_chests::phase_a_cohort};

#[test]
fn quest_catalogue_chests_qchk9_waterfall_quest_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["waterfall_quest"])
}

#[test]
fn quest_catalogue_chests_qchk9_wedding_ring_quest_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["wedding_ring_quest"])
}

#[test]
fn quest_catalogue_chests_qchk9_skeleton_decoration_quest_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["skeleton_decoration_quest"])
}

#[test]
fn quest_catalogue_chests_qchk9_studded_legs_quest_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["studded_legs_quest"])
}

#[test]
fn quest_catalogue_chests_qchk9_targuna_quest_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["targuna_quest"])
}
