//! M07 chosen Realm control: retained key, complete multi-floor census and two timers.
use super::m07_realm_trials_fixtures::{CONTROL};

use super::TestResult;

use super::m07_realm_trials_fixtures::NAME;

#[test]
fn m07_realm_postgres_retained_key_multifloor_clear_and_independent_resets()->TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none(){return Ok(());}
    let started=std::time::Instant::now();eprintln!("POSTGRES_CASE_STARTED {NAME}");
    let result=(||->TestResult {
        std::thread::Builder::new().stack_size(256<<20).spawn(||super::quest_world_callbacks_fixtures::scenario(NAME,CONTROL,false).map_err(|e|e.to_string()))?
            .join().map_err(|_|"Realm trial worker panicked")??;Ok(())
    })();
    eprintln!("POSTGRES_CASE_COMPLETED {NAME} duration={:?} success={}",started.elapsed(),result.is_ok());result
}
