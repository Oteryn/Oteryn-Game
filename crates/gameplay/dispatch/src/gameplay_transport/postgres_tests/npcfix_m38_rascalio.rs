//! M38 basic Rascalio contact through the live NPC command path; event ferrying remain separate.
use super::npcfix_m38_rascalio_fixtures::{EXERCISED};

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn npcfix_m38_rascalio_basic_contact_and_stale_actor_are_fenced_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    EXERCISED.store(false, std::sync::atomic::Ordering::SeqCst);
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("npcfix-m38-rascalio", || {
        super::quest_world_callbacks_fixtures::scenario(
            "npcfix-m38-rascalio",
            Position {
                x: 32260,
                y: 31642,
                floor: -7,
            },
            false,
        )
    })?;
    assert!(
        EXERCISED.load(std::sync::atomic::Ordering::SeqCst),
        "Rascalio scenario hook did not execute"
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED NPCFIX_M38_RASCALIO duration={:?}",
        started.elapsed()
    );
    Ok(())
}
