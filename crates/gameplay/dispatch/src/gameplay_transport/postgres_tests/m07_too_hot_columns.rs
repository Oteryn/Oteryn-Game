//! Native column repair trial: exact retained chisel, live level, personal credit.
use super::m07_too_hot_columns_fixtures::{SOURCE};

use super::TestResult;

#[test]
fn m07_too_hot_postgres_column_retains_chisel_and_credits_once()->TestResult {
    run("m07/too-hot-column-01")
}
#[test]
fn m07_too_hot_postgres_column_refuses_level79_without_credit()->TestResult {
    run("m07/too-hot-column-01-level79")
}
fn run(name:&'static str)->TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none(){return Ok(());}
    let begun=std::time::Instant::now();eprintln!("POSTGRES_CASE_STARTED {name}");
    let result=(||->TestResult {
        std::thread::Builder::new().stack_size(256<<20)
            .spawn(move ||super::quest_world_callbacks_fixtures::scenario(name,SOURCE,false).map_err(|e|e.to_string()))?
            .join().map_err(|_|"Too Hot column worker panicked")??;Ok(())
    })();
    eprintln!("POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",begun.elapsed(),result.is_ok());result
}
