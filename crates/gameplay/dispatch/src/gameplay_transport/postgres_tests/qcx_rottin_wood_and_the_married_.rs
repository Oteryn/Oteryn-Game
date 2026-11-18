use super::TestResult;

#[test]
fn rottin_accepts_quest_and_requires_two_confirmations_for_disrepair() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::classic_quests::rottin_disrepair_scenario()?;
    eprintln!("QCX72_ROTTIN_PG_EXECUTED duration={:?}", started.elapsed());
    Ok(())
}
