use super::{TestResult, quest_catalogue_chests_fixtures::phase_a_cohort};

fn use_reward_chests(quests: &[&str]) -> TestResult {
    // The shared executor skips SQL without a route. Skip catalogue loading too.
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    phase_a_cohort(quests)
}

#[test]
fn quest_catalogue_chests_qchk7_oriental_outfits_use_replay_and_refusal() -> TestResult {
    use_reward_chests(&["oriental_outfits_quest"])
}

#[test]
fn quest_catalogue_chests_qchk7_power_ring_use_replay_and_refusal() -> TestResult {
    use_reward_chests(&["power_ring_quest"])
}

#[test]
fn quest_catalogue_chests_qchk7_scale_armor_use_replay_and_refusal() -> TestResult {
    use_reward_chests(&["scale_armor_quest"])
}

#[test]
fn quest_catalogue_chests_qchk7_serpentine_tower_use_replay_and_refusal() -> TestResult {
    use_reward_chests(&["serpentine_tower_quest"])
}

#[test]
fn quest_catalogue_chests_qchk7_shaman_treasure_use_replay_and_refusal() -> TestResult {
    use_reward_chests(&["shaman_treasure_quest"])
}

#[test]
fn quest_catalogue_chests_qchk7_short_sword_use_replay_and_refusal() -> TestResult {
    use_reward_chests(&["short_sword_quest"])
}

#[test]
fn quest_catalogue_chests_qchk7_silver_brooch_use_replay_and_refusal() -> TestResult {
    use_reward_chests(&["silver_brooch_quest"])
}

#[test]
fn quest_catalogue_chests_qchk7_what_a_foolish_use_replay_and_refusal() -> TestResult {
    use_reward_chests(&["what_a_foolish_quest"])
}

#[test]
fn quest_catalogue_chests_qchk7_unlinked_use_replay_and_refusal() -> TestResult {
    use_reward_chests(&["unlinked"])
}
