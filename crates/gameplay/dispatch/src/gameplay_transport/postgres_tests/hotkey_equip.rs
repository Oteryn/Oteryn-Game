//! Real command routing, fenced custody, replay and durable equipment reload.
use super::hotkey_equip_fixtures::{ScenarioMode, scenario_options_impl};

use super::TestResult;

#[test]
fn hotkey_equip_postgres_dispatch_toggle_replay_and_durable_reload() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("hotkey-equip", || scenario(false))
}

#[test]
fn hotkey_equip_postgres_worn_item_toggles_with_spare_carried() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("hotkey-equip-spare", || scenario(true))
}

fn scenario(spare: bool) -> TestResult {
    scenario_options(spare, false, false, false, None, false)
}

#[test]
fn hotkey_equip_postgres_feared_refuses_without_mutation() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("hotkey-equip-feared", || {
        scenario_options(false, true, false, false, None, false)
    })
}

#[test]
fn hotkey_equip_postgres_nested_source_is_atomic_and_durable() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("hotkey-equip-nested", || {
        scenario_options(false, false, true, false, None, false)
    })
}

#[test]
fn hotkey_equip_postgres_first_started_copy_beats_unused_and_later_started() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("hotkey-equip-started", || {
        scenario_options(false, false, false, true, None, false)
    })
}

#[test]
fn hotkey_equip_postgres_nested_exchange_has_one_receipt_and_exact_edges() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("hotkey-equip-nested-swap", || {
        scenario_options(false, false, true, false, Some(false), false)
    })
}

#[test]
fn hotkey_equip_postgres_nested_exchange_full_main_refuses_atomically() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("hotkey-equip-nested-full", || {
        scenario_options(false, false, true, false, Some(true), false)
    })
}

#[test]
fn hotkey_equip_postgres_nested_search_with_equipped_extra_quiver() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("hotkey-equip-extra-quiver", || {
        scenario_options(false, false, true, false, None, true)
    })
}

fn scenario_options(
    spare: bool,
    feared: bool,
    nested: bool,
    started: bool,
    occupied: Option<bool>,
    extra_quiver: bool,
) -> TestResult {
    scenario_options_impl(
        spare,
        feared,
        nested,
        started,
        occupied,
        extra_quiver,
        ScenarioMode::Normal,
    )
}

#[test]
fn hotkey_equip_postgres_pouch_source_replay_and_foreign_refusal() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("hotkey-pouch", || {
        scenario_options_impl(false, false, true, false, None, false, ScenarioMode::Pouch)?;
        scenario_options_impl(
            false,
            false,
            true,
            false,
            None,
            false,
            ScenarioMode::ForeignPouch,
        )
    })
}

#[test]
fn hotkey_equip_postgres_managed_destination_replay() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("hotkey-managed", || {
        scenario_options_impl(
            false,
            false,
            true,
            false,
            None,
            false,
            ScenarioMode::Managed,
        )
    })
}
