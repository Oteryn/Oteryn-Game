//! Physical input custody for the owned, selected Treasure Hunt route.
use super::TestResult;

const TILE: oteryn_gameplay_quests::quest::world_callbacks::Position =
    oteryn_gameplay_quests::quest::world_callbacks::Position {
        x: 7,
        y: 1,
        floor: -7,
    };

#[test]
fn l6_treasure_item_9181_native_custody_and_replay() -> TestResult {
    run("l6-treasure-item-9181")
}

#[test]
fn l6_treasure_item_9182_native_custody_and_replay() -> TestResult {
    run("l6-treasure-item-9182")
}

#[test]
fn l6_treasure_item_9183_native_custody_and_replay() -> TestResult {
    run("l6-treasure-item-9183")
}

#[test]
fn l6_treasure_item_9184_native_custody_and_replay() -> TestResult {
    run("l6-treasure-item-9184")
}

#[test]
fn l6_treasure_item_9186_native_custody_and_replay() -> TestResult {
    run("l6-treasure-item-9186")
}

#[test]
fn l6_treasure_item_9177_native_custody_and_replay() -> TestResult {
    run("l6-treasure-item-9177")
}

fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        assert!(
            std::env::var_os("OTERYN_REQUIRE_PG").is_none(),
            "Treasure Hunt item custody requires PostgreSQL"
        );
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(name, TILE, false)
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) fn source_id(name: &str) -> Option<u32> {
    match name {
        "l6-treasure-item-9181" => Some(9181),
        "l6-treasure-item-9182" => Some(9182),
        "l6-treasure-item-9183" => Some(9183),
        "l6-treasure-item-9184" => Some(9184),
        "l6-treasure-item-9186" => Some(9186),
        "l6-treasure-item-9177" => Some(9177),
        _ => None,
    }
}
