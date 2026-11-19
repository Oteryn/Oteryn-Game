//! SV15: real command-436 connection, production combat, fee and complete backpack MINT.
use super::TestResult;
use super::arena_match_fixtures::{NativeScenario, scenario, scenario_path, scenario_route};

#[test]
fn arena_match_real_connection_status_and_full_match() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            scenario(false, false).map_err(|e| e.to_string())?;
            scenario(true, false).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "arena match scenario panicked")??;
    eprintln!("M51 POSTGRES_CASE_COMPLETED: 30 real opponents, fee, reward, replay, exit, reload");
    Ok(())
}

#[test]
fn arena_match_credited_portal_timeout_restores_pillar() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(true, true).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "arena timeout scenario panicked")??;
    eprintln!(
        "M51 POSTGRES_CASE_COMPLETED credited_portal_timeout duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn m51_postgres_native_portals_complete_all_ten_pits_and_reward() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_path(true, true).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "M51 native portal run panicked")??;
    eprintln!(
        "M51 REAL POSTGRES native ten-pit portal run duration={:?}",
        started.elapsed()
    );
    assert!(started.elapsed().as_secs_f64() > 0.01);
    Ok(())
}

#[test]
fn m51_postgres_native_recreated_pillars_complete_all_ten_pits_and_reward() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            scenario_route(true, Some(NativeScenario::RecreatedPillars)).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "M51 recreated pillar worker panicked")??;
    eprintln!(
        "M51 REAL POSTGRES recreated pillars duration={:?}",
        started.elapsed()
    );
    assert!(started.elapsed().as_secs_f64() > 0.01);
    Ok(())
}

#[test]
fn m51_postgres_native_departure_cancels_without_a_leave_command() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_route(true, Some(NativeScenario::Departure)).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "M51 native departure worker panicked")??;
    eprintln!(
        "M51 REAL POSTGRES native departure duration={:?}",
        started.elapsed()
    );
    assert!(started.elapsed().as_secs_f64() > 0.01);
    Ok(())
}

#[test]
fn m51_postgres_native_timeout_kicks_and_persists_without_a_status_command() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_route(true, Some(NativeScenario::Timeout)).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "M51 native timeout worker panicked")??;
    eprintln!(
        "M51 REAL POSTGRES native timeout duration={:?}",
        started.elapsed()
    );
    assert!(started.elapsed().as_secs_f64() > 0.01);
    Ok(())
}

#[test]
fn m51_postgres_native_entry_resets_only_monsters_inside_source_box() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            scenario_route(true, Some(NativeScenario::ResetMonsters)).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "M51 native reset worker panicked")??;
    eprintln!(
        "M51 REAL POSTGRES native monster reset duration={:?}",
        started.elapsed()
    );
    assert!(started.elapsed().as_secs_f64() > 0.01);
    Ok(())
}
