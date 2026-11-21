//! Pinned source selectors -> literal USE -> existing once-claim owner -> full
//! container custody, key/achievement facts, replay and atomic refusal.
use super::quest_container_chests_fixtures::{Case, execute};

use super::TestResult;

#[test]
fn quest_authored_placement_without_legacy_uid_mints_once_and_replays() -> TestResult {
    execute(Case::Authored, "QUESTPC8_AUTHORED_PLACEMENT")
}

#[test]
fn quest_container_chests_bear_bag_once_and_replay() -> TestResult {
    execute(Case::Bear, "W15B_BEAR_CONTAINER")
}
#[test]
fn quest_container_chests_key_attributes_are_atomic() -> TestResult {
    execute(Case::Key, "W15B_KEY_CONTAINER")
}
#[test]
fn quest_container_chests_annihilator_choices_share_storage_and_achievement() -> TestResult {
    execute(Case::Choice, "W15B_STORAGE_CHOICE")
}
#[test]
fn annihilator_reward_door_tracks_the_real_shared_claim() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    execute(Case::ChoiceDoor, "ANNIHILATOR_REWARD_DOOR")
}
#[test]
fn annihilator_captured_startup_chests_share_the_real_claim() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    execute(Case::ChoiceCaptured, "ANNIHILATOR_CAPTURED_CHESTS")
}
#[test]
fn quest_container_chests_authored_book_text_survives_replay() -> TestResult {
    execute(Case::Text, "W15B_TEXT_CONTAINER")
}
#[test]
fn quest_container_chests_leaf_text_is_frozen_and_replayed() -> TestResult {
    execute(Case::RootText, "W15B_LEAF_TEXT")
}
#[test]
fn quest_container_chests_crystal_key3002_is_minted_once() -> TestResult {
    execute(Case::CrystalKey, "W15B_CRYSTAL_KEY")
}
#[test]
fn quest_container_chests_full_definition_charges_mint_one_instance() -> TestResult {
    execute(Case::FullCharge, "W15B_FULL_CHARGES")
}
#[test]
fn quest_container_chests_full_backpack_writes_no_pending_claim() -> TestResult {
    execute(Case::Full, "W15B_CONTAINER_FULL")
}
#[test]
fn quest_container_chests_capacity_counts_every_child_before_reservation() -> TestResult {
    execute(Case::Weight, "W15B_CONTAINER_WEIGHT")
}

// Crystal 201189f:data-global/startup/tables/chest.lua:799-819. The three
// independent storage claims must survive both command and cold-owner replay.
#[test]
fn quest_demon_helmet_chests_steel_boots_once_and_replay() -> TestResult {
    execute(Case::DemonBoots, "QUESTS3_DEMON_BOOTS")
}
#[test]
fn quest_demon_helmet_chests_demon_helmet_once_and_replay() -> TestResult {
    execute(Case::DemonHelmet, "QUESTS3_DEMON_HELMET")
}
#[test]
fn quest_demon_helmet_chests_demon_shield_once_and_replay() -> TestResult {
    execute(Case::DemonShield, "QUESTS3_DEMON_SHIELD")
}
