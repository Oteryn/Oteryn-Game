//! PostgreSQL dispatch coverage for once-only and recurring quest claims.

use super::TestResult;
use super::qengine_rewards_fixtures::scenario;
use super::quest_claims_fixtures::*;

#[test]
fn once_only_well_restores_saved_and_live_vitals_and_grants_achievement_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(WELL)
}

#[test]
fn well_achievement_write_failure_rolls_back_the_heal_and_claim_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(WELL_ROLLBACK)
}

#[test]
fn well_claim_is_independent_for_two_players_on_the_same_source_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(WELL_SHARED)
}

#[test]
fn daily_claim_serializes_double_use_and_refuses_until_period_expires_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(DAILY)
}

#[test]
fn fountain_heart_is_eligible_until_the_noble_lion_is_tamed_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(DAILY_HEART)
}

#[test]
fn annual_claim_delivers_backpack_and_refuses_until_period_expires_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(ANNUAL)
}

#[test]
fn fountain_replay_projects_the_rock_and_refuses_it_after_cooldown_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(FOUNTAIN_TRANSFORM)
}

#[test]
fn paid_lions_sockets_open_the_shared_fountain_and_expire_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(SOCKET_WINDOW)
}

#[test]
fn lions_socket_window_rearms_after_the_daily_claim_clock_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(SOCKET_REPEAT)
}

#[test]
fn lions_socket_window_opens_on_elapsed_overlap_when_scheduler_catches_up_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(SOCKET_CATCHUP)
}

#[test]
fn channel_owner_catches_up_paid_socket_opening_after_fields_expire_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(SOCKET_OWNER_CATCHUP)
}

#[test]
fn full_backpack_rolls_back_bundle_and_cooldown_until_claim_retry_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(FULL_BACKPACK)
}

#[test]
fn hoard_bundle_resets_eligibility_and_awards_tenth_claim_achievement_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(HOARD)
}

#[test]
fn hoard_gem_bundle_commits_all_three_gems_with_the_claim_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(HOARD_GEMS)
}

#[test]
fn interrupted_well_use_recovers_committed_heal_without_second_credit_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(WELL_LOST_ACK)
}

#[test]
fn interrupted_socket_use_keeps_the_paid_field_deadlines_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(SOCKET_RECOVERY)
}

#[test]
fn step_claim_credits_the_same_achievement_only_once_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(STEP)
}
