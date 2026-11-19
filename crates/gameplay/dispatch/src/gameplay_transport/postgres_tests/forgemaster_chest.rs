//! Crystal 201189f8, the_forgemaster.lua:176-180 and
//! no_rest_for_the_wicked_rewards.lua:1-32. The positive fixture uses an admitted
//! item to prove the shared claim owner while the donor item awaits admission.
use super::TestResult;
use super::forgemaster_chest_fixtures::{CHEST, LOCKED, NATIVE, OPEN, UNADMITTED};
use super::quest_world_callbacks_fixtures::scenario;

#[test]
fn forgemaster_chest_requires_door_access_and_claims_the_retained_reward_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    for case in [LOCKED, OPEN, UNADMITTED] {
        super::kill_reward_e2e_fixtures::scenario_thread::run(case, move || {
            scenario(case, CHEST, false)
        })?;
    }
    Ok(())
}

#[test]
fn forgemaster_chest_use_matches_the_untagged_native_placement() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(NATIVE, || scenario(NATIVE, CHEST, false))
}
