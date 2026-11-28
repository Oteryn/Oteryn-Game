//! W18 carpet source-tile checks through the guest's actual House map USE.
use super::TestResult;
use super::house_balloon_fixtures::{CarpetResult, Case, run_case};

#[test]
fn w18_guest_carpet_map_use_adjacent_items_and_receipt_replay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    for (other, same_tile, result) in [
        (23431, true, CarpetResult::Refused),
        (23431, false, CarpetResult::Folded),
        (5302, true, CarpetResult::Removed),
        (5302, false, CarpetResult::Folded),
    ] {
        run_case(Case::CarpetWire { other, same_tile, result })?;
    }
    Ok(())
}
