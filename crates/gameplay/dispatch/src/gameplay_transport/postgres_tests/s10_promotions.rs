//! Every newly populated source promotion giver reaches the real purchase and replay writer.
use super::TestResult;

#[test]
fn s10_promotions_postgres_registered_sources_charge_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            for npc in ["ishebad", "enpa_rudra", "emperor_rehal"] {
                let subcase = std::time::Instant::now();
                super::promotion_npc::run_source_promotion_scenario(npc)
                    .map_err(|error| error.to_string())?;
                eprintln!(
                    "POSTGRES_SUBCASE_COMPLETED S10 promotion npc={npc} duration={:?}",
                    subcase.elapsed()
                );
            }
            Ok::<_, String>(())
        })?
        .join()
        .map_err(|_| "S10 promotion PostgreSQL thread panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED S10 promotions_source duration={:?}",
        started.elapsed()
    );
    Ok(())
}
