//! M38 basic Minzy contact through the live NPC command path; favour exchanges remain separate.
use super::npcfix_m38_minzy_fixtures::{EXERCISED};

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn npcfix_m38_minzy_basic_contact_and_stale_actor_are_fenced_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    EXERCISED.store(false, std::sync::atomic::Ordering::SeqCst);
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("npcfix-m38-minzy", || {
        super::quest_world_callbacks_fixtures::scenario(
            "npcfix-m38-minzy",
            Position {
                x: 32744,
                y: 31970,
                floor: -12,
            },
            false,
        )
    })?;
    assert!(
        EXERCISED.load(std::sync::atomic::Ordering::SeqCst),
        "Minzy scenario hook did not execute"
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED NPCFIX_M38_MINZY duration={:?}",
        started.elapsed()
    );
    Ok(())
}
