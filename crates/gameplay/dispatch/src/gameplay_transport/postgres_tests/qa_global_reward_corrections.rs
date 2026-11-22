//! Exact reward identity of the first Power Ring Quest box through dispatch USE.
use super::TestResult;

// Crystal 201189f chest.lua:1755–1761 grants one gold coin at UID 6151.
#[test]
fn qa_power_ring_quest_first_box_grants_crystal_gold_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            super::qready1_reward_claims::claim_case(
                "oteryn:reward-claim.kv.powerring1",
                (32599, 31776, -9),
                &[],
                2473,
                "oteryn:item.tibia.i3031",
                1,
            )
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "power ring dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qa_power_ring_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
