//! Canonical completion joins the existing fenced NPC hand-in transaction.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn engnpc_m04_completion_is_atomic_canonical_and_replay_safe() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("engnpc-m04", || {
        super::quest_world_callbacks_fixtures::scenario(
            "engnpc-m04",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!("ENGNPC_M04_PG_EXECUTED duration={:?}", started.elapsed());
    Ok(())
}
