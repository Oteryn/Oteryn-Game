//! A live PvP ATTACK uses a worn charge and retires its item in the swing commit.
use super::TestResult;

#[test]
fn equipped_charges_follow_the_pvp_swing_commit_and_replay() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::item_look_fixtures::large_stack(|| super::pvp_live_fixtures::scenario(17))
}

#[test]
fn paid_charged_pvp_splash_retries_busy_target_and_hit_holders() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::item_look_fixtures::large_stack(|| super::pvp_live_fixtures::scenario(18))
}

#[test]
fn pvp_late_commit_preserves_the_held_target_and_next_swing() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::item_look_fixtures::large_stack(|| super::pvp_live_fixtures::scenario(19))
}
