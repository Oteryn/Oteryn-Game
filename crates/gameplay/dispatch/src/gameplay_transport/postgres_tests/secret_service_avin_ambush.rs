//! Crystal 201189f8, data-global/npc/gamel.lua:59-74.

use super::TestResult;

#[test]
fn avin_greeting_requires_the_letter_and_spawns_both_bandits_atomically() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        super::secret_service_avin_ambush_fixtures::NAME,
        || {
            super::quest_world_callbacks_fixtures::scenario(
                super::secret_service_avin_ambush_fixtures::NAME,
                super::secret_service_avin_ambush_fixtures::AT,
                false,
            )
        },
    )
}
