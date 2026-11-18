#[cfg(feature = "test-support")]
#[test]
fn qualification_isolated_grant_requires_mode_and_reads_exact_source_build()
-> crate::bestiary_postgres_harness::TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| crate::node::serve::verify_isolated_row_grant().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "isolated qualification PostgreSQL test panicked")?
        .map_err(Into::into)
}
