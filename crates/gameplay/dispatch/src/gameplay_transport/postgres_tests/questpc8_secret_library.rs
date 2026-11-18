//! Selected library journey: real NPC discussions, owned component conversion and captured Oberon death.
use super::questpc8_secret_library_fixtures::{run_library_journey};

use super::TestResult;

#[test]
fn questpc8_secret_library_raft_research_fiddle_gail_oberon_and_access() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    run_library_journey()?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_secret_library_raft_research_fiddle_gail_oberon_and_access duration={:?}",
        started.elapsed()
    );
    Ok(())
}
