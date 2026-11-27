use super::{
    TestResult,
    item_forge_fixtures::support::{self, Case},
};
use oteryn_protocol_oteryn::item_forge::Operation;

#[test]
fn forge_chest_refuses_items_owned_by_timers() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "forge_chest_refuses_items_owned_by_timers",
        || support::scenario(Case::ChestScheduledInputs),
    )
}

#[test]
fn forge_fusion_delivers_one_chest_and_replay_preserves_contents() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "forge_fusion_delivers_one_chest_and_replay_preserves_contents",
        || support::scenario(Case::Chest(Operation::Fusion)),
    )
}

#[test]
fn forge_transfer_delivers_one_chest_and_replay_preserves_contents() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "forge_transfer_delivers_one_chest_and_replay_preserves_contents",
        || support::scenario(Case::Chest(Operation::Transfer)),
    )
}

#[test]
fn forge_full_inventory_refuses_chest_and_preserves_value() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "forge_full_inventory_refuses_chest_and_preserves_value",
        || support::scenario(Case::ChestFull),
    )
}

#[test]
fn forge_receipt_guards_reject_extra_mints_and_substituted_results() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "forge_receipt_guards_reject_extra_mints_and_substituted_results",
        || support::scenario(Case::ChestGuards(Operation::Fusion)),
    )
}

#[test]
fn forge_transfer_receipt_rejects_extra_mints_and_substituted_results() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "forge_transfer_receipt_rejects_extra_mints_and_substituted_results",
        || support::scenario(Case::ChestGuards(Operation::Transfer)),
    )
}
