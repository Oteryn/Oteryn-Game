//! M23 chosen personal trial: real native B3 + framed USE/STEP, canonical journal.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn m23_last_creep_eight_disguises_matching_portals_and_returns_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let begin = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("m23-last-creep", || {
        super::quest_world_callbacks_fixtures::scenario(
            "qengine-rewards-m23-last-creep",
            Position {
                x: 33261,
                y: 32365,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "M23_LAST_CREEP_PG_COMPLETE duration={:?} success={}",
        begin.elapsed(),
        result.is_ok()
    );
    if result.is_ok() {
        assert!(begin.elapsed() > std::time::Duration::from_millis(10));
    }
    result
}
