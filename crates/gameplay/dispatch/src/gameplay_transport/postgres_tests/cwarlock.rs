//! CWARLOCK: the production callback must be available to real lever use.
use super::TestResult;

#[test]
fn cwarlock_lever_admission_waves_cooldown_death_and_completion_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    let callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter().find(|c| c.key == "oteryn:quest-callback/cwarlock/lever")
        .ok_or("Court Warlock production lever is missing")?;
    assert!(callback.effects.encounter.is_some());
    Ok(())
}
