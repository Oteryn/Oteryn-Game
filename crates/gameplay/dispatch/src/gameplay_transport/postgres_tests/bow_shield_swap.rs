//! Real hotkey dispatch must stow the shield when equipping a bow from the backpack.
use super::TestResult;
use super::hotkey_equip_fixtures::{ScenarioMode, swap_scenario};

#[test]
fn hotkey_swap_postgres_bow_displaces_wooden_shield() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("hotkey-bow-shield", || {
        // Hands { both: true } starts with a one-handed weapon and wooden shield i3412.
        // It checks the worn bow, both displaced items in the backpack, and durable replay.
        swap_scenario(ScenarioMode::Hands {
            incoming: "oteryn:item.tibia.i3350",
            both: true,
            full: false,
        })
    })
}
