use crate::bestiary_postgres_harness::{Harness, TestResult, debug, runtime};

#[test]
fn p04_thirty_two_contended_qualification_spawns_and_kills_credit_bounty() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            runtime()
                .map_err(debug)?
                .block_on(async {
                    let harness = Harness::create(admin, "p04_spawn_bounty", true).await?;
                    let result = crate::node::serve::exercise_busy_spawn(&harness, 32).await;
                    harness.cleanup().await?;
                    result
                })
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "P04 qualification spawn thread failed")?
        .map_err(Into::into)
}
