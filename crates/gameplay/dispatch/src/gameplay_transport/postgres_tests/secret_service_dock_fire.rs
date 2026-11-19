//! Crystal 201189f8, data-global/scripts/quests/others/actions_fire_bug.lua:144-155.
//! The Fandom archive Secret Service Quest/Spoiler confirms the shared lockout.

use super::TestResult;

#[test]
fn dock_fire_blocks_eligible_use_until_its_exact_item_retires() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::qengine_rewards_fixtures::scenario(super::secret_service_dock_fire_fixtures::NAME)
}
