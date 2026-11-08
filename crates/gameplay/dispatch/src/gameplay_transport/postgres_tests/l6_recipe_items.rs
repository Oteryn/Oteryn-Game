//! Owned quest recipe NPC-item physical custody, using the shared native drop/replay fixture.
use super::TestResult;
const TILE: oteryn_gameplay_quests::quest::world_callbacks::Position =
    oteryn_gameplay_quests::quest::world_callbacks::Position {
        x: 7,
        y: 1,
        floor: -7,
    };

#[test]
fn l6_recipe_item_14682_native_custody_and_replay() -> TestResult {
    run("l6-recipe-item-14682")
}

#[test]
fn l6_recipe_item_5467_native_custody_and_replay() -> TestResult {
    run("l6-recipe-item-5467")
}

#[test]
fn l6_recipe_item_21504_native_custody_and_replay() -> TestResult {
    run("l6-recipe-item-21504")
}

#[test]
fn l6_recipe_item_21507_native_custody_and_replay() -> TestResult {
    run("l6-recipe-item-21507")
}

#[test]
fn l6_recipe_item_12674_native_custody_and_replay() -> TestResult {
    run("l6-recipe-item-12674")
}

#[test]
fn l6_recipe_item_3454_native_custody_and_replay() -> TestResult {
    run("l6-recipe-item-3454")
}

#[test]
fn l6_recipe_item_21466_native_custody_and_replay() -> TestResult {
    run("l6-recipe-item-21466")
}

#[test]
fn l6_recipe_item_21554_native_custody_and_replay() -> TestResult {
    run("l6-recipe-item-21554")
}

#[test]
fn l6_recipe_item_9106_native_custody_and_replay() -> TestResult {
    run("l6-recipe-item-9106")
}

#[test]
fn l6_recipe_item_36543_native_custody_and_replay() -> TestResult {
    run("l6-recipe-item-36543")
}

#[test]
fn l6_recipe_item_36544_native_custody_and_replay() -> TestResult {
    run("l6-recipe-item-36544")
}

#[test]
fn l6_recipe_item_9185_native_custody_and_replay() -> TestResult {
    run("l6-recipe-item-9185")
}

#[test]
fn l6_recipe_item_9205_native_custody_and_replay() -> TestResult {
    run("l6-recipe-item-9205")
}

fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        assert!(
            std::env::var_os("OTERYN_REQUIRE_PG").is_none(),
            "Owned quest recipe custody requires PostgreSQL"
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
        "l6-recipe-item-21466" => Some(21466),
        "l6-recipe-item-21554" => Some(21554),
        "l6-recipe-item-9106" => Some(9106),
        "l6-recipe-item-36543" => Some(36543),
        "l6-recipe-item-36544" => Some(36544),
        "l6-recipe-item-9185" => Some(9185),
        "l6-recipe-item-9205" => Some(9205),

        "l6-recipe-item-14682" => Some(14682),
        "l6-recipe-item-5467" => Some(5467),
        "l6-recipe-item-21504" => Some(21504),
        "l6-recipe-item-21507" => Some(21507),
        "l6-recipe-item-12674" => Some(12674),
        "l6-recipe-item-3454" => Some(3454),
        _ => None,
    }
}
