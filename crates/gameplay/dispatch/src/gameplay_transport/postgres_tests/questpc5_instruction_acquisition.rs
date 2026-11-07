//! Instruction custody and its durable access flag must compose through real USE.
use super::TestResult;

#[test]
fn questpc5_hoist_construction_plan_claim_writes_instruction_and_replays() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::quest_catalogue_chests::claim_progress_cohort(
        "oteryn:reward-claim.quest.u8_0.the_ice_islands.formorgar_glacier_chest",
        "oteryn:quest-progress/crystalserver/quest/u8_0/the_ice_islands/formorgar_mines_hoist_chest",
    )?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc5_hoist_construction_plan_claim_writes_instruction_and_replays {:?}",
        started.elapsed()
    );
    Ok(())
}
