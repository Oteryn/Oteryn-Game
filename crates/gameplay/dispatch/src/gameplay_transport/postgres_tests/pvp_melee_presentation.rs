use super::{TestResult, item_look_fixtures::large_stack, pvp_live_fixtures::melee_presentation};

#[test]
fn pvp_melee_publishes_committed_damage_to_capable_victim() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    large_stack(|| melee_presentation(true))
}

#[test]
fn pvp_melee_omits_damage_for_victim_without_presentation_capability() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    large_stack(|| melee_presentation(false))
}
