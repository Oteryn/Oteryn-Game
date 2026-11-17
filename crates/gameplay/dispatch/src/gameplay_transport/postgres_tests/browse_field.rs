//! I18: command 440 opens a virtual view; command 9 alone changes durable custody.

#[path = "browse_field/fixture.rs"]
mod fixture;
#[path = "browse_field/wire.rs"]
mod wire;

#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use super::TestResult;

#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
const BAG: &str = "oteryn:item.tibia.i2854";
#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
const COIN: &str = "oteryn:item.tibia.i3031";
#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
const FIELD: oteryn_protocol_oteryn::item_view::WorldTilePosition =
    oteryn_protocol_oteryn::item_view::WorldTilePosition {
        x: 7,
        y: 0,
        floor: -7,
    };

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
enum Case {
    Backpack,
    Nested,
    Navigation,
    Paging,
    Autowalk,
}

#[cfg(feature = "pg-tests-g1")]
#[test]
fn browse_field_postgres_drop_pickup_backpack_and_stale_handle_replay() -> TestResult {
    run_case(Case::Backpack)
}

#[cfg(feature = "pg-tests-g1")]
#[test]
fn browse_field_postgres_pickup_carried_container_and_atomic_refusal() -> TestResult {
    run_case(Case::Nested)
}

#[cfg(feature = "pg-tests-g1")]
#[test]
fn browse_field_postgres_ground_container_up_and_movement_close() -> TestResult {
    run_case(Case::Navigation)
}

#[cfg(feature = "pg-tests-g1")]
#[test]
fn browse_field_postgres_seek_sees_items_beyond_map_stack_cut() -> TestResult {
    run_case(Case::Paging)
}

#[cfg(feature = "pg-tests-g1")]
#[test]
fn browse_field_postgres_distant_open_walks_and_retries_with_floor_and_path_refusals() -> TestResult {
    run_case(Case::Autowalk)
}

#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
fn run_case(case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("BROWSE_FIELD POSTGRES_CASE_SKIPPED: admin URL is absent");
        return Ok(());
    }
    // The real connection/composed owner has the same stack requirement as
    // the neighbouring item-loop and container-view dispatch cases.
    std::thread::Builder::new()
        .name(format!("browse-field-{case:?}"))
        .stack_size(256 << 20)
        .spawn(move || fixture::scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "browse-field PostgreSQL case panicked")??;
    eprintln!("BROWSE_FIELD POSTGRES_CASE_COMPLETED {case:?}");
    Ok(())
}
