//! W19d: ordinary USE, atomic source effects, exact replay, and fail-closed canonical admission.
use super::item_cosmetics_fixtures::{run_all, scenario};

#[path = "item_cosmetics/ot_only.rs"]
mod ot_only;

#[path = "item_cosmetics/positive.rs"]
mod positive;

use super::fixture::debug;
use super::TestResult;


#[test]
fn item_cosmetics_unavailable_never_spends_the_source() -> TestResult {
    run_case(false, false)
}

#[test]
fn pegasus_feather_use_consumes_one_and_grants_mount_without_premium() -> TestResult {
    run_all(
        false,
        false,
        Some(super::item_cosmetics_fixtures::positive::Case::Pegasus),
    )
}

#[test]
fn item_cosmetics_ground_unavailable_preserves_actual_ground_source() -> TestResult {
    run_case(true, false)
}

#[test]
fn item_cosmetics_ground_afflicted_unavailable_preserves_all_carried_pieces() -> TestResult {
    run_case(true, true)
}

fn run_case(ground: bool, afflicted: bool) -> TestResult {
    run_all(ground, afflicted, None)
}

#[test]
fn srx4_ot_cosmetic_items_refuse_with_complete_synthetic_catalogue_on_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!("SRX4_OT_EXCLUSION_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    for ground in [false, true] {
        for item in [6579, 37746] {
            let started = std::time::Instant::now();
            std::thread::Builder::new()
                .stack_size(256 << 20)
                .spawn(move || {
                    scenario(ground, false, None, false, None, Some(item))
                        .map_err(|e| e.to_string())
                })?
                .join()
                .map_err(|_| "OT exclusion thread")??;
            eprintln!(
                "SRX4_OT_EXCLUSION_POSTGRES_EXECUTED item={item} ground={ground} duration={:.3}s",
                started.elapsed().as_secs_f64()
            );
        }
    }
    Ok(())
}
