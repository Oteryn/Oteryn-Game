//! IMBUE-WIRE-1 and PARITY-IMBUE-FORGE-WIRE-1: command 85 -> real composed dispatch -> durable
//! item owner read, and the shrine apply and clear writers.
use super::imbuement_fixtures::scenario;

use super::TestResult;


#[test]
fn imbuement_wire_dispatch_reads_the_carried_item_and_fails_closed_without_writes() -> TestResult {
    spawn(false, false)
}

#[test]
fn imbuement_wire_dispatch_applies_and_clears_at_a_shrine_in_one_fenced_transaction() -> TestResult
{
    let started = std::time::Instant::now();
    let result = spawn(true, false);
    eprintln!(
        "IMBUEMENT_BASIC POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn imbuement_elapsed_dispatch_pauses_expires_replays_and_reloads() -> TestResult {
    spawn(true, true)
}

fn spawn(shrine: bool, elapsed: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(shrine, elapsed, false, false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "imbuement wire e2e panic")??;
    Ok(())
}

#[test]
fn t174_shrine_dispatch_exposes_all_48_scroll_recipes() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(true, false, false, true).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "scroll catalogue panic")??;
    Ok(())
}
