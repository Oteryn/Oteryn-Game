//! Current fenced shaping observations through the production owner and real PostgreSQL.
use super::TestResult;

#[test]
fn weapon_proficiency_state_real_wire_track_revision_slots_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| super::proficiency_modification_fixtures::state_scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "proficiency state scenario panicked")?
        .map_err(Into::into)
}
