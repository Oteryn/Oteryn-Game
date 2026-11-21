//! Source crate USE borrows existing carried quest and appearance owners without progress.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn srvgap1108_quest_item_disguise_fenced_use_restores_on_exact_exit_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("srvgap1108-disguise", || {
        super::quest_world_callbacks_fixtures::scenario(
            "qengine-rewards-srvgap-disguise",
            Position {
                x: 33322,
                y: 31080,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "SRVGAP1108_DISGUISE_PG_COMPLETE duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
