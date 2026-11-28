//! W18 carpet source-tile checks through the guest's actual House map USE.
use super::TestResult;
use super::house_balloon_fixtures::{CarpetNeighbor as Neighbor, CarpetResult as Result, Case, run_case};

#[test]
fn w18_guest_carpet_map_use_adjacent_items_and_receipt_replay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    for (other, neighbor, result) in [
        (23431, Neighbor::SameTile, Result::Refused),
        (22736, Neighbor::SameTile, Result::Folded),
        (23431, Neighbor::Adjacent, Result::Folded),
        (5302, Neighbor::SameTile, Result::Removed),
        (5302, Neighbor::Adjacent, Result::Folded),
        (5302, Neighbor::Diagonal, Result::Folded),
        (5302, Neighbor::TiedWithBase, Result::Folded),
    ] {
        run_case(Case::CarpetWire { other, neighbor, result })?;
    }
    Ok(())
}
