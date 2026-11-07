//! The shared state operation proof uses PostgreSQL transaction time at its boundaries.
use super::TestResult;
use super::fixture::{Harness, run};

#[test]
fn questpc8_state_clock_postgres_exact_deadline_shapes_and_transaction_timestamp() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "questpc8_state_clock", false, false).await?;
        eprintln!("POSTGRES_CASE_STARTED questpc8_state_clock after PostgreSQL fixture initialization");
        sqlx::raw_sql(include_str!("questpc8_state_clock/boundaries.sql"))
            .execute(&h.pool).await?;
        Ok(())
    })?;
    eprintln!("POSTGRES_CASE_COMPLETED questpc8_state_clock duration={:?}", started.elapsed());
    Ok(())
}
