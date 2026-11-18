//! Atomic seven-instance native repair: no partial budget, occupied wall or replay charge.
use super::m07_machinery_repairs_fixtures::{SOURCE};

use super::TestResult;

use super::m07_machinery_repairs_fixtures::NAME;

#[test]
fn m07_machinery_postgres_repair_consumes_exact_budget_and_keeps_walls_solid()->TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none(){return Ok(());}
    let begun=std::time::Instant::now();eprintln!("POSTGRES_CASE_STARTED {NAME}");
    let result=(||->TestResult {
        std::thread::Builder::new().stack_size(256<<20)
            .spawn(||super::quest_world_callbacks_fixtures::scenario(NAME,SOURCE,false).map_err(|e|e.to_string()))?
            .join().map_err(|_|"Machinery repair worker panicked")??;Ok(())
    })();
    eprintln!("POSTGRES_CASE_COMPLETED {NAME} duration={:?} success={}",begun.elapsed(),result.is_ok());result
}
