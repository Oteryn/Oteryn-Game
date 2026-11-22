use super::TestResult;
use super::annihilator_route_fixtures::{Route, run};

#[test]
fn demon_helmet_requires_both_exact_plates_and_displaces_before_restoring_walls() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run(Route::DemonHelmetPlates)
}

#[test]
fn annihilator_rejects_a_claimed_non_initiator_at_any_station() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run(Route::ClaimedStation)
}

#[test]
fn annihilator_deadline_transfers_only_run_members_still_inside_the_arena() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run(Route::ArenaDeadline)
}
