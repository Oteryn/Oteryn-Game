//! Oldrak's source NPC steps only; the defender encounter is not qualified here.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;


#[test]
fn questpc2_oldrak_source_confirmation_decline_and_finish_are_durable() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("QUESTPC2_OLDRAK_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("questpc2-oldrak-npc", || {
        super::quest_world_callbacks_fixtures::scenario(
            "questpc2-oldrak-npc",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC2_OLDRAK_POSTGRES_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
