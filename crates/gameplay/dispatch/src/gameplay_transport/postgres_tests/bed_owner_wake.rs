use super::TestResult;
use super::bed_owner_wake_fixtures::Case;
use super::hireling_lifecycle_fixtures::{Case as LifecycleCase, run_case_with_admin};

#[test]
fn owner_use_wakes_both_bed_halves_once_postgres() -> TestResult {
    run(Case::Replay)
}

fn run(case: Case) -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    let result = run_case_with_admin(LifecycleCase::BedOwnerWake(case), admin);
    eprintln!(
        "PG_MARKER S07 bed-owner-wake {case:?} success={} duration={:.3}s",
        result.is_ok(),
        started.elapsed().as_secs_f64()
    );
    result
}

#[test]
fn revoked_owner_cannot_wake_while_house_transport_is_active_postgres() -> TestResult {
    run(Case::Revoked)
}

#[test]
fn owner_wakes_without_server_modal_capability_postgres() -> TestResult {
    run(Case::WorldOnly)
}

#[test]
fn nonpremium_bed_owner_receives_sealed_with_premium_message_postgres() -> TestResult {
    run(Case::NonPremium)
}

#[test]
fn bed_owner_use_obeys_world_action_cooldown_postgres() -> TestResult {
    run(Case::Cooldown)
}

#[test]
fn bed_owner_retries_temporary_holder_contention_postgres() -> TestResult {
    run(Case::Contended)
}

#[test]
fn bed_owner_resume_reconciles_committed_unacknowledged_wake_postgres() -> TestResult {
    run(Case::Resume)
}

#[test]
fn bed_owner_resume_terminalizes_database_rollback_postgres() -> TestResult {
    run(Case::Rollback)
}

#[test]
fn bed_owner_use_fences_intervening_placement_commit_postgres() -> TestResult {
    run(Case::PlacementRace)
}

#[test]
fn bed_owner_use_preserves_stale_static_revision_postgres() -> TestResult {
    run(Case::StaleTarget)
}

#[test]
fn bed_owner_use_preserves_map_handle_and_static_reach_refusals_postgres() -> TestResult {
    run(Case::TooFar)
}

#[test]
fn bed_owner_use_resyncs_static_bundle_mismatch_postgres() -> TestResult {
    run(Case::BundleMismatch)
}

#[test]
fn bed_reservation_releases_after_native_house_step_postgres() -> TestResult {
    run(Case::ReservationMove)
}
