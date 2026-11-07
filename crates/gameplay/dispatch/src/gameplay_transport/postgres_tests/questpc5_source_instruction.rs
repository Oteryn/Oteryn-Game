//! Source-only written rewards must use native custody and the existing quest obligation owner.
use super::TestResult;

const CLAIM: &str = "oteryn:reward-claim.source.formorgar_hoist_skeleton";
const TRACK: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_0/the_ice_islands/formorgar_mines_hoist_skeleton";

#[test]
fn questpc5_skeleton_instruction_source_use_and_cold_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let started = std::time::Instant::now();
        let h = super::fixture::Harness::create_with_admission_seed(
            admin,
            "questpc5_skeleton_source",
            false,
            false,
        )
        .await?;
        eprintln!(
            "POSTGRES_CASE_STARTED skeleton_instruction_source {:?}",
            started.elapsed()
        );
        let mut found = false;
        for raw in crate::content::world_reward_claims::REWARD_CLAIM_SHARDS {
            let shard: serde_json::Value = serde_json::from_str(raw)?;
            found |= shard["records"]
                .as_array()
                .ok_or("records")?
                .iter()
                .any(|r| r["definition"]["identity"]["key"] == CLAIM);
        }
        assert!(
            found,
            "pinned skeleton source instruction has no native reward definition"
        );
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_CASE_COMPLETED skeleton_instruction_source {:?}",
            started.elapsed()
        );
        Ok(())
    })?;
    super::quest_catalogue_chests::claim_progress_cohort(CLAIM, TRACK)
}
