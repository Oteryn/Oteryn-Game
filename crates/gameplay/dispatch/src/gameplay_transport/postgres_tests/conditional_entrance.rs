//! Conditional entrances through ordinary STEP and literal USE over PostgreSQL.
use super::TestResult;
use super::conditional_entrance_fixtures::{Case, scenario};

#[test]
fn conditional_step_admits_inquisition_with_any_exact_throne_value() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Inquisition { pumin: false })?;
    scenario(Case::Inquisition { pumin: true })
}

#[test]
fn conditional_step_reads_native_megalomania_completion() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Completion)
}

#[test]
fn conditional_step_counts_each_native_miniboss_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Flags)
}

#[test]
fn conditional_step_reads_carried_item_presence_without_consuming_it() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Carried)
}

#[test]
fn conditional_step_reads_live_level_after_progression_changes() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Level)
}

#[test]
fn conditional_step_uses_spike_action_before_native_floor_routing() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Premium)
}

#[test]
fn conditional_step_refuses_until_other_player_leaves_the_grove() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Occupied)
}

#[test]
fn conditional_step_and_use_evaluate_the_annual_calendar() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    for (use_action, utc_seconds, allowed) in [
        (false, 1_775_001_599, false), // March 31 23:59:59 UTC
        (false, 1_775_001_600, true),  // April 1 00:00:00 UTC
        (true, 1_777_679_999, true),   // May 1 23:59:59 UTC
        (true, 1_777_680_000, false),  // May 2 00:00:00 UTC
    ] {
        scenario(Case::Calendar { use_action, utc_seconds, allowed })?;
    }
    Ok(())
}

#[test]
fn chyllfroest_summit_use_mints_only_in_season_after_the_cooldown() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::ChyllfroestReward)
}

#[test]
fn conditional_use_refuses_vine_while_pvp_locked() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::PvpLock)
}

#[test]
fn conditional_step_keeps_the_original_down_stair_selector() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Stair)
}

#[test]
fn conditional_use_checks_bounac_level_without_writing_quest_progress() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Bounac)
}

#[test]
fn conditional_use_moves_through_and_transforms_the_exact_door() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Door)
}

#[test]
fn conditional_step_accepts_either_native_isle_ship_exit_track() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Isle { travel: false })?;
    scenario(Case::Isle { travel: true })
}

#[test]
fn conditional_step_requires_ten_with_all_warzone_flags_and_fifteen_otherwise() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Warzone { access: false })?;
    scenario(Case::Warzone { access: true })
}
