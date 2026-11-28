use super::TestResult;
use super::item_cosmetics_fixtures::{positive::Case, run_world_mount};

#[test]
fn s06_gloothomotive_requires_the_selected_loco_and_spends_both_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_world_mount(Case::Gloothomotive)
}

#[test]
fn s06_alpha_demonosaur_box_spends_one_and_unlocks_the_look_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_world_mount(Case::AlphaBox)
}
