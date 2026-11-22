use super::TestResult;
use super::fishing_wire::{GatheringCase, RiverProjection, gathering_scenario};
use oteryn_gameplay_items::fishing::{DESERT_FISHING_SPENT, DESERT_FISHING_WATER, SANDFISH};

#[test]
fn daily_variant_river_fishing_spends_bait_and_replays_the_single_sandfish() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let started = std::time::Instant::now();
    river_cast(GatheringCase {
        tool: 3483,
        target_id: DESERT_FISHING_WATER,
        after: Some(DESERT_FISHING_SPENT),
        yield_item: Some(SANDFISH),
        committed: true,
        in_sewers: false,
        projected_desert: Some(RiverProjection::Terrain),
        select_roll: None,
        consumes_bait: true,
    })?;
    eprintln!(
        "DAILY_VARIANT_POSTGRES_CASE_EXECUTED river_fishing elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

#[test]
fn daily_variant_river_fishing_spends_bait_without_a_sandfish_and_recovers() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    river_cast(GatheringCase {
        tool: 3483,
        target_id: DESERT_FISHING_WATER,
        after: Some(DESERT_FISHING_SPENT),
        yield_item: None,
        committed: true,
        in_sewers: false,
        projected_desert: Some(RiverProjection::Item),
        select_roll: None,
        consumes_bait: true,
    })?;
    Ok(())
}

#[test]
fn daily_variant_river_fishing_refuses_a_mechanical_cast_after_reset() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    river_cast(GatheringCase {
        tool: 9306,
        target_id: DESERT_FISHING_WATER,
        after: None,
        yield_item: None,
        committed: false,
        in_sewers: false,
        projected_desert: Some(RiverProjection::RetiredItem),
        select_roll: Some(|catch, loot| catch <= 10_000 && loot > 1_000),
        consumes_bait: false,
    })?;
    Ok(())
}

#[test]
fn daily_variant_river_fishing_refuses_a_cast_on_spent_projected_water() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    river_cast(GatheringCase {
        tool: 3483,
        target_id: DESERT_FISHING_SPENT,
        after: None,
        yield_item: None,
        committed: false,
        in_sewers: false,
        projected_desert: Some(RiverProjection::Item),
        select_roll: Some(|catch, loot| catch <= 10_000 && loot > 96_000),
        consumes_bait: false,
    })?;
    Ok(())
}

fn river_cast(case: GatheringCase) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || gathering_scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "desert fishing dispatch thread failed")??;
    Ok(())
}
