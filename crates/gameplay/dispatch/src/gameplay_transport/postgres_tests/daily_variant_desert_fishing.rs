use super::TestResult;
use oteryn_gameplay_items::fishing::{DESERT_FISHING_SPENT, DESERT_FISHING_WATER, SANDFISH};

#[test]
fn daily_variant_river_fishing_spends_bait_and_replays_the_single_sandfish() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            super::standard_fishing::gathering_scenario(
                3483,
                DESERT_FISHING_WATER,
                Some(DESERT_FISHING_SPENT),
                Some(SANDFISH),
                true,
                false,
            )
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "desert fishing dispatch thread failed")??;
    eprintln!("DAILY_VARIANT_POSTGRES_CASE_EXECUTED river_fishing elapsed_ms={}", started.elapsed().as_millis());
    Ok(())
}
