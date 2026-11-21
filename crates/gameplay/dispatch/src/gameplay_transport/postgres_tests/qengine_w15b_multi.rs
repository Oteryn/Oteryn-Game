//! Multiple top-level chest rewards must retain atomic claim, quest and replay semantics.
use super::TestResult;
#[test]
fn w15b_multiple_top_level_items_real_dispatch_once_and_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::quest_chest_postgres_tests_fixtures::run_multiple();
    eprintln!(
        "POSTGRES_CASE_COMPLETED W15B_MULTIPLE_ROOTS duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn w15b_multiple_top_level_items_one_free_slot_refuses_atomically() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::quest_chest_postgres_tests_fixtures::run_one_free_slot();
    eprintln!(
        "POSTGRES_CASE_COMPLETED W15B_NO_ROOM duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
