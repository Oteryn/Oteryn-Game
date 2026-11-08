//! Beregar NPC-item physical custody, using the shared native drop/replay fixture.
use super::TestResult;
const TILE: oteryn_gameplay_quests::quest::world_callbacks::Position =
    oteryn_gameplay_quests::quest::world_callbacks::Position {
        x: 7,
        y: 1,
        floor: -7,
    };

#[test]
fn l6_beregar_item_8774_native_custody_and_replay() -> TestResult {
    run("l6-beregar-item-8774")
}

#[test]
fn l6_beregar_item_8777_native_custody_and_replay() -> TestResult {
    run("l6-beregar-item-8777")
}

fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        assert!(
            std::env::var_os("OTERYN_REQUIRE_PG").is_none(),
            "Beregar custody requires PostgreSQL"
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
        "l6-beregar-item-8774" => Some(8774),
        "l6-beregar-item-8777" => Some(8777),
        _ => None,
    }
}
