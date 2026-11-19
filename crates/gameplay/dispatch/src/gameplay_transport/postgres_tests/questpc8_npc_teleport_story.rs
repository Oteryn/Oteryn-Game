//! M37: a real source NPC command reaches held placement and arrival consumption.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn m37_peter_npc_teleport_postgres_dispatches_and_replays_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("M37_DISPATCH_POSTGRES_NOT_EXECUTED no_admin_url");
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("qengine-rewards-m37-peter", || {
        super::quest_world_callbacks_fixtures::scenario(
            "qengine-rewards-m37-peter",
            Position {
                x: 32860,
                y: 31303,
                floor: -7,
            },
            false,
        )
    })?;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!(
        "M37_POSTGRES_NPC_DISPATCH_PLACED_AND_REPLAYED duration={:?}",
        started.elapsed()
    );
    Ok(())
}
