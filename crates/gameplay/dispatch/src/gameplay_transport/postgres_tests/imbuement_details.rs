//! The optional quote follows the actual fenced item reader and never mutates inventory.
use super::TestResult;

#[test]
fn imbuement_details_postgres_reads_canonical_materials_and_fails_closed_on_item_session_fences()
-> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| super::imbuement_fixtures::details_scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "imbuement details test panicked")??;
    Ok(())
}
