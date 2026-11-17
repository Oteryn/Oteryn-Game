//! QA lane: Tibia Global reward corrections of donor chests, proved through real dispatch USE.
use super::TestResult;

// Power Ring Quest rev 683946: the first box holds the Power Ring (Crystal
// chest.lua:1755-1761 encodes a gold coin under a "power ring" comment).
#[test]
fn qa_power_ring_quest_first_box_grants_power_ring_once() -> TestResult {
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
                &[(32599, 6151)],
                2473,
                "oteryn:item.tibia.i3050",
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
