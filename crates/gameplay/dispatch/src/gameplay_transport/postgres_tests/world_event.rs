//! Real reset finalization -> admitted command bytes -> receipt -> result -> restart.
use super::world_event_fixtures::{FlowerMonthCheck, scenario_run, scenario_with_flower_month};

use super::*;

#[test]
fn flower_month_world_event_query_reports_committed_june_only() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario_with_flower_month(FlowerMonthCheck::Announcement)
}

#[test]
fn flower_month_world_event_query_replays_original_epoch_response() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario_with_flower_month(FlowerMonthCheck::Replay)
}

#[test]
fn world_event_postgres_real_dispatch_reset_restart_and_fences() -> TestResult {
    let started = std::time::Instant::now();
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            for epochs in [1, 2] {
                scenario(epochs).map_err(|e| e.to_string())?;
            }
            Ok::<_, String>(())
        })?
        .join()
        .map_err(|_| "world event e2e panicked")??;
    eprintln!(
        "T94_DISPATCH_PG_EXECUTED reset/dispatch/restart elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

#[test]
fn w05d_halloween_calendar_reaches_real_reset_query_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_run(2, Some("Halloween"), false, None).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "seasonal reset query thread panicked")??;
    Ok(())
}

#[test]
fn w05d_christmas_real_npc_present_replay_and_permanent_claim() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_run(2, Some("Christmas"), false, None).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Christmas native NPC thread panicked")??;
    eprintln!(
        "W05D_CHRISTMAS_PG_EXECUTED elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

#[test]
fn w05d_cake_real_baker_burns_only_remaining_goal_and_replays() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_run(2, Some("A Piece of Cake"), false, None).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Cake native NPC thread panicked")??;
    eprintln!(
        "W05D_CAKE_BAKER_PG_EXECUTED elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

pub(super) fn scenario(epochs: u64) -> TestResult {
    scenario_run(epochs, None, false, None)
}
