//! Production NPC command, atomic acceptance, bound replay and session fence qualification.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn catalogue_npc_source_acceptance_is_atomic_and_replay_bound() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("NPC_PROGRAM_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("catalogue-npc", || {
        super::quest_world_callbacks_fixtures::scenario(
            "catalogue-npc",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED catalogue_npc_source_acceptance_is_atomic_and_replay_bound {:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn catalogue_npc_delayed_end_runs_in_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("catalogue-npc-end", || {
        super::quest_world_callbacks_fixtures::scenario(
            "catalogue-npc-end",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED catalogue_npc_delayed_end_runs_in_real_dispatch {:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn catalogue_npc_calendar_month_runs_in_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("catalogue-npc-calendar", || {
        super::quest_world_callbacks_fixtures::scenario(
            "catalogue-npc-calendar",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED catalogue_npc_calendar_month_runs_in_real_dispatch {:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn catalogue_npc_general_xp_handin_runs_in_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("catalogue-npc-xp", || {
        super::quest_world_callbacks_fixtures::scenario(
            "catalogue-npc-xp",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED catalogue_npc_general_xp_handin_runs_in_real_dispatch {:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn catalogue_npc_father_fresh_acceptance_is_one_eleven_track_transaction() -> TestResult {
    father_case(false)
}

#[test]
fn catalogue_npc_father_legacy_hash_preserves_defaults_and_restart_receipts() -> TestResult {
    father_case(true)
}

fn father_case(legacy: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let name = if legacy {
        "sidec-catalogue-npc-legacy"
    } else {
        "sidec-catalogue-npc-father"
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        super::quest_world_callbacks_fixtures::scenario(
            name,
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!("POSTGRES_CASE_COMPLETED {name} {:?}", started.elapsed());
    Ok(())
}

#[test]
fn catalogue_npc_scoped_a_m_starts_are_source_ordered_and_atomic() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("sidec-catalogue-npc-scoped", || {
        super::quest_world_callbacks_fixtures::scenario(
            "sidec-catalogue-npc-scoped",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED catalogue_npc_scoped_a_m_starts_are_source_ordered_and_atomic {:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn hero_of_rathleton_pipe_greeting_commits_access_once_and_replays() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("sidec-catalogue-npc-hero", || {
        super::quest_world_callbacks_fixtures::scenario(
            "sidec-catalogue-npc-hero",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED hero_of_rathleton_pipe_greeting_commits_access_once_and_replays {:?}",
        started.elapsed()
    );
    Ok(())
}
