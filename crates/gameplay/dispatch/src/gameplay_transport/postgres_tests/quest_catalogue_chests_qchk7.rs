use super::{TestResult, quest_catalogue_chests::phase_a_cohort};

#[test]
fn quest_catalogue_chests_qchk7_oriental_outfits_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["oriental_outfits_quest"])
}

#[test]
fn quest_catalogue_chests_qchk7_power_ring_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["power_ring_quest"])
}

#[test]
fn quest_catalogue_chests_qchk7_scale_armor_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["scale_armor_quest"])
}

#[test]
fn quest_catalogue_chests_qchk7_serpentine_tower_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["serpentine_tower_quest"])
}

#[test]
fn quest_catalogue_chests_qchk7_shaman_treasure_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["shaman_treasure_quest"])
}

#[test]
fn quest_catalogue_chests_qchk7_short_sword_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["short_sword_quest"])
}

#[test]
fn quest_catalogue_chests_qchk7_silver_brooch_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["silver_brooch_quest"])
}

#[test]
fn quest_catalogue_chests_qchk7_what_a_foolish_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["what_a_foolish_quest"])
}

#[test]
fn quest_catalogue_chests_qchk7_unlinked_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["unlinked"])
}
