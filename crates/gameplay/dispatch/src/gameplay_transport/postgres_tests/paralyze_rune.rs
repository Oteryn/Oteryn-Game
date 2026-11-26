//! Rune 3165 through the admitted USE dispatcher, native payment and condition owners.
use super::fixture::run;
use super::rune_use_bridge_fixtures::{Case, run_case};
use super::TestResult;

#[test]
fn rune_3165_dispatch_paralyzes_monster_and_replaces_haste() -> TestResult {
    super::item_look_fixtures::large_stack(|| {
        run(async |admin| run_case(admin, Case::ParalyzeMonster).await)
    })
}

#[test]
fn rune_3165_dispatch_paralyzes_equipped_player_and_replaces_haste() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    super::item_look_fixtures::large_stack(|| run(async |admin| run_case(admin, Case::ParalyzePlayer { open_pvp: true }).await))
}

#[test]
fn rune_3165_dispatch_refuses_equipped_player_on_optional_pvp_without_war() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    super::item_look_fixtures::large_stack(|| {
        run(async |admin| run_case(admin, Case::ParalyzePlayer { open_pvp: false }).await)
    })
}
