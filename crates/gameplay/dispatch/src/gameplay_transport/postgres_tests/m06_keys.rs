//! M06: carried key custody through production USE wire, replay and reconnect.
use super::m06_keys_fixtures::{scenario};

use super::TestResult;

#[test]
fn m06_carried_keys_use_wire_lock_wrong_key_replay_and_relog() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(None).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "M06 wire panic")??;
    Ok(())
}
