//! Persisted occurrence drives the existing source map and native spawn owners.
#[test]
fn qworld_postgres_fury_grimvale_voice_physical_restart()
-> crate::bestiary_postgres_harness::TestResult {
    let started = std::time::Instant::now();
    crate::node::serve::daily_world_changes::tests::qworld_physical_case()?;
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_some() {
        eprintln!(
            "QWORLD_PHYSICAL_PG_EXECUTED elapsed_ms={}",
            started.elapsed().as_millis()
        );
    }
    Ok(())
}
