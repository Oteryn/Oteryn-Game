//! Source fishing branches through framed USE, terrain and the durable item writer.
#[path = "support/fishing_wire.rs"]
mod fishing_wire;

use super::TestResult;
use fishing_wire::{GatheringCase, gathering_scenario};

fn run(case: GatheringCase) -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || gathering_scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "source fishing scenario panicked")??;
    Ok(())
}

#[test]
fn fishing_sources_mechanical_shallows_spend_nail_and_replay_the_catch() -> TestResult {
    run(GatheringCase {
        tool: 9306,
        target_id: 629,
        after: Some(4809),
        yield_item: Some(9307),
        committed: true,
        in_sewers: true,
        select_roll: Some(|catch, _| catch <= 10_000),
        consumes_bait: true,
    })
}

#[test]
fn fishing_sources_desert_water_yields_sandfish_on_the_four_percent_tier() -> TestResult {
    run(GatheringCase {
        tool: 3483,
        target_id: 13988,
        after: Some(13989),
        yield_item: Some(13992),
        committed: true,
        in_sewers: false,
        select_roll: Some(|catch, loot| catch <= 10_000 && loot > 96_000),
        consumes_bait: true,
    })
}

#[test]
fn fishing_sources_dirty_water_can_spend_a_worm_and_yield_nothing() -> TestResult {
    run(GatheringCase {
        tool: 9306,
        target_id: 12562,
        after: Some(12559),
        yield_item: None,
        committed: true,
        in_sewers: false,
        select_roll: Some(|catch, loot| catch <= 10_000 && loot > 30_000),
        consumes_bait: true,
    })
}

#[test]
fn fishing_sources_spent_desert_water_cannot_yield_fish() -> TestResult {
    run(GatheringCase {
        tool: 3483,
        target_id: 13989,
        after: None,
        yield_item: None,
        committed: false,
        in_sewers: false,
        select_roll: Some(|catch, _| catch <= 10_000),
        consumes_bait: false,
    })
}

#[test]
fn fishing_sources_owned_elemental_remains_are_spent_once_without_bait() -> TestResult {
    run(GatheringCase {
        tool: 3483,
        target_id: 9582,
        after: Some(9583),
        yield_item: Some(3578),
        committed: true,
        in_sewers: false,
        select_roll: None,
        consumes_bait: false,
    })
}
