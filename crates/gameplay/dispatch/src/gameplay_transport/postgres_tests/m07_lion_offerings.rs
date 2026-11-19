//! Native Lion offerings and renewable source pickers: an exact carried offering, source state and15s world lifecycle.
use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

const NAME:&str="m07/lion-strength";
const SOURCE:Position=Position{x:33137,y:32291,floor:-8};

#[test]
fn m07_lion_strength_postgres_exact_consumption_counter_and_pagoda_reset()->TestResult {
    run(NAME,SOURCE)
}
#[test]
fn m07_lion_tears_postgres_exact_consumption_counter_and_pagoda_reset()->TestResult {
    run("m07/lion-tears",Position{x:33154,y:32279,floor:-8})
}
#[test]
fn m07_lion_well_postgres_repeatable_exact_reward_and_command_replay()->TestResult {
    run("m07/lion-well",Position{x:33137,y:32351,floor:-6})
}
#[test]
fn m07_lion_beauty_postgres_exact_consumption_counter_and_pagoda_reset()->TestResult {
    run("m07/lion-beauty",Position{x:33138,y:32369,floor:-8})
}
#[test]
fn m07_lion_flower_west_postgres_bloom_reset_and_repeatable_pickup()->TestResult {
    run("m07/lion-flower-west",Position{x:33133,y:32359,floor:-6})
}
#[test]
fn m07_lion_flower_east_postgres_bloom_reset_and_repeatable_pickup()->TestResult {
    run("m07/lion-flower-east",Position{x:33157,y:32364,floor:-6})
}
fn run(name:&'static str,source:Position)->TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none(){return Ok(());}
    let started=std::time::Instant::now();eprintln!("POSTGRES_CASE_STARTED {name}");
    let result=(||->TestResult {
        std::thread::Builder::new().stack_size(256<<20).spawn(move ||super::quest_world_callbacks_fixtures::scenario(name,source,false).map_err(|e|e.to_string()))?
            .join().map_err(|_|"M07 Lion worker panicked")??;Ok(())
    })();
    eprintln!("POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",started.elapsed(),result.is_ok());result
}
