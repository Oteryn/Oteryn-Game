//! Lane qc: Twenty Miles Beneath the Sea. The NPC chain runs through native wire
//! conversations; every world step is its own scenario named after its callback key.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use super::qc_twenty_miles_fixtures::{START, matches_chain};

#[test]
fn qc_twenty_miles_npc_chain_from_brandon_to_the_time_compass() -> TestResult {
    run("qc-twenty-miles")
}

#[test]
fn qc_twenty_miles_pile_lays_one_skull_once() -> TestResult {
    run("qc-twenty-miles-pile-1")
}

#[test]
fn qc_twenty_miles_altar_takes_a_skull_and_counts_it() -> TestResult {
    run("qc-twenty-miles-altar-1")
}

#[test]
fn qc_twenty_miles_hoard_takes_cursed_gold_and_opens_the_way() -> TestResult {
    run("qc-twenty-miles-hoard-1")
}

#[test]
fn qc_twenty_miles_ring_chest_grants_the_wedding_ring_once() -> TestResult {
    run("qc-twenty-miles-ring-chest")
}

#[test]
fn qc_twenty_miles_shipwreck_chest_grants_the_first_slab_once() -> TestResult {
    run("qc-twenty-miles-shipwreck-chest")
}

#[test]
fn qc_twenty_miles_shrine_chest_grants_the_second_slab_once() -> TestResult {
    run("qc-twenty-miles-shrine-chest")
}

#[test]
fn qc_twenty_miles_broken_altar_takes_its_slab() -> TestResult {
    run("qc-twenty-miles-broken-altar-1")
}

#[test]
fn qc_twenty_miles_sundial_trades_the_sea_ear_for_the_compass() -> TestResult {
    run("qc-twenty-miles-sundial")
}

fn run(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let trigger = if matches_chain(name) {
        START
    } else if name == super::qc_pinned_world::PILE {
        // Crystal 201189f8, data-global/world/world.otbm, tile (33456, 31648, 15).
        Position {
            x: 33456,
            y: 31648,
            floor: -15,
        }
    } else {
        oteryn_gameplay_quests::quest::world_callbacks::load()?
            .into_iter()
            .find(|c| c.key == format!("oteryn:quest-callback/{name}"))
            .and_then(|c| c.selector.position)
            .ok_or("qc twenty miles callback")?
    };
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        super::quest_world_callbacks_fixtures::scenario(name, trigger, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
