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
fn rune_3165_dispatch_paralyzes_player_and_replaces_haste() -> TestResult {
    super::item_look_fixtures::large_stack(|| run(async |admin| run_case(admin, Case::ParalyzePlayer).await))
}
