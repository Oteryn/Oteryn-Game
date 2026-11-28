use super::TestResult;
use super::hotkey_equip_fixtures::{ScenarioMode, swap_scenario};
use super::kill_reward_e2e_fixtures::scenario_thread::run;
use oteryn_protocol_oteryn::item_view::ItemMoveOutcome;

#[test]
fn explicit_zaoan_helmet_move_accepts_knight_postgres() -> TestResult {
    run("explicit-helmet-knight", || {
        swap_scenario(ScenarioMode::ExplicitHead {
            vocation: "knight",
            expected: ItemMoveOutcome::Moved,
        })
    })
}

#[test]
fn explicit_zaoan_helmet_move_refuses_wrong_vocation_postgres() -> TestResult {
    run("explicit-helmet-druid", || {
        swap_scenario(ScenarioMode::ExplicitHead {
            vocation: "druid",
            expected: ItemMoveOutcome::RequirementNotMet,
        })
    })
}
