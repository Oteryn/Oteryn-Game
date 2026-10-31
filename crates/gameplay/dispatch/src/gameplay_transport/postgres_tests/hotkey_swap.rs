//! I17c real dispatch, committed custody reload and replay for each durable transition.
use super::TestResult;
use super::hotkey_equip::{ScenarioMode, swap_scenario};

#[test]
fn hotkey_swap_postgres_two_hander_displaces_both_hands() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("hotkey-two-hands", || {
        swap_scenario(ScenarioMode::Hands {
            incoming: "oteryn:item.tibia.i50273",
            both: true,
            full: false,
        })
    })
}
#[test]
fn hotkey_swap_postgres_shield_displaces_two_hander() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("hotkey-shield", || {
        swap_scenario(ScenarioMode::Hands {
            incoming: "oteryn:item.tibia.i3412",
            both: false,
            full: false,
        })
    })
}
#[test]
fn hotkey_swap_postgres_one_hander_displaces_two_hander() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("hotkey-one-hand", || {
        swap_scenario(ScenarioMode::Hands {
            incoming: "oteryn:item.tibia.i3264",
            both: false,
            full: false,
        })
    })
}
#[test]
fn hotkey_swap_postgres_full_backpack_refuses_both_hands_atomically() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("hotkey-hands-full", || {
        swap_scenario(ScenarioMode::Hands {
            incoming: "oteryn:item.tibia.i50273",
            both: true,
            full: true,
        })
    })
}
#[test]
fn hotkey_swap_postgres_ammunition_goes_to_quiver() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("hotkey-ammo", || {
        swap_scenario(ScenarioMode::Ammo {
            full: false,
            absent: false,
        })
    })
}
#[test]
fn hotkey_swap_postgres_full_quiver_refuses_atomically() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("hotkey-ammo-full", || {
        swap_scenario(ScenarioMode::Ammo {
            full: true,
            absent: false,
        })
    })
}
#[test]
fn hotkey_swap_postgres_ammunition_requires_quiver() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("hotkey-ammo-absent", || {
        swap_scenario(ScenarioMode::Ammo {
            full: false,
            absent: true,
        })
    })
}
