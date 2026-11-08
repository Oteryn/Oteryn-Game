//! Physical custody for source Yalahar NPC items; no custom use activation.
use super::TestResult;
const TILE: oteryn_gameplay_quests::quest::world_callbacks::Position =
    oteryn_gameplay_quests::quest::world_callbacks::Position {
        x: 7,
        y: 1,
        floor: -7,
    };

#[test]
fn l6_yalahar_item_8758_native_custody_and_replay() -> TestResult {
    run("l6-yalahar-item-8758")
}

#[test]
fn l6_yalahar_item_8759_native_custody_and_replay() -> TestResult {
    run("l6-yalahar-item-8759")
}

#[test]
fn l6_yalahar_item_8760_native_custody_and_replay() -> TestResult {
    run("l6-yalahar-item-8760")
}

#[test]
fn l6_yalahar_item_8761_native_custody_and_replay() -> TestResult {
    run("l6-yalahar-item-8761")
}

#[test]
fn l6_yalahar_item_8762_native_custody_and_replay() -> TestResult {
    run("l6-yalahar-item-8762")
}

#[test]
fn l6_yalahar_item_8763_native_custody_and_replay() -> TestResult {
    run("l6-yalahar-item-8763")
}

#[test]
fn l6_yalahar_item_8818_native_custody_and_replay() -> TestResult {
    run("l6-yalahar-item-8818")
}

#[test]
fn l6_yalahar_item_8819_native_custody_and_replay() -> TestResult {
    run("l6-yalahar-item-8819")
}

fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        assert!(
            std::env::var_os("OTERYN_REQUIRE_PG").is_none(),
            "Yalahar custody requires PostgreSQL"
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
        "l6-yalahar-item-8758" => Some(8758),
        "l6-yalahar-item-8759" => Some(8759),
        "l6-yalahar-item-8760" => Some(8760),
        "l6-yalahar-item-8761" => Some(8761),
        "l6-yalahar-item-8762" => Some(8762),
        "l6-yalahar-item-8763" => Some(8763),
        "l6-yalahar-item-8818" => Some(8818),
        "l6-yalahar-item-8819" => Some(8819),
        _ => None,
    }
}
