//! ENGENC: private settlement is independent of physical corpse capacity.
use super::TestResult;

#[test]
fn engenc_private_loot_postgres_large_corpse_settles_private_loot_once() -> TestResult {
    let started = std::time::Instant::now();
    super::boss_rewards::run_case(super::boss_rewards::Case::LargePrivateCorpse)?;
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_some() {
        eprintln!("ENGENC_PRIVATE_LOOT_PG_EXECUTED elapsed_ms={}", started.elapsed().as_millis());
    }
    Ok(())
}
