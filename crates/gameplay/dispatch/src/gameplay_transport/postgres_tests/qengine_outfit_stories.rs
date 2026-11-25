//! Complete outfit reward stories: real conversations, exact custody and parent ownership.
use super::TestResult;


#[test]
fn qengine_l2_battle_mage_whole_outfit_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l2-battle-mage-story")
}
#[test]
fn qengine_l2_festive_whole_outfit_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l2-festive-story")
}
#[test]
fn qengine_l2_elementalist_whole_outfit_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l2-elementalist-story")
}

#[test]
fn qengine_l3_demon_whole_outfit_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l3-demon-story")
}
#[test]
fn qengine_l3_demon_true_dedication_also_grants_when_shield_is_last() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l3-demon-helmet-first-story")
}
#[test]
fn qengine_l3_glooth_engineer_high_votes_preserve_source_priority() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l3-glooth-engineer-story")
}
#[test]
fn qengine_l3_dark_trails_base_and_glooth_addons_follow_live_votes() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l3-dark-trails-story")
}

#[test]
fn qengine_l10_golden_complete_commission_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l10-golden-story")
}

#[test]
fn qengine_l4_nobleman_paid_outfit_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l4-nobleman-story")
}

#[test]
fn qengine_l4_royal_costume_full_token_exchange_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l4-royal-costume-story")
}

#[test]
fn qengine_l4_pirate_full_source_outfit_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l4-pirate-story")
}

#[test]
fn qengine_l5_oriental_female_complete_source_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l5-oriental-story")
}

#[test]
fn qengine_l5_norseman_and_formorgar_complete_outfit_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l5-norseman-story")
}

#[test]
fn qengine_l6_druid_rook_complete_available_body_outfit_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l6-druid-rook-story")
}

#[test]
fn qengine_l6_citizen_rook_complete_source_outfit_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l6-citizen-rook-story")
}

#[test]
fn qengine_l6_beggar_female_complete_source_outfit_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l6-beggar-female-story")
}
#[test]
fn qengine_l6_beggar_male_complete_source_outfit_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l6-beggar-male-story")
}

#[test]
fn qengine_l7_knight_complete_source_outfit_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l7-knight-story")
}
#[test]
fn qengine_l7_druid_complete_source_outfit_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l7-druid-story")
}
#[test]
fn qengine_l7_assassin_complete_source_outfit_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l7-assassin-story")
}

#[test]
fn qengine_l8_hunter_female_source_outfit_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l8-hunter-female-story")
}
#[test]
fn qengine_l8_hunter_male_source_outfit_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l8-hunter-male-story")
}
#[test]
fn qengine_l8_mage_female_source_outfit_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l8-mage-female-story")
}
#[test]
fn qengine_l8_mage_male_source_outfit_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l8-mage-male-story")
}
#[test]
fn qengine_l8_dream_warden_source_outfit_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l8-dream-warden-story")
}

#[test]
fn qb_outfit_progress_advances_from_main_definition() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qb-main-predecessor")
}

#[test]
fn qb_citizen_owned_hat_refuses_mainland_exchange_without_debit() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qb-owned-hat")
}

#[test]
fn qb_citizen_free_account_accepts_mainland_exchange() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qb-free-account")
}

#[test]
fn qb_citizen_hanna_opens_recipe_for_owned_hat() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qb-start-owned-hat")
}

#[test]
fn qb_citizen_hanna_opens_recipe_for_free_account() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qb-start-free-account")
}

#[test]
fn qb_crystal_citizen_lubo_completed_reply() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qb-crystal-lubo-completed")
}

#[test]
fn qb_crystal_citizen_amber_completed_reply() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qb-crystal-amber-completed")
}

#[test]
fn qb_crystal_citizen_lubo_refuses_owned_backpack_without_debit() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qb-crystal-lubo-progress")
}

#[test]
fn qb_crystal_citizen_lubo_accepts_free_character() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qb-crystal-lubo-free-account")
}

#[test]
fn qb_crystal_citizen_norma_refuses_second_hat_without_debit() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qb-crystal-norma-restart")
}

#[test]
fn qb_crystal_citizen_lubo_mission_zero() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qb-crystal-lubo-mission-zero")
}

#[test]
fn qb_crystal_citizen_amber_mission_zero() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qb-crystal-amber-mission-zero")
}

#[test]
fn qb_crystal_citizen_norma_mission_zero() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qb-crystal-norma-mission-zero")
}

#[test]
fn qb_crystal_citizen_hanna_mission_zero() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qb-crystal-hanna-mission-zero")
}

#[test]
fn qb_crystal_citizen_hanna_completed() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qb-crystal-hanna-completed")
}
