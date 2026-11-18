//! QCHK-5 reward delivery at authored chest placements.
//! The standalone cases also check that the chest cohort covers the full source claim inventory.

use super::TestResult;
use super::quest_catalogue_chests::{phase_a_cohort, standalone_claim_cohort};

#[test]
fn quest_catalogue_chests_qchk5_the_queen_of_the_banshees_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["the_queen_of_the_banshees_quest"])
}

#[test]
fn quest_catalogue_chests_qchk5_the_thieves_guild_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["the_thieves_guild_quest"])
}

#[test]
fn quest_catalogue_chests_qchk5_tomes_of_knowledge_use_replay_and_refusal() -> TestResult {
    standalone_claim_cohort(
        &["tomes_of_knowledge_quest"],
        "quest_catalogue_chests_qchk5_tomes_of_knowledge_use_replay_and_refusal",
    )
}

#[test]
fn quest_catalogue_chests_qchk5_to_blind_the_enemy_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["to_blind_the_enemy_quest"])
}

#[test]
fn quest_catalogue_chests_qchk5_newhaven_legion_helmet_use_replay_and_refusal() -> TestResult {
    standalone_claim_cohort(
        &["newhaven_legion_helmet_quest"],
        "quest_catalogue_chests_qchk5_newhaven_legion_helmet_use_replay_and_refusal",
    )
}
