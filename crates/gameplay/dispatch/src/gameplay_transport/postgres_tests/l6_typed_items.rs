//! Owned typed-item physical custody, using the shared native drop/replay fixture.
use super::TestResult;
const TILE: oteryn_gameplay_quests::quest::world_callbacks::Position =
    oteryn_gameplay_quests::quest::world_callbacks::Position {
        x: 7,
        y: 1,
        floor: -7,
    };

#[test]
fn l6_typed_item_5949_native_custody_and_replay() -> TestResult {
    run("l6-typed-item-5949")
}

#[test]
fn l6_typed_item_24382_native_custody_and_replay() -> TestResult {
    run("l6-typed-item-24382")
}

#[test]
fn l6_typed_item_21506_native_custody_and_replay() -> TestResult {
    run("l6-typed-item-21506")
}

#[test]
fn l6_typed_item_7785_native_custody_and_replay() -> TestResult {
    run("l6-typed-item-7785")
}

#[test]
fn l6_typed_item_52632_native_custody_and_replay() -> TestResult {
    run("l6-typed-item-52632")
}

fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        assert!(
            std::env::var_os("OTERYN_REQUIRE_PG").is_none(),
            "Typed-item custody requires PostgreSQL"
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
        "l6-typed-item-5949" => Some(5949),
        "l6-typed-item-24382" => Some(24382),
        "l6-typed-item-21506" => Some(21506),
        "l6-typed-item-7785" => Some(7785),
        "l6-typed-item-52632" => Some(52632),
        _ => None,
    }
}
