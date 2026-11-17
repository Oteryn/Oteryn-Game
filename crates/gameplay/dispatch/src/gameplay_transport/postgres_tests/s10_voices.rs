//! S10 source-population regression through the production voice cadence and CHAT owner.
use super::TestResult;

#[test]
fn s10_valindara_postgres_source_voice_reaches_chat() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::nx4_npc_presence::source_voice_case("oteryn:npc.valindara", "Valindara", "")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED S10 valindara_voice duration={:?}",
        started.elapsed()
    );
    Ok(())
}
