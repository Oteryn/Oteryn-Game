//! Crystal 201189f8, chayenne_realm/actions_lever.lua:3-22 supplies the
//! retained key and timed wall control. The added clearance floor uses a
//! Dragon from Crystal world-monster.xml:47455-47457.
use super::TestResult;
use super::five_floor_clearance_fixtures::NAME;
use super::m07_realm_trials_fixtures::CONTROL;
use super::quest_world_callbacks_fixtures as quest_world_callbacks;

#[test]
fn realm_control_waits_for_guardian_on_fifth_floor() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| quest_world_callbacks::scenario(NAME, CONTROL, false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "five-floor clearance worker panicked")??;
    Ok(())
}
