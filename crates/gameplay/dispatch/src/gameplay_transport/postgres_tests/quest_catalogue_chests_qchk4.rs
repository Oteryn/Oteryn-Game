//! PostgreSQL USE qualification for the QCHK-4 reward sources.
//!
//! Each cohort exercises every reward placement in its quest through USE,
//! including duplicate-command replay, cold-owner replay and repeat-use refusal.
//! The scope is reward delivery at the source chest.

use super::{TestResult, quest_catalogue_chests::phase_a_cohort};

#[test]
fn quest_catalogue_chests_qchk4_beregar_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["the_hidden_city_of_beregar_quest"])
}

#[test]
fn quest_catalogue_chests_qchk4_new_frontier_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["the_new_frontier_quest"])
}

#[test]
fn quest_catalogue_chests_qchk4_paradox_tower_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["the_paradox_tower_quest"])
}

#[test]
fn quest_catalogue_chests_qchk4_outlaw_camp_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["the_outlaw_camp_quest"])
}
