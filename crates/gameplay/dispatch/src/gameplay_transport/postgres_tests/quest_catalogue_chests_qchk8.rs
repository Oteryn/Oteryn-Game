//! QCHK-8 reward chests at their authored source placements.

use super::TestResult;
use super::quest_catalogue_chests::phase_a_cohort;

#[test]
fn quest_catalogue_chests_qchk8_six_rubies_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["six_rubies_quest"])
}

#[test]
fn quest_catalogue_chests_qchk8_spike_sword_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["spike_sword_quest"])
}

#[test]
fn quest_catalogue_chests_qchk8_stealth_ring_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["stealth_ring_quest"])
}

#[test]
fn quest_catalogue_chests_qchk8_strong_potions_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["strong_potions_quest"])
}

#[test]
fn quest_catalogue_chests_qchk8_the_shattered_isles_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["the_shattered_isles_quest"])
}

#[test]
fn quest_catalogue_chests_qchk8_the_white_raven_monastery_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["the_white_raven_monastery_quest"])
}

#[test]
fn quest_catalogue_chests_qchk8_throwing_star_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["throwing_star_quest"])
}

#[test]
fn quest_catalogue_chests_qchk8_to_outfox_a_fox_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["to_outfox_a_fox_quest"])
}

#[test]
fn quest_catalogue_chests_qchk8_vampire_hunter_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["vampire_hunter_quest"])
}
