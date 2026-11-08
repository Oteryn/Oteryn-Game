//! Owned special-item physical custody, using the shared native drop/replay fixture.
use super::TestResult;
const TILE: oteryn_gameplay_quests::quest::world_callbacks::Position =
    oteryn_gameplay_quests::quest::world_callbacks::Position {
        x: 7,
        y: 1,
        floor: -7,
    };

#[test]
fn l6_special_item_8862_native_custody_and_replay() -> TestResult {
    run("l6-special-item-8862")
}

#[test]
fn l6_special_item_21389_native_custody_and_replay() -> TestResult {
    run("l6-special-item-21389")
}

#[test]
fn l6_special_item_5552_native_custody_and_replay() -> TestResult {
    run("l6-special-item-5552")
}

#[test]
fn l6_special_item_9171_native_custody_and_replay() -> TestResult {
    run("l6-special-item-9171")
}

#[test]
fn l6_special_item_21572_native_custody_and_replay() -> TestResult {
    run("l6-special-item-21572")
}

fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        assert!(
            std::env::var_os("OTERYN_REQUIRE_PG").is_none(),
            "Special-item custody requires PostgreSQL"
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
        "l6-special-item-8862" => Some(8862),
        "l6-special-item-21389" => Some(21389),
        "l6-special-item-5552" => Some(5552),
        "l6-special-item-9171" => Some(9171),
        "l6-special-item-21572" => Some(21572),
        _ => None,
    }
}
